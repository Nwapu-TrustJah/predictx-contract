#![no_std]

mod matches;
mod payouts;
mod staking;
mod payouts;
pub(crate) mod token_utils;

use predictx_shared::{
    Match, PlatformStats, Poll, PollCategory, PollStatus, PredictXError, Stake, StakeSide,
    UserStats, BPS_DENOMINATOR, MAX_POLLS_PER_MATCH,
};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol, Vec};

mod voting_oracle {
    soroban_sdk::contractimport!(file = "wasm/voting_oracle.wasm");
}

/// Central poll-status state machine.
///
/// Legal transition graph:
///
/// ```text
/// Active ──→ Locked ──→ Voting ──→ AdminReview
///   │                        │    └──→ Disputed ──→ Resolved
///   │                        └───────────────→ Resolved
///   └──────→ Cancelled        Locked ────────→ Cancelled
/// ```
///
/// - `Active` → `Locked` : staking window closed (lock time reached).
/// - `Locked` → `Voting` : match finished; community voting opens.
/// - `Voting` → `AdminReview` : consensus between the review thresholds.
/// - `Voting` → `Disputed` : result challenged during the dispute window.
/// - `AdminReview` → `Resolved` : admins finalise the outcome.
/// - `Disputed` → `Resolved` : dispute settled.
/// - `Voting` → `Resolved` : consensus reached the auto-resolve threshold.
/// - `Active` | `Locked` → `Cancelled` : emergency cancellation (refunds).
///
/// `Resolved` and `Cancelled` are terminal — no outgoing transitions.
/// Every other status change is rejected with `InvalidStateTransition`.
pub(crate) fn transition_status(
    env: &Env,
    poll: &Poll,
    to: PollStatus,
) -> Result<(), PredictXError> {
    use PollStatus::{
        Active, AdminReview, Cancelled, Disputed, Locked, Resolved, Voting,
    };

    let legal = match poll.status {
        Active => matches!(to, Locked | Cancelled),
        Locked => matches!(to, Voting | Cancelled),
        Voting => matches!(to, AdminReview | Disputed | Resolved),
        AdminReview => matches!(to, Resolved),
        Disputed => matches!(to, Resolved),
        // Terminal states accept no further transitions.
        Resolved | Cancelled => false,
    };

    if !legal {
        return Err(PredictXError::InvalidStateTransition);
    }

    env.events().publish(
        (Symbol::new(env, "PollStatusChanged"), poll.poll_id),
        (poll.status, to),
   );
    Ok(())
}

fn map_oracle_poll_status(status: voting_oracle::PollStatus) -> PollStatus {
    match status {
        voting_oracle::PollStatus::Active      => PollStatus::Active,
        voting_oracle::PollStatus::Locked      => PollStatus::Locked,
        voting_oracle::PollStatus::Voting      => PollStatus::Voting,
        voting_oracle::PollStatus::AdminReview => PollStatus::AdminReview,
        voting_oracle::PollStatus::Disputed    => PollStatus::Disputed,
        voting_oracle::PollStatus::Resolved    => PollStatus::Resolved,
        voting_oracle::PollStatus::Cancelled   => PollStatus::Cancelled,
    }
}

#[contract]
pub struct PredictionMarket;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    // ── oracle / admin keys ───────────────────────────────────────────────────
    Admin,
    PendingAdmin,
    VotingOracle,
    Paused,
    TokenAddress,
    TreasuryAddress,
    PlatformFeeBps,
    Stake(u64, Address),
    EmergencyClaimed(u64, Address),
    /// Set once the platform fee for a poll has been sent to the treasury.
    FeePaid(u64),
    PlatformStats,
    // ── match management keys ─────────────────────────────────────────────────
    Initialized,
    NextMatchId,
    NextPollId,
    Match(u64),
    MatchPolls(u64),
    // ── poll & staking keys ───────────────────────────────────────────────────
    Poll(u64),
    UserStakes(Address),
    HasStaked(u64, Address),
    /// `user` → `UserStats` — activity totals returned by the stats views. (Persistent)
    /// Appended last so the discriminants of the existing keys never shift.
    UserStats(Address),
}

/// Pool state returned by `get_pool_info`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolInfo {
    pub yes_pool: i128,
    pub no_pool: i128,
    pub yes_count: u32,
    pub no_count: u32,
}

fn get_admin(env: &Env) -> Result<Address, PredictXError> {
    env.storage().instance().get(&DataKey::Admin)
        .ok_or(PredictXError::NotInitialized)
}

pub(crate) fn get_oracle(env: &Env) -> Result<Address, PredictXError> {
    env.storage().instance().get(&DataKey::VotingOracle)
        .ok_or(PredictXError::NotInitialized)
}

fn is_paused(env: &Env) -> bool {
    env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
}

pub(crate) fn ensure_not_paused(env: &Env) -> Result<(), PredictXError> {
    if is_paused(env) {
        return Err(PredictXError::EmergencyWithdrawNotAllowed);
    }
    Ok(())
}

pub(crate) fn get_platform_stats(env: &Env) -> PlatformStats {
    env.storage().instance().get(&DataKey::PlatformStats)
        .unwrap_or(PlatformStats {
            total_value_locked: 0,
            total_polls_created: 0,
            total_stakes_placed: 0,
            total_payouts: 0,
            total_users: 0,
        })
}

pub(crate) fn set_platform_stats(env: &Env, stats: &PlatformStats) {
    env.storage().instance().set(&DataKey::PlatformStats, stats);
}

/// A `UserStats` record with every counter at zero.
pub(crate) fn empty_user_stats() -> UserStats {
    UserStats {
        total_staked: 0,
        total_won: 0,
        total_lost: 0,
        polls_participated: 0,
        polls_won: 0,
        polls_lost: 0,
        votes_cast: 0,
        voting_rewards_earned: 0,
    }
}

/// Read a user's activity totals.
///
/// Users with no recorded activity get a zeroed record rather than an error so
/// callers never have to special-case a first look-up.
pub(crate) fn get_user_stats(env: &Env, user: &Address) -> UserStats {
    env.storage()
        .persistent()
        .get(&DataKey::UserStats(user.clone()))
        .unwrap_or_else(empty_user_stats)
}

/// A user's win rate in basis points:
/// `polls_won * BPS_DENOMINATOR / (polls_won + polls_lost)`.
///
/// Returns `0` when no polls have settled yet — a brand-new account must not
/// divide by zero, and an unsettled record has no meaningful ratio.
pub(crate) fn get_user_win_rate(env: &Env, user: &Address) -> u32 {
    let stats = get_user_stats(env, user);
    let settled = stats.polls_won as u64 + stats.polls_lost as u64;
    if settled == 0 {
        return 0;
    }
    (stats.polls_won as u64 * BPS_DENOMINATOR as u64 / settled) as u32
}

fn load_stake(env: &Env, poll_id: u64, user: &Address) -> Option<Stake> {
    env.storage().persistent().get(&DataKey::Stake(poll_id, user.clone()))
}

