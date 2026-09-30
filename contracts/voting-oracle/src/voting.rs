use predictx_shared::constants::{ADMIN_REVIEW_THRESHOLD_BPS, AUTO_RESOLVE_THRESHOLD_BPS, BPS_DENOMINATOR};
use predictx_shared::PollStatus;
use soroban_sdk::{Env, symbol_short};
use crate::{DataKey, StoredPollStatus};

pub fn determine_consensus_status(yes_votes: u32, no_votes: u32) -> PollStatus {
    let total = yes_votes + no_votes;
    if total == 0 {
        return PollStatus::AdminReview;
    }

    let max_votes = if yes_votes > no_votes { yes_votes } else { no_votes };
    let consensus_bps = (max_votes as u64 * BPS_DENOMINATOR as u64) / (total as u64);

    if consensus_bps >= AUTO_RESOLVE_THRESHOLD_BPS as u64 {
        PollStatus::Resolved
    } else if consensus_bps >= ADMIN_REVIEW_THRESHOLD_BPS as u64 {
        PollStatus::AdminReview
    } else {
        PollStatus::Disputed
    }
}

pub fn update_poll_status(env: &Env, poll_id: u64, yes_votes: u32, no_votes: u32) -> PollStatus {
    let status = determine_consensus_status(yes_votes, no_votes);
    let provisional_outcome = if yes_votes > no_votes {
        Some(true)
    } else if no_votes > yes_votes {
        Some(false)
    } else {
        None
    };
    
    let stored = StoredPollStatus {
        status,
        updated_at: env.ledger().timestamp(),
        provisional_outcome,
    };

    env.storage()
        .persistent()
        .set(&DataKey::PollStatus(poll_id), &stored);

    // emit status event
    let topics = (symbol_short!("status"), poll_id);
    env.events().publish(topics, status as u32);
    
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_review_boundaries() {
        // EXACTLY 60% parks in AdminReview
        // 60 votes vs 40 votes => 60% max => AdminReview
        assert_eq!(determine_consensus_status(60, 40), PollStatus::AdminReview);

        // EXACTLY 85% auto-resolves
        // 85 votes vs 15 votes => 85% max => Resolved
        assert_eq!(determine_consensus_status(85, 15), PollStatus::Resolved);

        // 70% consensus parks in AdminReview
        // 70 votes vs 30 votes => 70% max => AdminReview
        assert_eq!(determine_consensus_status(70, 30), PollStatus::AdminReview);

        // 59% disputes
        // 59 votes vs 41 votes => 59% max => Disputed
        assert_eq!(determine_consensus_status(59, 41), PollStatus::Disputed);
//! Voting lifecycle functions for the VotingOracle contract.

use predictx_shared::{PollStatus, PredictXError, VoteTally, VOTING_WINDOW_SECS};
use soroban_sdk::{Address, Env, String};

use crate::{DataKey, StoredPollStatus};

/// Opens a two-hour community voting window for a finished poll.
///
/// # Arguments
/// * `env`           — Soroban environment
/// * `admin`         — The admin address; must match the stored admin
/// * `poll_id`       — ID of the poll whose match has concluded
/// * `evidence_hash` — IPFS/hash of evidence supporting the outcome
///
/// # Errors
/// * [`PredictXError::Unauthorized`]        — caller is not the stored admin
/// * [`PredictXError::PollAlreadyResolved`] — voting has already been opened
///                                            for this poll (status is already
///                                            `PollStatus::Voting`)
///
/// # Storage written
/// * `DataKey::VoteTally(poll_id)`      — zeroed tally with
///   `voting_end_time = now + VOTING_WINDOW_SECS`, written to **temporary**
///   storage (only needed for the duration of the voting window)
/// * `DataKey::VotingEvidence(poll_id)` — the IPFS evidence hash, written to
///   **temporary** storage alongside the tally
/// * `DataKey::PollStatus(poll_id)`     — set to `PollStatus::Voting` via the
///   same `StoredPollStatus` pattern used by `set_poll_status`
pub fn initiate_voting(
    env: Env,
    admin: Address,
    poll_id: u64,
    evidence_hash: String,
) -> Result<(), PredictXError> {
    // 1. Require admin authentication.
    admin.require_auth();

    // 2. Verify the caller is the stored admin.
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(PredictXError::NotInitialized)?;

    if admin != stored_admin {
        return Err(PredictXError::Unauthorized);
    }

    // 3. Guard against double-opening: if the poll is already in the Voting
    //    state we treat it the same as "already resolved" — the window is open.
    let current_status: PollStatus = env
        .storage()
        .persistent()
        .get::<DataKey, StoredPollStatus>(&DataKey::PollStatus(poll_id))
        .map(|s| s.status)
        .unwrap_or(PollStatus::Active);

    if current_status == PollStatus::Voting {
        return Err(PredictXError::PollAlreadyResolved);
    }

    // 4. Compute the voting deadline (checked arithmetic — no silent overflow).
    let now: u64 = env.ledger().timestamp();
    let voting_end_time: u64 = now
        .checked_add(VOTING_WINDOW_SECS)
        .expect("timestamp overflow");

    // 5. Build a zeroed VoteTally with every field from the actual struct.
    //    evidence_hash is not a field of VoteTally; it is stored separately
    //    (see step 6b).
    let tally = VoteTally {
use crate::{storage, DataKey, MAX_VOTERS};
use predictx_shared::{
    PollStatus, PredictXError, VoteChoice, VoteTally, AUTO_RESOLVE_THRESHOLD_BPS, BPS_DENOMINATOR,
    VOTER_REWARD_BPS, VOTING_WINDOW_SECS,
};
use soroban_sdk::{Address, Env, IntoVal, Symbol};

fn has_user_staked(env: &Env, poll_id: u64, voter: &Address) -> Result<bool, PredictXError> {
    let prediction_market: Address = env
        .storage()
        .instance()
        .get(&DataKey::PredictionMarket)
        .ok_or(PredictXError::NotInitialized)?;

    Ok(env.invoke_contract(
        &prediction_market,
        &Symbol::new(env, "has_user_staked"),
        (poll_id, voter.clone()).into_val(env),
    ))
}

/// Record a voter's choice on a poll.
///
/// # Voter cap policy
///
/// The roster of distinct voters per poll is bounded by [`MAX_VOTERS`]. The
/// cap is **window-scoped**, not a permanent freeze: it only limits how many
/// distinct addresses may be recorded during the poll's voting window, and it
/// never blocks the poll from reaching a settlement path.
///
/// When the cap is reached:
///
/// - New distinct addresses are rejected with [`PredictXError::MaxVotersReached`]
///   (see [`cast_vote`]); the roster is *not* silently truncated or rotated,
///   so already-recorded votes stay intact and auditable.
/// - The poll remains resolvable. [`auto_resolve`] only depends on the tally
///   and the voting window, so a capped poll still settles once the window
///   closes and the leading outcome clears
///   [`AUTO_RESOLVE_THRESHOLD_BPS`]. A capped poll is therefore never left
///   permanently unsettleable by community vote.
/// - Recovery for a poll that cannot reach consensus is handled by the
///   admin/community resolution path (tracked separately), which does not
///   require reopening the roster.
///
/// # Abuse model for exhausting the roster
///
/// Because [`cast_vote`] does not yet gate on stake or eligibility (see the
/// "excluding stakers" note below), an adversary can fill all [`MAX_VOTERS`]
/// slots with sybil addresses and lock out honest voters for the remainder of
/// the window. The cap bounds the blast radius of that griefing: it limits the
/// roster to a fixed size, keeps every recorded vote auditable, and — crucially
/// — does not prevent the poll from settling. Mitigating the sybil fill itself
/// (stake-weighting, eligibility proofs, or a per-window reset) is out of scope
/// for this change and tracked separately.
///
/// Flow (Checks → Effects):
/// 1. Authenticates the caller as the voter.
/// 2. Verifies the poll is known to the oracle, else `PollNotFound`.
/// 3. Rejects a duplicate vote from the same voter, else `AlreadyVoted`.
/// 4. Loads the existing tally (or seeds a fresh one) and increments the
///    chosen outcome's counter plus the total voter count.
/// 5. Persists the updated tally and the per-voter dedup marker, and returns
///    the tally.
///
    env: &Env,
    voter: Address,
    poll_id: u64,
    choice: VoteChoice,
) -> Result<VoteTally, PredictXError> {
    voter.require_auth();

    // ── Checks ────────────────────────────────────────────────────────────────

    // Only accept votes on polls the oracle already knows about.
    if !env
        .storage()
        .persistent()
        .has(&DataKey::PollStatus(poll_id))
    {
        return Err(PredictXError::PollNotFound);
    }

    if crate::read_poll_status(env, poll_id) != PollStatus::Voting {
        return Err(PredictXError::VotingNotOpen);
    }

    // Each address may vote at most once per poll.
    let mut voters = storage::read_voters(env, poll_id);
    if storage::has_voted(env, poll_id, &voter) || voters.contains(voter.clone()) {
        return Err(PredictXError::AlreadyVoted);
    }

    if has_user_staked(env, poll_id, &voter)? {
        return Err(PredictXError::VoterIsStaker);
    // Cap is window-scoped: once the roster is full, new distinct addresses
    // are rejected for this window only. The poll still settles via
    // `auto_resolve` (or the admin/community path), so this is not a
    // permanent freeze. See the cap policy in `cast_vote`'s doc comment.
    if voters.len() >= MAX_VOTERS {
        return Err(PredictXError::MaxVotersReached);
    }

    // ── Effects ───────────────────────────────────────────────────────────────

    // Load-or-create the tally, then record this vote.
    let mut tally = storage::read_tally(env, poll_id).unwrap_or(VoteTally {
        poll_id,
        yes_votes: 0,
        no_votes: 0,
        unclear_votes: 0,
        total_voters: 0,
        voting_end_time,
        reward_pool: 0,
    };

    // 6a. Persist the tally in temporary storage — it is only needed for the
    //     duration of the two-hour voting window.
    env.storage()
        .temporary()
        .set(&DataKey::VoteTally(poll_id), &tally);

    // 6b. Store the evidence hash alongside the tally so voters and resolvers
    //     can retrieve it.  VoteTally itself has no evidence_hash field.
    env.storage()
        .temporary()
        .set(&DataKey::VotingEvidence(poll_id), &evidence_hash);

    // 7. Advance poll status to Voting using the same StoredPollStatus pattern
    //    that set_poll_status (lib.rs:50) uses.
    let stored_status = StoredPollStatus {
        status: PollStatus::Voting,
        voting_end_time: crate::read_poll_status_updated_at(env, poll_id)
            .checked_add(VOTING_WINDOW_SECS)
            .unwrap_or(0),
        reward_pool: 0,
    });

    match choice {
        VoteChoice::Yes => tally.yes_votes += 1,
        VoteChoice::No => tally.no_votes += 1,
        VoteChoice::Unclear => tally.unclear_votes += 1,
    }
    tally.total_voters += 1;

    storage::write_tally(env, &tally);
    voters.push_back(voter.clone());
    storage::write_voters(env, poll_id, &voters);
    storage::write_voted(env, poll_id, &voter);
    // Persist the choice itself (not just the count) so reward eligibility can
    // be checked against the resolved outcome later.
    storage::write_vote_choice(env, poll_id, &voter, choice);
    Ok(tally)
}

/// Set (fund) the voter reward reserve for `poll_id`.
///
/// Admin-only. The reserve *size* policy is out of scope; this just records the
/// amount that eligible voters will share.
pub fn set_reward_pool(
    env: &Env,
    caller: Address,
    poll_id: u64,
    amount: i128,
) -> Result<(), PredictXError> {
    storage::require_admin(env, &caller)?;
    caller.require_auth();

    if amount < 0 {
        return Err(PredictXError::InvalidRewardAmount);
    }
    if !env
        .storage()
        .persistent()
        .has(&DataKey::PollStatus(poll_id))
    {
        return Err(PredictXError::PollNotFound);
    }

    storage::write_reward_pool(env, poll_id, amount);
    Ok(())
}

/// Claim the caller's voter reward for a resolved poll.
///
/// Only voters who backed the winning outcome are eligible: a voter on the
/// losing side, an `Unclear` voter, or a non-voter is rejected with
/// [`PredictXError::VoterNotEligible`]. The reward reserve is split evenly
/// across the *eligible* voters (never the total voter count), and each voter
/// may claim at most once.
pub fn claim_reward(env: &Env, voter: Address, poll_id: u64) -> Result<i128, PredictXError> {
    voter.require_auth();

    if !env
        .storage()
        .persistent()
        .has(&DataKey::PollStatus(poll_id))
    {
        return Err(PredictXError::PollNotFound);
    }

    // Rewards only open once the poll has a resolved, decisive outcome.
    let outcome: VoteChoice = env
        .storage()
        .persistent()
        .get(&DataKey::PollOutcome(poll_id))
        .ok_or(PredictXError::OutcomeNotAvailable)?;
    if outcome == VoteChoice::Unclear {
        return Err(PredictXError::OutcomeNotAvailable);
    }

    if storage::has_claimed_reward(env, poll_id, &voter) {
        return Err(PredictXError::AlreadyClaimed);
    }

    // The claimant must have voted, and must have backed the winning side.
    let choice =
        storage::read_vote_choice(env, poll_id, &voter).ok_or(PredictXError::VoterNotEligible)?;
    if choice != outcome {
        return Err(PredictXError::VoterNotEligible);
    }

    let eligible = eligible_voter_count(env, poll_id, outcome);
    if eligible == 0 {
        return Err(PredictXError::VoterNotEligible);
    }

    let pool = storage::read_reward_pool(env, poll_id);
    let share = if pool > 0 {
        pool / i128::from(eligible)
    } else {
        0
    };

    storage::write_reward_claimed(env, poll_id, &voter);
    env.storage()
        .persistent()
        .set(&DataKey::VoterReward(poll_id, voter.clone()), &share);

    env.events()
        .publish((Symbol::new(env, "RewardClaimed"), poll_id, voter), share);

    Ok(share)
}

/// Count the voters on `poll_id` whose recorded choice equals `outcome`.
///
/// Bounded by `MAX_VOTERS`, so a full roster scan is cheap.
fn eligible_voter_count(env: &Env, poll_id: u64, outcome: VoteChoice) -> u32 {
    let voters = storage::read_voters(env, poll_id);
    let mut count = 0_u32;
    for i in 0..voters.len() {
        let voter = voters.get(i).unwrap();
        if storage::read_vote_choice(env, poll_id, &voter) == Some(outcome) {
            count += 1;
        }
    }
    count
}

/// Resolve a voting poll when the winning outcome reaches the automatic
/// resolution threshold after the voting window closes.
///
/// `total_pool` is the staking pool the caller (eventually the
/// `PredictionMarket` contract) settles against; `VOTER_REWARD_BPS` of it is
/// reserved for eligible voters and written to the tally's `reward_pool`.
/// The reserve is computed once at resolution: once the poll status moves to
/// `Resolved`, every later resolution attempt is rejected, so it is never
/// recomputed.
pub fn auto_resolve(
    env: &Env,
    poll_id: u64,
    total_pool: i128,
) -> Result<VoteChoice, PredictXError> {
    if !env
        .storage()
        .persistent()
        .has(&DataKey::PollStatus(poll_id))
    {
        return Err(PredictXError::PollNotFound);
    }

    if crate::read_poll_status(env, poll_id) != PollStatus::Voting {
        return Err(PredictXError::VotingNotOpen);
    }

    let mut tally = storage::read_tally(env, poll_id).ok_or(PredictXError::PollNotFound)?;
    if env.ledger().timestamp() < tally.voting_end_time {
        return Err(PredictXError::VotingNotOpen);
    }

    let (outcome, winning_votes) =
        if tally.yes_votes >= tally.no_votes && tally.yes_votes >= tally.unclear_votes {
            (VoteChoice::Yes, tally.yes_votes)
        } else if tally.no_votes >= tally.unclear_votes {
            (VoteChoice::No, tally.no_votes)
        } else {
            (VoteChoice::Unclear, tally.unclear_votes)
        };

    if tally.total_voters == 0 {
        return Err(PredictXError::ConsensusNotReached);
    }

    let consensus_bps = (u64::from(winning_votes) * u64::from(BPS_DENOMINATOR)
        / u64::from(tally.total_voters)) as u32;
    if consensus_bps < AUTO_RESOLVE_THRESHOLD_BPS {
        return Err(PredictXError::ConsensusNotReached);
    }

    // ── Effects ───────────────────────────────────────────────────────────────

    // Reserve the voters' share of the pool once, at resolution time. Reaching
    // this point guarantees the poll settles exactly once: every later attempt
    // is rejected by the `PollStatus` guard above, so the reserve is never
    // recomputed.
    tally.reward_pool = voter_reward_reserve(total_pool);
    storage::write_tally(env, &tally);

    let now = env.ledger().timestamp();
    let stored_status = crate::StoredPollStatus {
        status: PollStatus::Resolved,
        updated_at: now,
    };
    env.storage()
        .persistent()
        .set(&DataKey::PollStatus(poll_id), &stored_status);

    Ok(())
    env.storage()
        .persistent()
        .set(&DataKey::PollOutcome(poll_id), &outcome);

    env.events().publish(
        (Symbol::new(env, "AutoResolved"), poll_id, outcome),
        consensus_bps,
    );

    Ok(outcome)
}

/// Share of the total pool reserved for eligible voters at resolution time.
///
/// Computes `total_pool * VOTER_REWARD_BPS / 10_000`, rounded down. Pure so
/// it can be unit-tested directly: non-positive pools reserve nothing, and
/// the default `VOTER_REWARD_BPS` of 100 yields exactly 1% of the pool.
pub(crate) fn voter_reward_reserve(total_pool: i128) -> i128 {
    if total_pool <= 0 {
        return 0;
    }
    total_pool * i128::from(VOTER_REWARD_BPS) / i128::from(BPS_DENOMINATOR)
}

/// Share of the decisive (Yes/No) votes held by the leading outcome.
///
/// Returns `(leading_is_yes, share_bps)`, where `share_bps` is rounded down
/// to whole basis points out of [`BPS_DENOMINATOR`].
///
/// - `Unclear` votes are excluded from the denominator: they signal "cannot
///   judge", not a preference.
/// - A Yes/No tie resolves to Yes (`true`) at 5000 bps, so the result is
///   deterministic.
/// - A tally with no decisive votes (e.g. all `Unclear`) returns `(false, 0)`
///   instead of dividing by zero.
///
/// Pure and side-effect free so the routing thresholds can be unit-tested
/// against it directly.
#[allow(dead_code)] // consumed by the upcoming threshold-routing issues
pub(crate) fn consensus_bps(tally: &VoteTally) -> (bool, u32) {
    let decisive = u64::from(tally.yes_votes) + u64::from(tally.no_votes);
    if decisive == 0 {
        return (false, 0);
    }

    let leading_is_yes = tally.yes_votes >= tally.no_votes;
    let leading_votes = if leading_is_yes {
        tally.yes_votes
    } else {
        tally.no_votes
    };

    let share_bps = (u64::from(leading_votes) * u64::from(BPS_DENOMINATOR) / decisive) as u32;
    (leading_is_yes, share_bps)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    extern crate std;

    use predictx_shared::{
        PollStatus, PredictXError, VoteChoice, VoteTally, VOTER_REWARD_BPS, VOTING_WINDOW_SECS,
    };
    use soroban_sdk::{
        contract, contractimpl, contracttype,
        testutils::{Address as _, Ledger},
        Address, Env,
    };

    use crate::{storage, VotingOracle, VotingOracleClient};
    #[contract]
    struct TestPredictionMarket;

    #[contracttype]
    enum TestDataKey {
        Staked(Address),
    }

    #[contractimpl]
    impl TestPredictionMarket {
        pub fn set_staked(env: Env, user: Address, staked: bool) {
            env.storage()
                .instance()
                .set(&TestDataKey::Staked(user), &staked);
        }

        pub fn has_user_staked(env: Env, _poll_id: u64, user: Address) -> bool {
            env.storage()
                .instance()
                .get(&TestDataKey::Staked(user))
                .unwrap_or(false)
        }
    }

    use crate::{VotingOracle, VotingOracleClient};

    fn setup() -> (Env, Address, VotingOracleClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let cid = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &cid);
        let admin = Address::generate(&env);

        client.initialize(&admin);
        let prediction_market_id = env.register(TestPredictionMarket, ());
        client.set_prediction_market(&prediction_market_id);
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);

        // Register poll 1 as a known poll. `initiate_voting` (#80) will later
        // be the real production path for this transition.
        client.set_poll_status(&1_u64, &PollStatus::Voting);

        (env, admin, client)
    }

    fn voter(env: &Env) -> Address {
        Address::generate(env)
    }

    /// Read a poll's tally from the contract's own storage (tests run outside
    /// the contract, so direct storage access must be wrapped).
    fn stored_tally(env: &Env, client: &VotingOracleClient, poll_id: u64) -> Option<VoteTally> {
        env.as_contract(&client.address, || storage::read_tally(env, poll_id))
    }

    #[test]
    fn cast_vote_records_choice_and_counts_voters() {
        let (env, _admin, client) = setup();

        let tally = client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);

        assert_eq!(tally.poll_id, 1);
        assert_eq!(tally.yes_votes, 1);
        assert_eq!(tally.no_votes, 0);
        assert_eq!(tally.unclear_votes, 0);
        assert_eq!(tally.total_voters, 1);
    }

    #[test]
    fn cast_vote_rejects_staker() {
        let (env, _admin, client) = setup();
        let prediction_market_id = env.register(TestPredictionMarket, ());
        client.set_prediction_market(&prediction_market_id);
        let prediction_market =
            TestPredictionMarketClient::new(&env, &prediction_market_id);
        let staker = voter(&env);
        prediction_market.set_staked(&staker, &true);

        let err = client
            .try_cast_vote(&staker, &1_u64, &VoteChoice::Yes)
            .expect_err("stakers must not vote on their own poll");

        assert_eq!(err, Ok(PredictXError::VoterIsStaker));
    }

    #[test]
    fn cast_vote_allows_configured_prediction_market_non_staker() {
        let (env, _admin, client) = setup();
        let non_staker = voter(&env);

        let tally = client.cast_vote(&non_staker, &1_u64, &VoteChoice::Yes);

        assert_eq!(tally.total_voters, 1);
        assert_eq!(tally.yes_votes, 1);
    }

    #[test]
    fn capped_poll_still_reaches_settlement_path() {
        let (env, _admin, client) = setup();

        // Fill the roster to the cap with a decisive Yes majority.
        for _ in 0..MAX_VOTERS {
            client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);
        }

        // The cap is hit: a further distinct voter is rejected...
        let err = client
            .try_cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes)
            .expect_err("voter roster cap must be enforced");
        assert_eq!(err, Ok(PredictXError::MaxVotersReached));

        // ...but the capped poll is not frozen: it still settles once the
        // voting window closes and consensus clears the threshold.
        env.ledger().set_timestamp(1_000_000 + VOTING_WINDOW_SECS);
        let outcome = client.auto_resolve(&1_u64);

        assert_eq!(outcome, VoteChoice::Yes);
        assert_eq!(client.get_poll_status(&1_u64), PollStatus::Resolved);
        assert_eq!(client.get_poll_outcome(&1_u64), VoteChoice::Yes);
    }

    #[test]
    fn cast_vote_updates_persisted_tally() {
        let (env, _admin, client) = setup();

        client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);

        let tally = client.cast_vote(&voter(&env), &1_u64, &VoteChoice::No);

        assert_eq!(tally.yes_votes, 1);
        assert_eq!(tally.no_votes, 1);
        assert_eq!(tally.total_voters, 2);
    }

    #[test]
    fn cast_vote_accumulates_all_three_choices() {
        let (env, _admin, client) = setup();

        client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);
        client.cast_vote(&voter(&env), &1_u64, &VoteChoice::No);
        let tally = client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Unclear);

        assert_eq!(tally.yes_votes, 1);
        assert_eq!(tally.no_votes, 1);
        assert_eq!(tally.unclear_votes, 1);
        assert_eq!(tally.total_voters, 3);
    }

    #[test]
    fn cast_vote_rejects_unknown_poll() {
        let (env, _admin, client) = setup();

        let err = client
            .try_cast_vote(&voter(&env), &999_u64, &VoteChoice::Yes)
            .expect_err("unknown poll must be rejected");

        assert_eq!(err, Ok(PredictXError::PollNotFound));
    }

    #[test]
    fn cast_vote_rejects_active_poll() {
        let (env, _admin, client) = setup();
        client.set_poll_status(&1_u64, &PollStatus::Active);

        let err = client
            .try_cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes)
            .expect_err("active poll must reject voting");

        assert_eq!(err, Ok(PredictXError::VotingNotOpen));
    }

    #[test]
    fn cast_vote_rejects_resolved_poll() {
        let (env, _admin, client) = setup();
        client.set_poll_status(&1_u64, &PollStatus::Resolved);

        let err = client
            .try_cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes)
            .expect_err("resolved poll must reject voting");

        assert_eq!(err, Ok(PredictXError::VotingNotOpen));
    }

    #[test]
    fn cast_vote_rejects_duplicate_vote_from_same_voter() {
        let (env, _admin, client) = setup();
        let v = voter(&env);

        client.cast_vote(&v, &1_u64, &VoteChoice::Yes);

        let err = client
            .try_cast_vote(&v, &1_u64, &VoteChoice::No)
            .expect_err("a second vote from the same voter must be rejected");

        assert_eq!(err, Ok(PredictXError::AlreadyVoted));
    }

    #[test]
    fn rejected_duplicate_vote_leaves_tally_unchanged() {
        let (env, _admin, client) = setup();
        let v = voter(&env);

        client.cast_vote(&v, &1_u64, &VoteChoice::Yes);
        let rejected = client
            .try_cast_vote(&v, &1_u64, &VoteChoice::No)
            .expect_err("second vote must be rejected");
        assert_eq!(rejected, Ok(PredictXError::AlreadyVoted));

        // A fresh voter's tally proves the rejected vote added nothing.
        let tally = client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Unclear);

        assert_eq!(tally.yes_votes, 1);
        assert_eq!(tally.no_votes, 0);
        assert_eq!(tally.unclear_votes, 1);
        assert_eq!(tally.total_voters, 2);
    }

    #[test]
    fn two_different_voters_can_vote_on_the_same_poll() {
        let (env, _admin, client) = setup();

        client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);
        let tally = client.cast_vote(&voter(&env), &1_u64, &VoteChoice::No);

        assert_eq!(tally.yes_votes, 1);
        assert_eq!(tally.no_votes, 1);
        assert_eq!(tally.total_voters, 2);
    }

    #[test]
    fn same_voter_can_vote_on_two_different_polls() {
        let (env, _admin, client) = setup();
        let v = voter(&env);

        client.cast_vote(&v, &1_u64, &VoteChoice::Yes);
        client.set_poll_status(&2_u64, &PollStatus::Voting);

        let tally = client.cast_vote(&v, &2_u64, &VoteChoice::Yes);

        assert_eq!(tally.poll_id, 2);
        assert_eq!(tally.yes_votes, 1);
        assert_eq!(tally.total_voters, 1);
    }

    fn cast_votes(env: &Env, client: &VotingOracleClient, yes_votes: u32, no_votes: u32) {
        for _ in 0..yes_votes {
            client.cast_vote(&voter(env), &1_u64, &VoteChoice::Yes);
        }
        for _ in 0..no_votes {
            client.cast_vote(&voter(env), &1_u64, &VoteChoice::No);
        }
    }

    #[test]
    fn auto_resolves_at_or_above_threshold_and_emits_event() {
        use soroban_sdk::{testutils::Events, TryIntoVal};

        let (env, _admin, client) = setup();
        cast_votes(&env, &client, 24, 1);
        env.ledger().set_timestamp(1_000_000 + VOTING_WINDOW_SECS);

        let outcome = client.auto_resolve(&1_u64, &1_000_000_000_000i128);
        let events = env.events().all();

        assert_eq!(outcome, VoteChoice::Yes);
        assert_eq!(client.get_poll_status(&1_u64), PollStatus::Resolved);
        assert_eq!(client.get_poll_outcome(&1_u64), VoteChoice::Yes);

        assert_eq!(events.len(), 1);
        let (_, topics, data) = events.get(0).unwrap();
        let name: soroban_sdk::Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        let event_outcome: VoteChoice = topics.get(2).unwrap().try_into_val(&env).unwrap();
        let consensus_bps: u32 = data.try_into_val(&env).unwrap();
        assert_eq!(name, soroban_sdk::Symbol::new(&env, "AutoResolved"));
        assert_eq!(event_outcome, VoteChoice::Yes);
        assert_eq!(consensus_bps, 9_600);
    }

    #[test]
    fn auto_resolve_rejects_consensus_below_threshold() {
        let (env, _admin, client) = setup();
        cast_votes(&env, &client, 54, 10);
        env.ledger().set_timestamp(1_000_000 + VOTING_WINDOW_SECS);

        let err = client
            .try_auto_resolve(&1_u64, &0i128)
            .expect_err("84.9% consensus must not auto-resolve");

        assert_eq!(err, Ok(PredictXError::ConsensusNotReached));
        assert_eq!(client.get_poll_status(&1_u64), PollStatus::Voting);
    }

    #[test]
    fn auto_resolve_rejects_open_voting_window() {
        let (env, _admin, client) = setup();
        cast_votes(&env, &client, 24, 1);

        let err = client
            .try_auto_resolve(&1_u64, &0i128)
            .expect_err("resolution must wait for the voting window to close");

        assert_eq!(err, Ok(PredictXError::VotingNotOpen));
        assert_eq!(client.get_poll_status(&1_u64), PollStatus::Voting);
    }

    // ── consensus_bps ─────────────────────────────────────────────────────────

    fn tally(yes_votes: u32, no_votes: u32, unclear_votes: u32) -> predictx_shared::VoteTally {
        predictx_shared::VoteTally {
            poll_id: 1,
            yes_votes,
            no_votes,
            unclear_votes,
            total_voters: yes_votes + no_votes + unclear_votes,
            voting_end_time: 0,
            reward_pool: 0,
        }
    }

    #[test]
    fn consensus_bps_matches_spec_worked_example() {
        assert_eq!(super::consensus_bps(&tally(45, 2, 0)), (true, 9_574));
        assert_eq!(super::consensus_bps(&tally(2, 45, 0)), (false, 9_574));
    }

    #[test]
    fn consensus_bps_ignores_unclear_votes() {
        assert_eq!(
            super::consensus_bps(&tally(45, 2, 30)),
            super::consensus_bps(&tally(45, 2, 0))
        );
    }

    #[test]
    fn consensus_bps_all_unclear_returns_zero() {
        assert_eq!(super::consensus_bps(&tally(0, 0, 7)), (false, 0));
        assert_eq!(super::consensus_bps(&tally(0, 0, 0)), (false, 0));
    }

    #[test]
    fn consensus_bps_tie_favours_yes() {
        assert_eq!(super::consensus_bps(&tally(10, 10, 3)), (true, 5_000));
    }

    // ── voter_reward_reserve / auto_resolve reward pool ───────────────────────

    #[test]
    fn voter_reward_reserve_is_one_percent_at_default_bps() {
        assert_eq!(
            super::voter_reward_reserve(1_000_000_000_000i128),
            10_000_000_000i128
        );
        assert_eq!(super::voter_reward_reserve(1_000i128), 10i128);
        // Rounded down to whole token units.
        assert_eq!(super::voter_reward_reserve(55i128), 0i128);
    }

    #[test]
    fn auto_resolve_with_no_voters_reserves_zero() {
        let (env, _admin, client) = setup();
        // Seed a tally with zero voters so resolution reaches the voter
        // check with a stored tally in place.
        env.as_contract(&client.address, || {
            storage::write_tally(&env, &tally(0, 0, 0))
        });
        env.ledger().set_timestamp(1_000_000 + VOTING_WINDOW_SECS);

        let err = client
            .try_auto_resolve(&1_u64, &1_000_000_000_000i128)
            .expect_err("a poll with no voters must not resolve");

        assert_eq!(err, Ok(PredictXError::ConsensusNotReached));
        let stored = stored_tally(&env, &client, 1_u64).expect("seeded tally must persist");
        assert_eq!(stored.reward_pool, 0, "no voters means nothing is reserved");
    }

    #[test]
    fn auto_resolve_reserves_reward_pool_from_total_pool() {
        let (env, _admin, client) = setup();
        cast_votes(&env, &client, 24, 1);
        env.ledger().set_timestamp(1_000_000 + VOTING_WINDOW_SECS);

        let total_pool = 1_000_000_000_000i128;
        client.auto_resolve(&1_u64, &total_pool);

        let tally = stored_tally(&env, &client, 1_u64).expect("tally must exist after resolution");
        assert_eq!(
            tally.reward_pool,
            total_pool * i128::from(VOTER_REWARD_BPS) / 10_000,
            "reserve must be VOTER_REWARD_BPS of the settled pool"
        );
    }

    #[test]
    fn auto_resolve_recomputes_nothing_once_resolved() {
        let (env, _admin, client) = setup();
        cast_votes(&env, &client, 24, 1);
        env.ledger().set_timestamp(1_000_000 + VOTING_WINDOW_SECS);

        client.auto_resolve(&1_u64, &1_000_000_000_000i128);
        let tally = stored_tally(&env, &client, 1_u64).expect("tally must exist after resolution");
        assert_eq!(tally.reward_pool, 10_000_000_000i128);

        // A later attempt (e.g. a different pool figure) must not touch the
        // stored reserve — resolution is idempotent-by-rejection.
        let err = client
            .try_auto_resolve(&1_u64, &500_000_000_000i128)
            .expect_err("a resolved poll must not resolve again");
        assert_eq!(err, Ok(PredictXError::VotingNotOpen));

        let tally_after = stored_tally(&env, &client, 1_u64).expect("tally must persist");
        assert_eq!(
            tally_after.reward_pool, 10_000_000_000i128,
            "reserve must be computed exactly once"
        );
    // ── Voter rewards (#105) ──────────────────────────────────────────────────

    /// Fund poll 1 and resolve it to `Yes` (30 Yes / 5 No == 85.7% consensus).
    fn fund_and_resolve_yes(env: &Env, client: &VotingOracleClient, admin: &Address, pool: i128) {
        client.set_reward_pool(admin, &1_u64, &pool);
        env.ledger().set_timestamp(1_000_000 + VOTING_WINDOW_SECS);
        assert_eq!(client.auto_resolve(&1_u64), VoteChoice::Yes);
    }

    #[test]
    fn majority_voter_claims_share_divided_by_eligible_count() {
        let (env, admin, client) = setup();
        let winner = voter(&env);
        client.cast_vote(&winner, &1_u64, &VoteChoice::Yes);
        for _ in 0..29 {
            client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);
        }
        for _ in 0..5 {
            client.cast_vote(&voter(&env), &1_u64, &VoteChoice::No);
        }
        fund_and_resolve_yes(&env, &client, &admin, 300);

        // 300 shared across the 30 eligible (winning) voters, not 300/35.
        assert_eq!(client.claim_reward(&winner, &1_u64), 10);
        assert!(client.has_claimed_reward(&1_u64, &winner));
    }

    #[test]
    fn minority_voter_cannot_claim() {
        let (env, admin, client) = setup();
        let loser = voter(&env);
        client.cast_vote(&loser, &1_u64, &VoteChoice::No);
        for _ in 0..30 {
            client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);
        }
        fund_and_resolve_yes(&env, &client, &admin, 300);

        let err = client
            .try_claim_reward(&loser, &1_u64)
            .expect_err("a losing-side voter must not be paid");

        assert_eq!(err, Ok(PredictXError::VoterNotEligible));
        assert!(!client.has_claimed_reward(&1_u64, &loser));
    }

    #[test]
    fn unclear_voter_cannot_claim() {
        let (env, admin, client) = setup();
        let unclear = voter(&env);
        client.cast_vote(&unclear, &1_u64, &VoteChoice::Unclear);
        for _ in 0..30 {
            client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);
        }
        fund_and_resolve_yes(&env, &client, &admin, 300);

        let err = client
            .try_claim_reward(&unclear, &1_u64)
            .expect_err("an Unclear voter must not be paid");

        assert_eq!(err, Ok(PredictXError::VoterNotEligible));
    }

    #[test]
    fn claim_rejected_before_resolution_and_on_double_claim() {
        let (env, admin, client) = setup();
        let winner = voter(&env);
        client.cast_vote(&winner, &1_u64, &VoteChoice::Yes);
        for _ in 0..29 {
            client.cast_vote(&voter(&env), &1_u64, &VoteChoice::Yes);
        }
        client.set_reward_pool(&admin, &1_u64, &300);

        // No resolved outcome yet.
        let early = client
            .try_claim_reward(&winner, &1_u64)
            .expect_err("claims must wait for a resolved outcome");
        assert_eq!(early, Ok(PredictXError::OutcomeNotAvailable));

        env.ledger().set_timestamp(1_000_000 + VOTING_WINDOW_SECS);
        client.auto_resolve(&1_u64);
        client.claim_reward(&winner, &1_u64);

        let twice = client
            .try_claim_reward(&winner, &1_u64)
            .expect_err("a second claim must be rejected");
        assert_eq!(twice, Ok(PredictXError::AlreadyClaimed));
    }
}