fn has_emergency_claimed(env: &Env, poll_id: u64, user: &Address) -> bool {
    env.storage().persistent()
        .get(&DataKey::EmergencyClaimed(poll_id, user.clone()))
        .unwrap_or(false)
}

fn set_emergency_claimed(env: &Env, poll_id: u64, user: &Address) {
    env.storage().persistent()
        .set(&DataKey::EmergencyClaimed(poll_id, user.clone()), &true);
}

/// Extend the TTL of instance storage so the contract's admin, token and
/// configuration entries are not archived during periods of inactivity.
pub(crate) fn extend_instance_ttl(env: &Env) {
    env.storage().instance().extend_ttl(100, 1000);
}

const EMERGENCY_TIMEOUT_SECS: u64 = 7 * 24 * 60 * 60;

/// Poll statuses for which the oracle will never be consulted again.
fn is_terminal_poll_status(status: PollStatus) -> bool {
    matches!(status, PollStatus::Resolved | PollStatus::Cancelled)
}

/// Whether any poll created through this contract is still open.
///
/// Poll ids are allocated sequentially from `NextPollId`, so a bounded scan of
/// `1..NextPollId` visits every poll this contract has ever created.
fn has_unresolved_polls(env: &Env) -> bool {
    let next_poll_id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextPollId)
        .unwrap_or(1);
    let mut poll_id: u64 = 1;
    while poll_id < next_poll_id {
        if let Some(poll) = env
            .storage()
            .persistent()
            .get::<DataKey, Poll>(&DataKey::Poll(poll_id))
        {
            if !is_terminal_poll_status(poll.status) {
                return true;
            }
        }
        poll_id += 1;
    }
    false
}

/// Confirm `oracle_id` is a live, initialised `VotingOracle`.
///
/// Probes the well-known `admin` view over the oracle interface. A missing
/// contract, a non-oracle contract, or an uninitialised oracle all fail this
/// check and map to `PredictXError::InvalidOracle`.
fn assert_compatible_oracle(env: &Env, oracle_id: &Address) -> Result<(), PredictXError> {
    let client = voting_oracle::Client::new(env, oracle_id);
    match client.try_admin() {
        Ok(Ok(_)) => Ok(()),
        _ => Err(PredictXError::InvalidOracle),
    }
}

#[contractimpl]
impl PredictionMarket {
    pub fn initialize(
        env: Env,
        admin: Address,
        voting_oracle: Address,
        token_address: Address,
        treasury_address: Address,
        platform_fee_bps: u32,
    ) -> Result<(), PredictXError> {
        extend_instance_ttl(&env);
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(PredictXError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::VotingOracle, &voting_oracle);
        env.storage().instance().set(&DataKey::TokenAddress, &token_address);
        env.storage().instance().set(&DataKey::TreasuryAddress, &treasury_address);
        env.storage().instance().set(&DataKey::PlatformFeeBps, &platform_fee_bps);
        env.storage().instance().set(&DataKey::NextMatchId, &1u64);
        env.storage().instance().set(&DataKey::NextPollId, &1u64);
        env.storage().instance().set(&DataKey::Initialized, &true);
        Ok(())
    }

    /// Propose a new admin. The proposal does not change the active admin
    /// until the candidate calls `accept_admin`. Only the current admin may
    /// propose, and a new proposal overwrites any pending one.
    pub fn propose_admin(
        env: Env,
        current: Address,
        candidate: Address,
    ) -> Result<(), PredictXError> {
        let stored_admin = get_admin(&env)?;
        if current != stored_admin {
            return Err(PredictXError::Unauthorized);
        }
        current.require_auth();
        env.storage().instance().set(&DataKey::PendingAdmin, &candidate);
        env.events()
            .publish((Symbol::new(&env, "AdminProposed"),), candidate);
        Ok(())
    }

    /// Cancel a pending admin proposal. Only the current admin may cancel.
    pub fn cancel_admin_proposal(env: Env, current: Address) -> Result<(), PredictXError> {
        let stored_admin = get_admin(&env)?;
        if current != stored_admin {
            return Err(PredictXError::Unauthorized);
        }
        current.require_auth();
        env.storage().instance().remove(&DataKey::PendingAdmin);
        Ok(())
    }

    /// Accept a pending admin proposal. Only the proposed candidate may call
    /// this, and it completes the handover: the candidate becomes the active
    /// admin and the previous admin loses access.
    pub fn accept_admin(env: Env, candidate: Address) -> Result<(), PredictXError> {
        let pending: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .ok_or(PredictXError::Unauthorized)?;
        if candidate != pending {
            return Err(PredictXError::Unauthorized);
        }
        candidate.require_auth();
        env.storage().instance().set(&DataKey::Admin, &candidate);
        env.storage().instance().remove(&DataKey::PendingAdmin);
        env.events()
            .publish((Symbol::new(&env, "AdminTransferred"),), candidate);
        Ok(())
    }

    pub fn admin(env: Env) -> Result<Address, PredictXError> { get_admin(&env) }
    pub fn oracle(env: Env) -> Result<Address, PredictXError> { get_oracle(&env) }

    /// Rotate the voting oracle that governs this market.
    ///
    /// Rotation is refused while any poll is still unresolved: a replacement
    /// oracle has no record of the existing poll ids, so in-flight polls would
    /// silently fall back to `Active` with `updated_at = 0`, emergency claims
    /// would stop being eligible, and already-staked funds would become
    /// unrecoverable. The target must also be a live, initialised
    /// `VotingOracle`. On success an `OracleChanged(previous, next)` event is
    /// emitted.
    pub fn set_oracle(env: Env, voting_oracle: Address) -> Result<(), PredictXError> {
        extend_instance_ttl(&env);
        ensure_not_paused(&env)?;
        let admin = get_admin(&env)?;
        admin.require_auth();

        let previous = get_oracle(&env)?;

        if has_unresolved_polls(&env) {
            return Err(PredictXError::OracleRotationBlocked);
        }

        assert_compatible_oracle(&env, &voting_oracle)?;

        env.storage().instance().set(&DataKey::VotingOracle, &voting_oracle);
        env.events().publish(
            (Symbol::new(&env, "OracleChanged"),),
            (previous, voting_oracle),
        );
        Ok(())
    }

    pub fn pause(env: Env, admin: Address) -> Result<(), PredictXError> {
        extend_instance_ttl(&env);
        let stored_admin = get_admin(&env)?;
        if admin != stored_admin { return Err(PredictXError::Unauthorized); }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish((Symbol::new(&env, "ContractPaused"),), true);
        Ok(())
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), PredictXError> {
        extend_instance_ttl(&env);
        let stored_admin = get_admin(&env)?;
        if admin != stored_admin { return Err(PredictXError::Unauthorized); }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &false);
        env.events().publish((Symbol::new(&env, "ContractUnpaused"),), true);
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool { is_paused(&env) }

    pub fn oracle_poll_status(env: Env, poll_id: u64) -> Result<PollStatus, PredictXError> {
        let oracle_id = get_oracle(&env)?;
        let client = voting_oracle::Client::new(&env, &oracle_id);
        Ok(map_oracle_poll_status(client.get_poll_status(&poll_id)))
    }

    pub fn cancel_poll(env: Env, admin: Address, poll_id: u64) -> Result<(), PredictXError> {
        extend_instance_ttl(&env);
        ensure_not_paused(&env)?;
        let stored_admin = get_admin(&env)?;
        if admin != stored_admin { return Err(PredictXError::Unauthorized); }
        admin.require_auth();

        let mut poll: Poll = env
            .storage()
            .persistent()
            .get(&DataKey::Poll(poll_id))
            .ok_or(PredictXError::PollNotFound)?;
        if poll.status == PollStatus::Resolved
            || poll.status == PollStatus::Cancelled
            || poll.outcome.is_some()
        {
            return Err(PredictXError::PollAlreadyResolved);
        }

        let oracle_id = get_oracle(&env)?;
        let client = voting_oracle::Client::new(&env, &oracle_id);
        let oracle_status = client.get_poll_status(&poll_id);
        if oracle_status == voting_oracle::PollStatus::Resolved
            || oracle_status == voting_oracle::PollStatus::Cancelled
        {
            return Err(PredictXError::PollAlreadyResolved);
        }
        client.set_poll_status(&poll_id, &voting_oracle::PollStatus::Cancelled);
        poll.status = PollStatus::Cancelled;
        env.storage()
            .persistent()
            .set(&DataKey::Poll(poll_id), &poll);
        env.events().publish((Symbol::new(&env, "PollCancelled"),), poll_id);
        Ok(())
    }

    pub fn check_emergency_eligible(env: Env, poll_id: u64) -> bool {
        let oracle_id = match get_oracle(&env) {
            Ok(id) => id,
            Err(_) => return false,
        };
        let client = voting_oracle::Client::new(&env, &oracle_id);
        let status = map_oracle_poll_status(client.get_poll_status(&poll_id));
        if status == PollStatus::Cancelled { return true; }
        if status != PollStatus::Disputed && status != PollStatus::Locked { return false; }
        let updated_at = client.get_poll_status_updated_at(&poll_id);
        if updated_at == 0 { return false; }
        env.ledger().timestamp().saturating_sub(updated_at) >= EMERGENCY_TIMEOUT_SECS
    }

    pub fn emergency_withdraw(env: Env, user: Address, poll_id: u64) -> Result<i128, PredictXError> {
        extend_instance_ttl(&env);
        user.require_auth();
        if has_emergency_claimed(&env, poll_id, &user) {
            return Err(PredictXError::AlreadyClaimed);
        }
        let oracle_id = get_oracle(&env)?;
        let client = voting_oracle::Client::new(&env, &oracle_id);
        let status = map_oracle_poll_status(client.get_poll_status(&poll_id));
        let eligible = if status == PollStatus::Cancelled {
            true
        } else if status == PollStatus::Disputed || status == PollStatus::Locked {
            let updated_at = client.get_poll_status_updated_at(&poll_id);
            updated_at != 0 && env.ledger().timestamp().saturating_sub(updated_at) >= EMERGENCY_TIMEOUT_SECS
        } else {
            false
        };
        if !eligible { return Err(PredictXError::EmergencyWithdrawNotAllowed); }
        let stake = load_stake(&env, poll_id, &user).ok_or(PredictXError::NotStaker)?;
        set_emergency_claimed(&env, poll_id, &user);

        // Transfer tokens back to user
        token_utils::transfer_from_contract(&env, &user, stake.amount)?;

        let mut stats = get_platform_stats(&env);
        stats.total_value_locked -= stake.amount;
        set_platform_stats(&env, &stats);
        env.events().publish((Symbol::new(&env, "EmergencyWithdrawal"), poll_id, user.clone()), stake.amount);
        Ok(stake.amount)
    }

    // ── Poll management ──────────────────────────────────────────────────────

    pub fn create_poll(
        env: Env,
        creator: Address,
        match_id: u64,
        question: String,
        category: PollCategory,
        lock_time: u64,
    ) -> Result<u64, PredictXError> {
        extend_instance_ttl(&env);
        ensure_not_paused(&env)?;
        creator.require_auth();

        // Validate match exists
        let _m: Match = env
            .storage()
            .persistent()
            .get(&DataKey::Match(match_id))
            .ok_or(PredictXError::MatchNotFound)?;

        // Validate lock_time is in the future
        if lock_time <= env.ledger().timestamp() {
            return Err(PredictXError::InvalidLockTime);
        }

        // Check max polls per match
        let mut match_polls: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::MatchPolls(match_id))
            .unwrap_or(Vec::new(&env));
        if match_polls.len() >= MAX_POLLS_PER_MATCH {
            return Err(PredictXError::MaxPollsPerMatchReached);
        }

        let poll_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextPollId)
            .unwrap_or(1);

        let poll = Poll {
            poll_id,
            match_id,
            creator: creator.clone(),
            question,
            category,
            lock_time,
            yes_pool: 0,
            no_pool: 0,
            yes_count: 0,
            no_count: 0,
            status: PollStatus::Active,
            outcome: None,
            resolution_time: 0,
            created_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Poll(poll_id), &poll);

        match_polls.push_back(poll_id);
        env.storage()
            .persistent()
            .set(&DataKey::MatchPolls(match_id), &match_polls);

        env.storage()
            .instance()
            .set(&DataKey::NextPollId, &(poll_id + 1));

        let mut stats = get_platform_stats(&env);
        stats.total_polls_created += 1;
        set_platform_stats(&env, &stats);

        env.events()
            .publish((Symbol::new(&env, "PollCreated"), poll_id), ());

        Ok(poll_id)
    }


    /// Resolve a poll with a boolean outcome. Callable only by the registered
    /// oracle; delegates to [`payouts::resolve_poll`].
    pub fn resolve_poll(
    /// Resolve a poll with a boolean outcome. Callable only by the registered oracle.
    pub fn oracle_resolve_poll(
        env: Env,
        caller: Address,
        poll_id: u64,
        outcome: bool,
    ) -> Result<(), PredictXError> {
        extend_instance_ttl(&env);
        ensure_not_paused(&env)?;
        caller.require_auth();
        let oracle = get_oracle(&env)?;
        if caller != oracle {
            return Err(PredictXError::Unauthorized);
        }

    pub fn get_poll(env: Env, poll_id: u64) -> Result<Poll, PredictXError> {
        let mut poll: Poll = env
            .storage()
            .persistent()
            .get(&DataKey::Poll(poll_id))
            .ok_or(PredictXError::PollNotFound)?;

        // Lazily self-correct (issue #125): a poll past its lock time whose
        // stored status is still `Active` is persisted as `Locked` on read.
        // This is the access path where the correction can actually survive:
        // writes performed by a *failed* `stake` invocation are reverted by
        // the host, so the persisted lock lands here instead.
        if poll.status == PollStatus::Active && env.ledger().timestamp() >= poll.lock_time {
            poll.status = PollStatus::Locked;
            env.storage()
                .persistent()
                .set(&DataKey::Poll(poll_id), &poll);
        }

        Ok(poll)
        payouts::resolve_poll(&env, caller, poll_id, outcome)
    }

    pub fn get_poll(env: Env, poll_id: u64) -> Result<Poll, PredictXError> {
        env.storage()
            .persistent()
            .get(&DataKey::Poll(poll_id))
            .ok_or(PredictXError::PollNotFound)
    }

    // ── Staking ───────────────────────────────────────────────────────────────

    pub fn stake(
        env: Env,
        staker: Address,
        poll_id: u64,
        amount: i128,
        side: StakeSide,
    ) -> Result<Stake, PredictXError> {
        extend_instance_ttl(&env);
        staking::stake(&env, staker, poll_id, amount, side)
    }

    pub fn get_stake_info(env: Env, poll_id: u64, user: Address) -> Result<Stake, PredictXError> {
        staking::get_stake_info(&env, poll_id, &user)
    }

    pub fn get_user_stakes(env: Env, user: Address) -> Vec<u64> {
        staking::get_user_stakes(&env, &user)
    }

    pub fn get_user_stakes_paged(env: Env, user: Address, start: u32, limit: u32) -> Vec<u64> {
        staking::get_user_stakes_paged(&env, &user, start, limit)
    }

    pub fn has_user_staked(env: Env, poll_id: u64, user: Address) -> bool {
        staking::has_user_staked(&env, poll_id, &user)
    }

    pub fn calculate_potential_winnings(
        env: Env,
        poll_id: u64,
        side: StakeSide,
        amount: i128,
    ) -> Result<i128, PredictXError> {
        staking::calculate_potential_winnings(&env, poll_id, side, amount)
    }

    pub fn get_pool_info(env: Env, poll_id: u64) -> Result<PoolInfo, PredictXError> {
        staking::get_pool_info(&env, poll_id)
    }

    /// Read-only view: return the token amount that `claim_winnings` would
    /// transfer to `user` for the given poll.  Returns `0` for every
    /// ineligible case (unresolved poll, non-staker, losing staker,
    /// already-claimed) rather than erroring.  No `require_auth` — public.
    pub fn get_claimable_amount(env: Env, poll_id: u64, user: Address) -> i128 {
        payouts::get_claimable_amount(&env, poll_id, &user)
    }

    /// Claim winnings for a resolved poll.
    ///
    /// Transfers the same amount `get_claimable_amount` would return.
    /// Requires the caller to be the staker (`user.require_auth()`).
    pub fn claim_winnings(env: Env, user: Address, poll_id: u64) -> Result<i128, PredictXError> {
        ensure_not_paused(&env)?;
        payouts::claim_winnings_for_poll(&env, user, poll_id)
    }

    pub fn get_platform_stats(env: Env) -> PlatformStats {
        get_platform_stats(&env)
    }

    // ── User stats view functions ─────────────────────────────────────────────

    /// Per-user activity totals. Unknown users get a zeroed record.
    pub fn get_user_stats(env: Env, user: Address) -> UserStats {
        get_user_stats(&env, &user)
    }

    /// The user's win rate in basis points (`10_000` = 100%).
    ///
    /// Returns `0` for users with no settled polls.
    pub fn get_user_win_rate(env: Env, user: Address) -> u32 {
        get_user_win_rate(&env, &user)
    }

    // ── Token view functions ──────────────────────────────────────────────────

    pub fn get_token_address(env: Env) -> Result<Address, PredictXError> {
        token_utils::get_token_address(&env)
    }

    pub fn get_treasury_address(env: Env) -> Result<Address, PredictXError> {
        token_utils::get_treasury_address(&env)
    }

    pub fn get_platform_fee_bps(env: Env) -> u32 {
        token_utils::get_platform_fee_bps(&env)
    }

    pub fn get_contract_balance(env: Env) -> Result<i128, PredictXError> {
        token_utils::get_balance(&env)
    }

    // ── Match management ──────────────────────────────────────────────────────

    pub fn create_match(
        env: Env, admin: Address,
        home_team: String, away_team: String,
        league: String, venue: String,
        kickoff_time: u64,
    ) -> Result<u64, PredictXError> {
        extend_instance_ttl(&env);
        matches::create_match(&env, admin, home_team, away_team, league, venue, kickoff_time)
    }

    pub fn update_match(
        env: Env, admin: Address, match_id: u64,
        home_team: Option<String>, away_team: Option<String>,
        league: Option<String>, venue: Option<String>,
        kickoff_time: Option<u64>,
    ) -> Result<Match, PredictXError> {
        extend_instance_ttl(&env);
        matches::update_match(&env, admin, match_id, home_team, away_team, league, venue, kickoff_time)
    }

    pub fn finish_match(env: Env, admin: Address, match_id: u64) -> Result<(), PredictXError> {
        extend_instance_ttl(&env);
        matches::finish_match(&env, admin, match_id)
    }

    pub fn get_match(env: Env, match_id: u64) -> Result<Match, PredictXError> {
        matches::get_match(&env, match_id)
    }

    pub fn get_match_polls(env: Env, match_id: u64) -> Result<Vec<u64>, PredictXError> {
        matches::get_match_polls(&env, match_id)
    }

    pub fn get_match_count(env: Env) -> u64 {
        matches::get_match_count(&env)
    }

    // ── Payouts ───────────────────────────────────────────────────────────────

    pub fn resolve_poll(
        env: Env,
        admin: Address,
        poll_id: u64,
        outcome: bool,
    ) -> Result<(), PredictXError> {
        extend_instance_ttl(&env);
        payouts::resolve_poll(&env, admin, poll_id, outcome)
    }

    /// Claim winnings after a resolved poll.
    ///
    /// If the winning pool is empty (every staker was on the losing side),
    /// any staker may recover their original stake fee-free.  See
    /// [`payouts::claim_winnings`] for the full description.
    pub fn claim_winnings(
        env: Env,
        claimant: Address,
        poll_id: u64,
    ) -> Result<i128, PredictXError> {
        extend_instance_ttl(&env);
        payouts::claim_winnings(&env, claimant, poll_id)
    }

    pub fn calculate_winnings(
        env: Env,
        poll_id: u64,
        user: Address,
    ) -> Result<i128, PredictXError> {
        extend_instance_ttl(&env);
        payouts::calculate_winnings(&env, poll_id, user)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod test {
    use super::*;
    use predictx_shared::{PollCategory, StakeSide};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::token;

    /// Default platform fee BPS for tests (5%).
    const TEST_FEE_BPS: u32 = 500;

    fn seed_active_poll(env: &Env, contract_id: &Address, poll_id: u64) {
        let poll = Poll {
            poll_id,
            match_id: 1,
            creator: Address::generate(env),
            question: String::from_str(env, "Will the home team win?"),
            category: PollCategory::TeamEvent,
            lock_time: env.ledger().timestamp() + 3_600,
            yes_pool: 0,
            no_pool: 0,
            yes_count: 0,
            no_count: 0,
            status: PollStatus::Active,
            outcome: None,
            resolution_time: 0,
            created_at: env.ledger().timestamp(),
        };
        env.as_contract(contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::Poll(poll_id), &poll);
        });
    }

    #[test]
    fn initialize_sets_admin_and_oracle() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let token = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &token, &treasury, &TEST_FEE_BPS);
        assert_eq!(client.admin(), admin);
        assert_eq!(client.oracle(), oracle);
    }

    #[test]
    fn initialize_stores_token_and_treasury() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        assert_eq!(client.get_token_address(), tok);
        assert_eq!(client.get_treasury_address(), treasury);
        assert_eq!(client.get_platform_fee_bps(), TEST_FEE_BPS);
    }

    #[test]
    fn initialize_is_one_time_only() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let token = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &token, &treasury, &TEST_FEE_BPS);
        let err = client.try_initialize(&admin, &oracle, &token, &treasury, &TEST_FEE_BPS).expect_err("should fail");
        assert_eq!(err, Ok(PredictXError::AlreadyInitialized));
    }

    #[test]
    fn cross_contract_oracle_call_works() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let oracle_id = env.register(voting_oracle::WASM, ());
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        oracle_client.initialize(&admin, &tok);
        oracle_client.set_poll_status(&7_u64, &voting_oracle::PollStatus::Resolved);
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        client.initialize(&admin, &oracle_id, &tok, &treasury, &TEST_FEE_BPS);
        let status = client.oracle_poll_status(&7_u64);
        assert_eq!(status, PollStatus::Resolved);
    }

    #[test]
    fn pause_and_unpause_toggle_contract_state() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        assert_eq!(client.is_paused(), false);
        client.pause(&admin);
        assert_eq!(client.is_paused(), true);
        let err = client.try_set_oracle(&oracle).expect_err("should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));
        client.unpause(&admin);
        assert_eq!(client.is_paused(), false);
    }

    #[test]
    fn cancel_poll_sets_cancelled_status_and_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let oracle_id = env.register(voting_oracle::WASM, ());
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);
        let tok = Address::generate(&env);
        oracle_client.initialize(&admin, &tok);
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle_id, &tok, &treasury, &TEST_FEE_BPS);
        seed_active_poll(&env, &contract_id, 1);
        client.cancel_poll(&admin, &1_u64);
        assert_eq!(client.get_poll(&1_u64).status, PollStatus::Cancelled);
        assert_eq!(oracle_client.get_poll_status(&1_u64), voting_oracle::PollStatus::Cancelled);
        assert_eq!(client.oracle_poll_status(&1_u64), PollStatus::Cancelled);

        let staker = Address::generate(&env);
        let err = client
            .try_stake(&staker, &1_u64, &50_i128, &StakeSide::Yes)
            .expect_err("cancelled polls must not accept stakes");
        assert_eq!(err, Ok(PredictXError::PollNotActive));

        let err = client
            .try_cancel_poll(&admin, &1_u64)
            .expect_err("terminal polls cannot be cancelled again");
        assert_eq!(err, Ok(PredictXError::PollAlreadyResolved));
    }

    // Helper to set up a real-token environment for emergency withdrawal tests
    fn setup_emergency_env() -> (Env, Address, Address, Address, PredictionMarketClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        // Real token for transfers
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_addr = token_contract.address();
        let treasury = Address::generate(&env);

        let oracle_id = env.register(voting_oracle::WASM, ());
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);
        oracle_client.initialize(&admin, &token_addr);

        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        client.initialize(&admin, &oracle_id, &token_addr, &treasury, &TEST_FEE_BPS);

        (env, admin, oracle_id, contract_id, client)
    }

    /// Mint tokens to an address using the real SAC
    fn mint_to(env: &Env, token_addr: &Address, to: &Address, amount: i128) {
        let sac = token::StellarAssetClient::new(env, token_addr);
        sac.mint(to, &amount);
    }

    #[test]
    fn emergency_withdraw_on_cancelled_poll_refunds_stake() {
        let (env, admin, oracle_id, contract_id, client) = setup_emergency_env();
        let _oracle_client = voting_oracle::Client::new(&env, &oracle_id);
        let token_addr: Address = env.as_contract(&contract_id, || {
            env.storage().instance().get(&DataKey::TokenAddress).unwrap()
        });

        let user = Address::generate(&env);
        let amount: i128 = 50;

        // Fund the contract so it can transfer back
        mint_to(&env, &token_addr, &contract_id, amount);

        let stake = Stake { user: user.clone(), poll_id: 10, amount, side: StakeSide::Yes, claimed: false, staked_at: env.ledger().timestamp() };
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&DataKey::Stake(10, user.clone()), &stake);
        });
        seed_active_poll(&env, &contract_id, 10);
        client.cancel_poll(&admin, &10_u64);
        let refunded = client.emergency_withdraw(&user, &10_u64);
        assert_eq!(refunded, amount);

        // Verify token was transferred
        let tok = token::Client::new(&env, &token_addr);
        assert_eq!(tok.balance(&user), amount);
        assert_eq!(tok.balance(&contract_id), 0);
    }

    #[test]
    fn emergency_withdraw_after_dispute_timeout() {
        let (env, _admin, oracle_id, contract_id, client) = setup_emergency_env();
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);
        let token_addr: Address = env.as_contract(&contract_id, || {
            env.storage().instance().get(&DataKey::TokenAddress).unwrap()
        });

        env.ledger().set_timestamp(100);
        oracle_client.set_poll_status(&5_u64, &voting_oracle::PollStatus::Disputed);

        let user = Address::generate(&env);
        let amount: i128 = 25;
        mint_to(&env, &token_addr, &contract_id, amount);

        let stake = Stake { user: user.clone(), poll_id: 5, amount, side: StakeSide::No, claimed: false, staked_at: env.ledger().timestamp() };
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&DataKey::Stake(5, user.clone()), &stake);
        });
        env.ledger().set_timestamp(100 + EMERGENCY_TIMEOUT_SECS + 1);
        assert!(client.check_emergency_eligible(&5_u64));
        let refunded = client.emergency_withdraw(&user, &5_u64);
        assert_eq!(refunded, amount);
    }

    #[test]
    fn emergency_withdraw_rejected_before_timeout() {
        let (env, _admin, oracle_id, contract_id, client) = setup_emergency_env();
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);

        env.ledger().set_timestamp(200);
        oracle_client.set_poll_status(&2_u64, &voting_oracle::PollStatus::Locked);

        let user = Address::generate(&env);
        let stake = Stake { user: user.clone(), poll_id: 2, amount: 30, side: StakeSide::Yes, claimed: false, staked_at: env.ledger().timestamp() };
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&DataKey::Stake(2, user.clone()), &stake);
        });
        env.ledger().set_timestamp(200 + EMERGENCY_TIMEOUT_SECS - 1);
        assert!(!client.check_emergency_eligible(&2_u64));
        let err = client.try_emergency_withdraw(&user, &2_u64).expect_err("should reject");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));
    }

    #[test]
    fn emergency_withdraw_prevents_double_withdrawal() {
        let (env, _admin, oracle_id, contract_id, client) = setup_emergency_env();
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);
        let token_addr: Address = env.as_contract(&contract_id, || {
            env.storage().instance().get(&DataKey::TokenAddress).unwrap()
        });

        env.ledger().set_timestamp(300);
        oracle_client.set_poll_status(&3_u64, &voting_oracle::PollStatus::Disputed);

        let user = Address::generate(&env);
        let amount: i128 = 40;
        mint_to(&env, &token_addr, &contract_id, amount);

        let stake = Stake { user: user.clone(), poll_id: 3, amount, side: StakeSide::No, claimed: false, staked_at: env.ledger().timestamp() };
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&DataKey::Stake(3, user.clone()), &stake);
        });
        env.ledger().set_timestamp(300 + EMERGENCY_TIMEOUT_SECS + 1);
        let refunded = client.emergency_withdraw(&user, &3_u64);
        assert_eq!(refunded, amount);
        let err = client.try_emergency_withdraw(&user, &3_u64).expect_err("double withdrawal should fail");
        assert_eq!(err, Ok(PredictXError::AlreadyClaimed));
    }

    /// Force a poll's stored status (test scaffolding for lifecycle tests).
    fn force_poll_status(env: &Env, contract_id: &Address, poll_id: u64, status: PollStatus) {
        env.as_contract(contract_id, || {
            let mut poll: Poll = env.storage().persistent().get(&DataKey::Poll(poll_id)).unwrap();
            poll.status = status;
            env.storage().persistent().set(&DataKey::Poll(poll_id), &poll);
        });
    }

    fn seed_active_poll(env: &Env, contract_id: &Address, poll_id: u64, creator: &Address) {
        let poll = Poll {
            poll_id,
            match_id: 1,
            creator: creator.clone(),
            question: String::from_str(env, "Will the home team win?"),
            category: PollCategory::TeamEvent,
            lock_time: env.ledger().timestamp() + 3600,
            yes_pool: 0,
            no_pool: 0,
            yes_count: 0,
            no_count: 0,
            status: PollStatus::Active,
            outcome: None,
            resolution_time: 0,
            created_at: env.ledger().timestamp(),
        };
        env.as_contract(contract_id, || {
            env.storage().persistent().set(&DataKey::Poll(poll_id), &poll);
        });
    }

    #[test]
    fn resolve_poll_rejects_non_oracle() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let stranger = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        seed_active_poll(&env, &contract_id, 1, &admin);
        let err = client.try_oracle_resolve_poll(&stranger, &1_u64, &true).expect_err("non-oracle");
        assert_eq!(err, Ok(PredictXError::Unauthorized));
    }

    #[test]
    fn resolve_poll_rejects_unknown_poll() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        let err = client.try_resolve_poll(&admin, &99_u64, &false).expect_err("missing");
        let err = client.try_oracle_resolve_poll(&oracle, &99_u64, &false).expect_err("missing");
        assert_eq!(err, Ok(PredictXError::PollNotFound));
    }

    #[test]
    fn resolve_poll_sets_outcome_and_resolution_time() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_700_000_000);
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        seed_active_poll(&env, &contract_id, 7, &admin);
        // Resolution is only legal once the poll has reached the voting phase.
        force_poll_status(&env, &contract_id, 7, PollStatus::Voting);
        client.resolve_poll(&admin, &7_u64, &true);
        client.oracle_resolve_poll(&oracle, &7_u64, &true);
        let poll = client.get_poll(&7_u64);
        assert_eq!(poll.outcome, Some(true));
        assert_eq!(poll.resolution_time, 1_700_000_000);
        assert_eq!(poll.status, PollStatus::Resolved);
    }

    #[test]
    fn resolve_poll_rejects_already_resolved() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        seed_active_poll(&env, &contract_id, 3, &admin);
        force_poll_status(&env, &contract_id, 3, PollStatus::Voting);
        client.resolve_poll(&admin, &3_u64, &false);
        let err = client.try_resolve_poll(&admin, &3_u64, &true).expect_err("already");
        client.oracle_resolve_poll(&oracle, &3_u64, &false);
        let err = client.try_oracle_resolve_poll(&oracle, &3_u64, &true).expect_err("already");
        assert_eq!(err, Ok(PredictXError::PollAlreadyResolved));
    }

    #[test]
    fn propose_admin_does_not_change_active_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        let candidate = Address::generate(&env);
        client.propose_admin(&admin, &candidate);
        assert_eq!(client.admin(), admin);
    }

    #[test]
    fn accept_admin_only_callable_by_candidate() {
    // ── User stats view tests ─────────────────────────────────────────────────

    /// Register and initialise a market contract for the stats view assertions.
    fn stats_env() -> (Env, Address, PredictionMarketClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        let candidate = Address::generate(&env);
        let stranger = Address::generate(&env);
        client.propose_admin(&admin, &candidate);
        let err = client
            .try_accept_admin(&stranger)
            .expect_err("stranger cannot accept");
        assert_eq!(err, Ok(PredictXError::Unauthorized));
        assert_eq!(client.admin(), admin);
    }

    #[test]
    fn proposal_can_be_overwritten_or_cancelled() {
    // ── transition_status state machine (issue #123) ──────────────────────────

    fn poll_with_status(env: &Env, status: PollStatus) -> Poll {
        Poll {
            poll_id: 1,
            match_id: 1,
            creator: Address::generate(env),
            question: String::from_str(env, "q"),
            category: PollCategory::TeamEvent,
            lock_time: 1_000,
            yes_pool: 0,
            no_pool: 0,
            yes_count: 0,
            no_count: 0,
            status,
            outcome: None,
            resolution_time: 0,
            created_at: 0,
        }
    }

    #[test]
    fn transition_accepts_every_legal_edge() {
        let env = Env::default();
        let cases = [
            (PollStatus::Active, PollStatus::Locked),
            (PollStatus::Locked, PollStatus::Voting),
            (PollStatus::Voting, PollStatus::AdminReview),
            (PollStatus::Voting, PollStatus::Disputed),
            (PollStatus::Voting, PollStatus::Resolved),
            (PollStatus::AdminReview, PollStatus::Resolved),
            (PollStatus::Disputed, PollStatus::Resolved),
            (PollStatus::Active, PollStatus::Cancelled),
            (PollStatus::Locked, PollStatus::Cancelled),
        ];
        for (from, to) in cases {
            let poll = poll_with_status(&env, from);
            assert_eq!(
                transition_status(&env, &poll, to),
                Ok(()),
                "legal edge {:?} -> {:?} rejected",
                from,
                to
            );
        }
    }

    #[test]
    fn transition_rejects_illegal_edges() {
        let env = Env::default();
        let cases = [
            (PollStatus::Active, PollStatus::Resolved),
            (PollStatus::Active, PollStatus::Voting),
            (PollStatus::Active, PollStatus::AdminReview),
            (PollStatus::Active, PollStatus::Disputed),
            (PollStatus::Locked, PollStatus::Resolved),
            (PollStatus::Locked, PollStatus::Locked),
            (PollStatus::Voting, PollStatus::Locked),
            (PollStatus::Voting, PollStatus::Cancelled),
            (PollStatus::Voting, PollStatus::Active),
            (PollStatus::AdminReview, PollStatus::Disputed),
            (PollStatus::Disputed, PollStatus::AdminReview),
        ];
        for (from, to) in cases {
            let poll = poll_with_status(&env, from);
            assert_eq!(
                transition_status(&env, &poll, to),
                Err(PredictXError::InvalidStateTransition),
                "illegal edge {:?} -> {:?} was accepted",
                from,
                to
            );
        }
    }

    #[test]
    fn resolved_and_cancelled_are_terminal() {
        let env = Env::default();
        let statuses = [
            PollStatus::Active,
            PollStatus::Locked,
            PollStatus::Voting,
            PollStatus::AdminReview,
            PollStatus::Disputed,
            PollStatus::Resolved,
            PollStatus::Cancelled,
        ];
        for terminal in [PollStatus::Resolved, PollStatus::Cancelled] {
            for to in statuses {
                let poll = poll_with_status(&env, terminal);
                assert_eq!(
                    transition_status(&env, &poll, to),
                    Err(PredictXError::InvalidStateTransition),
                    "terminal {:?} must not transition to {:?}",
                    terminal,
                    to
                );
            }
        }
    }

    #[test]
    fn resolve_poll_rejects_active_poll_with_invalid_transition() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        let first = Address::generate(&env);
        let second = Address::generate(&env);
        client.propose_admin(&admin, &first);
        client.propose_admin(&admin, &second);
        let err = client
            .try_accept_admin(&first)
            .expect_err("overwritten proposal");
        assert_eq!(err, Ok(PredictXError::Unauthorized));
        client.cancel_admin_proposal(&admin);
        let err = client
            .try_accept_admin(&second)
            .expect_err("cancelled proposal");
        assert_eq!(err, Ok(PredictXError::Unauthorized));
        assert_eq!(client.admin(), admin);
    }

    #[test]
    fn accept_admin_transfers_ownership_and_old_admin_loses_access() {
        seed_active_poll(&env, &contract_id, 4, &admin);

        // Active -> Resolved is not part of the legal graph.
        let err = client
            .try_resolve_poll(&admin, &4_u64, &true)
            .expect_err("active polls cannot be resolved directly");
        assert_eq!(err, Ok(PredictXError::InvalidStateTransition));
    }

    #[test]
    fn full_lifecycle_walk_via_state_machine() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle, &tok, &treasury, &TEST_FEE_BPS);
        let candidate = Address::generate(&env);
        client.propose_admin(&admin, &candidate);
        client.accept_admin(&candidate);
        assert_eq!(client.admin(), candidate);
        let err = client
            .try_pause(&admin)
            .expect_err("old admin should lose access");
        assert_eq!(err, Ok(PredictXError::Unauthorized));
        client.pause(&candidate);
        assert_eq!(client.is_paused(), true);
        seed_active_poll(&env, &contract_id, 9, &admin);

        force_poll_status(&env, &contract_id, 9, PollStatus::Locked);
        force_poll_status(&env, &contract_id, 9, PollStatus::Voting);
        force_poll_status(&env, &contract_id, 9, PollStatus::Disputed);

        client.resolve_poll(&admin, &9_u64, &false);
        let poll = client.get_poll(&9_u64);
        assert_eq!(poll.status, PollStatus::Resolved);
        assert_eq!(poll.outcome, Some(false));
    }

        (env, contract_id, client)
    }

    /// Write a `UserStats` record straight into contract storage.
    fn seed_user_stats(env: &Env, contract_id: &Address, user: &Address, stats: &UserStats) {
        env.as_contract(contract_id, || {
            env.storage()
                .persistent()
                .set(&DataKey::UserStats(user.clone()), stats);
        });
    }

    /// An unknown user must read back a zeroed record, not an error.
    #[test]
    fn get_user_stats_returns_zeros_for_unknown_user() {
        let (env, _contract_id, client) = stats_env();
        let stranger = Address::generate(&env);

        assert_eq!(client.get_user_stats(&stranger), empty_user_stats());
    }

    /// The stored record is exposed verbatim and the win rate is 3/4 = 75%.
    #[test]
    fn get_user_stats_exposes_stored_record_and_win_rate() {
        let (env, contract_id, client) = stats_env();
        let user = Address::generate(&env);
        seed_user_stats(
            &env,
            &contract_id,
            &user,
            &UserStats {
                total_staked: 200_000_000,
                total_won: 120_000_000,
                total_lost: 60_000_000,
                polls_participated: 4,
                polls_won: 3,
                polls_lost: 1,
                votes_cast: 2,
                voting_rewards_earned: 5_000_000,
            },
        );

        let stats = client.get_user_stats(&user);
        assert_eq!(stats.total_staked, 200_000_000);
        assert_eq!(stats.polls_participated, 4);
        assert_eq!(stats.polls_won, 3);
        assert_eq!(stats.polls_lost, 1);

        // polls_won * 10_000 / (polls_won + polls_lost) = 3 * 10_000 / 4 = 7_500 bps.
        assert_eq!(client.get_user_win_rate(&user), 7_500);

        // An unbeaten user sits at 10_000 bps (100%).
        let unbeaten = Address::generate(&env);
        seed_user_stats(
            &env,
            &contract_id,
            &unbeaten,
            &UserStats {
                total_staked: 100_000_000,
                polls_participated: 2,
                polls_won: 2,
                ..empty_user_stats()
            },
        );
        assert_eq!(client.get_user_win_rate(&unbeaten), 10_000);
    }

    /// No settled polls means no ratio yet — return 0 instead of dividing by zero.
    #[test]
    fn get_user_win_rate_is_zero_without_settled_polls() {
        let (env, contract_id, client) = stats_env();

        // Never seen before: no record at all.
        let stranger = Address::generate(&env);
        assert_eq!(client.get_user_win_rate(&stranger), 0);

        // Has staked, but none of their polls have settled yet.
        let pending = Address::generate(&env);
        seed_user_stats(
            &env,
            &contract_id,
            &pending,
            &UserStats {
                total_staked: 50_000_000,
                polls_participated: 3,
                ..empty_user_stats()
            },
        );
        assert_eq!(client.get_user_win_rate(&pending), 0);
    }
    // ── set_oracle rotation guard (issue #233) ────────────────────────────────

    /// Register and initialise a second, real voting oracle.
    fn register_oracle(env: &Env, admin: &Address) -> Address {
        let oracle_id = env.register(voting_oracle::WASM, ());
        let client = voting_oracle::Client::new(env, &oracle_id);
        client.initialize(admin);
        oracle_id
    }

    /// Create a match plus a single still-open poll, returning the poll id.
    fn create_open_poll(
        env: &Env,
        admin: &Address,
        client: &PredictionMarketClient,
        lock_time: u64,
    ) -> u64 {
        let match_id = client.create_match(
            admin,
            &String::from_str(env, "Arsenal"),
            &String::from_str(env, "Chelsea"),
            &String::from_str(env, "Premier League"),
            &String::from_str(env, "Emirates"),
            &(lock_time + 3_600),
        );
        client.create_poll(
            admin,
            &match_id,
            &String::from_str(env, "Will the home team win?"),
            &PollCategory::TeamEvent,
            &lock_time,
        )
    }

    #[test]
    fn set_oracle_rejects_incompatible_target() {
        let (env, _admin, oracle_id, _contract_id, client) = setup_emergency_env();

        // A registered but uninitialised oracle is not a usable target.
        let uninitialised = env.register(voting_oracle::WASM, ());
        let err = client
            .try_set_oracle(&uninitialised)
            .expect_err("uninitialised oracle must be rejected");
        assert_eq!(err, Ok(PredictXError::InvalidOracle));

        // A plain account address is not a contract at all.
        let not_a_contract = Address::generate(&env);
        let err = client
            .try_set_oracle(&not_a_contract)
            .expect_err("non-contract address must be rejected");
        assert_eq!(err, Ok(PredictXError::InvalidOracle));

        // Neither attempt may have changed the governing oracle.
        assert_eq!(client.oracle(), oracle_id);
    }

    #[test]
    fn set_oracle_rejects_rotation_while_polls_unresolved() {
        let (env, admin, oracle_id, _contract_id, client) = setup_emergency_env();
        env.ledger().set_timestamp(1_000_000);

        let _poll_id = create_open_poll(&env, &admin, &client, 1_500_000);
        let replacement = register_oracle(&env, &admin);

        let err = client
            .try_set_oracle(&replacement)
            .expect_err("rotation must be refused while a poll is open");
        assert_eq!(err, Ok(PredictXError::OracleRotationBlocked));
        assert_eq!(client.oracle(), oracle_id);
    }

    #[test]
    fn set_oracle_emits_oracle_changed_after_all_polls_resolve() {
        use soroban_sdk::{testutils::Events, Symbol, TryIntoVal};

        let (env, admin, previous_oracle, _contract_id, client) = setup_emergency_env();
        env.ledger().set_timestamp(1_000_000);

        let poll_id = create_open_poll(&env, &admin, &client, 1_500_000);
        // Resolving the only poll clears the guard.
        client.resolve_poll(&previous_oracle, &poll_id, &true);
        assert_eq!(client.get_poll(&poll_id).status, PollStatus::Resolved);

        let next_oracle = register_oracle(&env, &admin);
        client.set_oracle(&next_oracle);

        assert_eq!(client.oracle(), next_oracle);

        let events = env.events().all();
        let mut oracle_changed = false;
        for i in 0..events.len() {
            let (_, topics, data) = events.get(i).unwrap();
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == Symbol::new(&env, "OracleChanged") {
                let (previous, next): (Address, Address) =
                    data.try_into_val(&env).unwrap();
                assert_eq!(previous, previous_oracle);
                assert_eq!(next, next_oracle);
                oracle_changed = true;
            }
        }
        assert!(oracle_changed, "OracleChanged(previous, next) must be emitted");
    }

    #[test]
    fn emergency_eligibility_unchanged_when_rotation_rejected() {
        let (env, admin, oracle_id, _contract_id, client) = setup_emergency_env();
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);

        env.ledger().set_timestamp(1_000_000);
        let poll_id = create_open_poll(&env, &admin, &client, 1_500_000);

        // The governing oracle reports an in-flight dispute past the timeout.
        oracle_client.set_poll_status(&poll_id, &voting_oracle::PollStatus::Disputed);
        env.ledger().set_timestamp(1_000_000 + EMERGENCY_TIMEOUT_SECS + 1);
        assert!(client.check_emergency_eligible(&poll_id));

        let replacement = register_oracle(&env, &admin);
        let err = client
            .try_set_oracle(&replacement)
            .expect_err("rotation must be refused while the dispute is open");
        assert_eq!(err, Ok(PredictXError::OracleRotationBlocked));

        // The failed rotation leaves the oracle and emergency eligibility intact.
        assert_eq!(client.oracle(), oracle_id);
        assert!(client.check_emergency_eligible(&poll_id));
    fn paused_mutators_verdicts() {
        let (env, admin, oracle_id, contract_id, client) = setup_emergency_env();

        // Prepare a poll + stake that can be emergency-withdrawn.
        let poll_id: u64 = 77;
        let user = Address::generate(&env);
        let amount: i128 = 42;

        // Fund contract to allow refunds
        let token_addr: Address = env.as_contract(&contract_id, || {
            env.storage().instance().get(&DataKey::TokenAddress).unwrap()
        });
        mint_to(&env, &token_addr, &contract_id, amount);

        // Inject stake record
        let stake = Stake { user: user.clone(), poll_id, amount, side: StakeSide::Yes, claimed: false, staked_at: env.ledger().timestamp() };
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&DataKey::Stake(poll_id, user.clone()), &stake);
        });

        // Mark poll cancelled on oracle so emergency_withdraw is eligible
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);
        oracle_client.set_poll_status(&poll_id, &voting_oracle::PollStatus::Cancelled);

        // Pause contract
        client.pause(&admin);

        // Mutators that must be rejected while paused — assert individually
        let err = client.try_set_oracle(&oracle_id).expect_err("set_oracle should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_cancel_poll(&admin, &poll_id).expect_err("cancel_poll should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_create_poll(&admin, &1_u64, &String::from_str(&env, "Q"), &PollCategory::PlayerEvent, &999999_u64).expect_err("create_poll should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_stake(&user, &poll_id, &amount, &StakeSide::Yes).expect_err("stake should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_create_match(&admin, &String::from_str(&env, "A"), &String::from_str(&env, "B"), &String::from_str(&env, "L"), &String::from_str(&env, "V"), &1_600_000_u64).expect_err("create_match should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_update_match(&admin, &1_u64, &None, &None, &None, &None, &None).expect_err("update_match should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_finish_match(&admin, &1_u64).expect_err("finish_match should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_oracle_resolve_poll(&oracle_id, &poll_id, &true).expect_err("oracle resolve_poll should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_resolve_poll(&admin, &poll_id, &true).expect_err("admin resolve_poll should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        let err = client.try_claim_winnings(&user, &poll_id).expect_err("claim_winnings should be blocked");
        assert_eq!(err, Ok(PredictXError::EmergencyWithdrawNotAllowed));

        // Emergency withdraw should remain callable while paused
        let refunded = client.emergency_withdraw(&user, &poll_id);
        assert_eq!(refunded, amount);
    }

}