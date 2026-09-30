#![no_std]

use predictx_shared::{PredictXError, PollStatus, VoteTally};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
mod voting;

use predictx_shared::{PollStatus, PredictXError};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String};
use predictx_shared::{PollStatus, PredictXError, VoteTally};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
mod storage;
mod voting;

use predictx_shared::{Dispute, PollStatus, PredictXError, VoteChoice, VoteTally};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};
use predictx_shared::{PollStatus, PredictXError, VoteChoice, VoteTally};
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};
use storage::{get_admin, read_poll_status, read_poll_status_updated_at, DataKey, StoredPollStatus};
use predictx_shared::DataKey;
use predictx_shared::{PollStatus, PredictXError, VoteChoice, VoteTally, VOTING_WINDOW_SECS};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};
mod dispute;
mod storage;
mod voting;
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec};

use predictx_shared::{PollStatus, PredictXError, VoteChoice, VoteTally};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

mod voting;

pub mod voting;
mod storage;
mod voting;

/// Maximum number of admins that may be registered at once.
///
/// Keeps `list_admins` bounded so it cannot grow without limit.
pub const MAX_ADMINS: u32 = 10;
/// Maximum voters retained per poll; keeping this low bounds full-vector reads.
pub const MAX_VOTERS: u32 = 64;

#[contract]
pub struct VotingOracle;

#[contractevent]
pub struct AdminVerified {
    poll_id: u64,
    admin: Address,
    outcome: bool,
}

#[contracttype]
#[derive(Clone)]
pub(crate) struct StoredPollStatus {
    pub status: PollStatus,
    pub updated_at: u64,
    pub provisional_outcome: Option<bool>,
#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredPollStatus {
    status: PollStatus,
    updated_at: u64,
    outcome: Option<bool>,
    reasoning: String,
#[derive(Clone)]
pub(crate) struct StoredPollStatus {
    pub(crate) status: PollStatus,
    pub(crate) updated_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub(crate) enum DataKey {
    Admin,
    PredictionMarket,
    /// Registered admins `Vec<Address>`. (Instance)
    AdminList,
    /// Soroban token contract `Address` used for dispute fees. (Instance)
    TokenAddress,
    Paused,
    PollStatus(u64),
    Evidence(u64),
    /// `poll_id` → `VoteTally`. (Temporary — only needed during voting window)
    VoteTally(u64),
    /// `poll_id` → IPFS evidence hash `String`. (Temporary — alongside tally)
    VotingEvidence(u64),
    Tally(u64),
    /// `poll_id` → vote tally. (Temporary — only needed during the voting window)
    VoteTally(u64),
    /// `poll_id` → automatically resolved outcome.
    PollOutcome(u64),
    /// `poll_id` → persistent roster of voters who cast a vote.
    Voters(u64),
    /// `poll_id` → `Dispute`. (Persistent)
    Dispute(u64),
    /// `(poll_id, voter)` → `bool` — has this voter cast a vote? (Temporary)
    HasVoted(u64, Address),
    /// `poll_id` → `Dispute`. (Persistent)
    Dispute(u64),
    /// `(poll_id, voter)` → the choice the voter recorded. (Persistent)
    VoterChoice(u64, Address),
    /// `poll_id` → voter reward reserve (unclaimed incentive pool). (Persistent)
    RewardPool(u64),
    /// `(poll_id, voter)` → `i128` reward paid to an eligible voter. (Persistent)
    VoterReward(u64, Address),
    /// `(poll_id, voter)` → `bool` — has the voter claimed their reward? (Persistent)
    RewardClaimed(u64, Address),
    /// `(poll_id, admin)` → `VoteChoice` — outcome approved by admin. (Persistent)
    Approval(u64, Address),
    /// `poll_id` → `u32` approval count. (Persistent)
    ApprovalCount(u64),
    /// Soroban token contract used to pay out voter rewards. (Instance)
    TokenAddress,
    /// `(poll_id, voter)` → `bool` — has this voter claimed their reward? (Persistent)
    RewardClaimed(u64, Address),
    /// Soroban token contract used for payouts. (Instance)
    TokenAddress,
    /// Treasury address that receives forfeited dispute fees. (Instance)
    TreasuryAddress,
    /// `poll_id` → `Dispute`. (Persistent)
    Dispute(u64),
    /// `(poll_id, outcome)` → admin approvals recorded for resolving a
    /// disputed poll to `outcome`.
    DisputeApprovals(u64, VoteChoice),
    /// `poll_id` → open dispute against the poll's resolution.
    Dispute(u64),
}

fn get_admin(env: &Env) -> Result<Address, PredictXError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(PredictXError::NotInitialized)
}

pub(crate) fn get_token_address(env: &Env) -> Result<Address, PredictXError> {
    env.storage()
        .instance()
        .get(&DataKey::TokenAddress)
        .ok_or(PredictXError::NotInitialized)
}

fn empty_poll_status(env: &Env) -> StoredPollStatus {
    StoredPollStatus {
        status: PollStatus::Active,
        updated_at: 0,
        outcome: None,
        reasoning: String::from_str(env, ""),
    }
/// Read the stored vote tally for `poll_id`, if any.
fn read_tally(env: &Env, poll_id: u64) -> Option<VoteTally> {
    env.storage().persistent().get(&DataKey::Tally(poll_id))
}

/// Persist `tally` under its poll ID.
fn write_tally(env: &Env, tally: &VoteTally) {
    env.storage()
        .persistent()
        .set(&DataKey::Tally(tally.poll_id), tally);
fn is_paused(env: &Env) -> bool {
    env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
}

fn ensure_not_paused(env: &Env) -> Result<(), PredictXError> {
    if is_paused(env) {
        return Err(PredictXError::ContractPaused);
    }
    Ok(())
}

pub(crate) fn read_poll_status(env: &Env, poll_id: u64) -> PollStatus {
    let stored: Option<StoredPollStatus> = env
        .storage()
        .persistent()
        .get(&DataKey::PollStatus(poll_id));

    stored.map(|s| s.status).unwrap_or(PollStatus::Active)
}

pub(crate) fn read_poll_status_updated_at(env: &Env, poll_id: u64) -> u64 {
    env.storage()
        .persistent()
        .get::<DataKey, StoredPollStatus>(&DataKey::PollStatus(poll_id))
        .map(|stored| stored.updated_at)
        .unwrap_or(0)
}

#[contractimpl]
impl VotingOracle {
    pub fn initialize(env: Env, admin: Address, token: Address) -> Result<(), PredictXError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(PredictXError::AlreadyInitialized);
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::TokenAddress, &token);
        env.storage().instance().set(&DataKey::Paused, &false);

        // Seed the multi-admin registry with the initial admin.
        let mut admins: Vec<Address> = Vec::new(&env);
        admins.push_back(admin);
        env.storage().instance().set(&DataKey::AdminList, &admins);

        Ok(())
    }

    pub fn admin(env: Env) -> Result<Address, PredictXError> {
        get_admin(&env)
    }

    pub fn set_prediction_market(
        env: Env,
        prediction_market: Address,
    ) -> Result<(), PredictXError> {
        let admin = get_admin(&env)?;
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::PredictionMarket, &prediction_market);
        Ok(())
    }

    pub fn set_poll_status(env: Env, poll_id: u64, status: PollStatus) -> Result<(), PredictXError> {
    pub fn pause(env: Env, admin: Address) -> Result<(), PredictXError> {
        storage::require_admin(&env, &admin)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish((Symbol::new(&env, "ContractPaused"),), true);
        Ok(())
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), PredictXError> {
        storage::require_admin(&env, &admin)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &false);
        env.events().publish((Symbol::new(&env, "ContractUnpaused"),), true);
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        is_paused(&env)
    }

    /// Register `new_admin` in the multi-admin registry.
    ///
    /// Only an existing admin may call this. Returns `AdminAlreadyRegistered`
    /// if the address is already registered.
    pub fn add_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), PredictXError> {
        ensure_not_paused(&env)?;
        storage::require_admin(&env, &caller)?;
        caller.require_auth();

        let mut admins = storage::read_admins(&env);
        if admins.contains(new_admin.clone()) {
            return Err(PredictXError::AdminAlreadyRegistered);
        }
        if admins.len() >= MAX_ADMINS {
            return Err(PredictXError::AdminAlreadyRegistered);
        }

        admins.push_back(new_admin);
        storage::write_admins(&env, &admins);
        Ok(())
    }

    /// Remove `admin` from the multi-admin registry.
    ///
    /// Only an existing admin may call this. The last remaining admin cannot
    /// be removed.
    pub fn remove_admin(env: Env, caller: Address, admin: Address) -> Result<(), PredictXError> {
        ensure_not_paused(&env)?;
        storage::require_admin(&env, &caller)?;
        caller.require_auth();

        let admins = storage::read_admins(&env);
        if admins.len() <= 1 {
            return Err(PredictXError::Unauthorized);
        }

        let mut found = false;
        let mut updated: Vec<Address> = Vec::new(&env);
        for i in 0..admins.len() {
            let a = admins.get(i).unwrap();
            if a == admin {
                found = true;
            } else {
                updated.push_back(a);
            }
        }

        if !found {
            return Err(PredictXError::Unauthorized);
        }

        // Keep the legacy singleton (`DataKey::Admin`) and the registry in
        // agreement. If the address being removed is the singleton, migrate the
        // singleton to a remaining registry admin in the same transaction
        // rather than leaving a registry-evicted address with residual admin
        // identity (`admin()` would otherwise report an address `is_admin()`
        // denies). The `len() > 1` guard above guarantees `updated` is non-empty.
        if get_admin(&env)? == admin {
            let replacement = updated.get(0).ok_or(PredictXError::Unauthorized)?;
            env.storage().instance().set(&DataKey::Admin, &replacement);
        }

        storage::write_admins(&env, &updated);
        Ok(())
    }

    /// Returns `true` if `addr` is a registered admin.
    pub fn is_admin(env: Env, addr: Address) -> bool {
        storage::read_admins(&env).contains(addr)
    }

    /// Returns all registered admins.
    pub fn list_admins(env: Env) -> Vec<Address> {
        storage::read_admins(&env)
    }

    /// Admin-gated setter for the token contract used to pay voter rewards.
    /// Admin-gated setter for the token contract used for payouts.
    pub fn set_token_address(
        env: Env,
        admin: Address,
        token_address: Address,
    ) -> Result<(), PredictXError> {
        storage::require_admin(&env, &admin)?;
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::TokenAddress, &token_address);
        Ok(())
    }

    /// Returns the stored voter-reward token address.
    /// Returns the stored payout token address.
    pub fn get_token_address(env: Env) -> Result<Address, PredictXError> {
        env.storage()
            .instance()
            .get(&DataKey::TokenAddress)
            .ok_or(PredictXError::NotInitialized)
    }

    /// Admin-gated setter for the treasury that collects forfeited dispute fees.
    pub fn set_treasury_address(
        env: Env,
        admin: Address,
        treasury_address: Address,
    ) -> Result<(), PredictXError> {
        storage::require_admin(&env, &admin)?;
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::TreasuryAddress, &treasury_address);
        Ok(())
    }

    /// Returns the stored treasury address.
    pub fn get_treasury_address(env: Env) -> Result<Address, PredictXError> {
        env.storage()
            .instance()
            .get(&DataKey::TreasuryAddress)
            .ok_or(PredictXError::NotInitialized)
    }

    /// Placeholder oracle state setter.
    ///
    /// This exists only to validate cross-contract invocation patterns during
    /// Phase 1 scaffolding.
    ///
    /// `caller` is authorised against the multi-admin registry rather than
    /// against the owning address: the `PredictionMarket` contract mirrors its
    /// own cancellations here, and a contract call can never present the
    /// owner's signature. That means the market address must be registered
    /// with [`add_admin`] for the mirror to succeed.
    /// Set the status of `poll_id` (opens, cancels, locks or resolves it).
    ///
    /// Authorized against the multi-admin `AdminList` registry — the same
    /// authority `add_admin` / `remove_admin` use — rather than the legacy
    /// `DataKey::Admin` singleton, which `remove_admin` can evict. That keeps a
    /// single source of truth for who may drive a poll's lifecycle: every
    /// registered admin can, and a registry-evicted address cannot.
    pub fn set_poll_status(
        env: Env,
        caller: Address,
        poll_id: u64,
        status: PollStatus,
    ) -> Result<(), PredictXError> {
        ensure_not_paused(&env)?;
        let admin = get_admin(&env)?;
        admin.require_auth();
        caller.require_auth();
        storage::require_admin(&env, &caller)?;
        storage::require_admin(&env, &caller)?;
        caller.require_auth();

        let stored = StoredPollStatus {
            status,
            updated_at: env.ledger().timestamp(),
            provisional_outcome: None,
            outcome: None,
            reasoning: String::from_str(&env, ""),
        };

        env.storage().persistent().set(&DataKey::PollStatus(poll_id), &stored);
        Ok(())
    }

    pub fn admin_verify(
        env: Env,
        admin: Address,
        poll_id: u64,
        outcome: bool,
        reasoning: String,
    ) -> Result<(), PredictXError> {
        let stored_admin = get_admin(&env)?;
        if admin != stored_admin {
            return Err(PredictXError::Unauthorized);
        }
        admin.require_auth();

        let mut stored: StoredPollStatus = env
            .storage()
            .persistent()
            .get(&DataKey::PollStatus(poll_id))
            .unwrap_or_else(|| empty_poll_status(&env));

        if stored.status != PollStatus::AdminReview {
            return Err(PredictXError::ConsensusNotReached);
        }

        stored.status = PollStatus::Resolved;
        stored.updated_at = env.ledger().timestamp();
        stored.outcome = Some(outcome);
        stored.reasoning = reasoning.clone();

        env.storage().persistent().set(&DataKey::PollStatus(poll_id), &stored);

        AdminVerified {
            poll_id,
            admin: admin.clone(),
            outcome,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_poll_status(env: Env, poll_id: u64) -> PollStatus {
        read_poll_status(&env, poll_id)
    }

    pub fn get_poll_status_updated_at(env: Env, poll_id: u64) -> u64 {
        read_poll_status_updated_at(&env, poll_id)
    }

    /// Return the voters who have cast a vote on `poll_id`.
    pub fn get_voters(env: Env, poll_id: u64) -> Vec<Address> {
        storage::read_voters(&env, poll_id)
    }

    /// Returns whether `voter` has already voted on a known poll.
    pub fn has_voted(env: Env, poll_id: u64, voter: Address) -> bool {
        if !env
            .storage()
            .persistent()
            .has(&DataKey::PollStatus(poll_id))
        {
            return false;
        }

        storage::has_voted(&env, poll_id, &voter)
            || storage::read_voters(&env, poll_id).contains(voter)
    }

    /// Returns whether `voter` can cast a vote on `poll_id` right now.
    ///
    /// Unknown polls, polls outside the voting window, repeat voters, and polls
    /// at the voter limit are ineligible. Staker exclusion is handled separately.
    pub fn can_vote(env: Env, poll_id: u64, voter: Address) -> bool {
        if !env
            .storage()
            .persistent()
            .has(&DataKey::PollStatus(poll_id))
            || read_poll_status(&env, poll_id) != PollStatus::Voting
        {
            return false;
        }

        let voting_end_time = read_poll_status_updated_at(&env, poll_id)
            .checked_add(VOTING_WINDOW_SECS)
            .unwrap_or(0);
        if env.ledger().timestamp() >= voting_end_time
            || Self::has_voted(env.clone(), poll_id, voter)
            || storage::read_voters(&env, poll_id).len() >= MAX_VOTERS
        {
            return false;
        }

        true
    }

    pub fn process_tally(env: Env, tally: VoteTally) -> Result<(), PredictXError> {
        let admin = get_admin(&env)?;
        admin.require_auth();

        voting::process_tally(&env, &tally);
        Ok(())
    /// Record a voter's choice on a poll.
    pub fn cast_vote(
        env: Env,
        voter: Address,
        poll_id: u64,
        choice: VoteChoice,
    ) -> Result<VoteTally, PredictXError> {
        ensure_not_paused(&env)?;
        voting::cast_vote(&env, voter, poll_id, choice)
    }

    /// Resolve a poll once consensus is reached, reserving the voter reward
    /// pool (`total_pool * VOTER_REWARD_BPS / 10_000`) in the tally.
    pub fn auto_resolve(
        env: Env,
        poll_id: u64,
        total_pool: i128,
    ) -> Result<VoteChoice, PredictXError> {
        voting::auto_resolve(&env, poll_id, total_pool)
    pub fn auto_resolve(env: Env, poll_id: u64) -> Result<VoteChoice, PredictXError> {
        ensure_not_paused(&env)?;
        voting::auto_resolve(&env, poll_id)
    }

    pub fn get_poll_outcome(env: Env, poll_id: u64) -> Result<VoteChoice, PredictXError> {
        env.storage()
            .persistent()
            .get(&DataKey::PollOutcome(poll_id))
            .ok_or(PredictXError::PollNotFound)
    }

    /// Resolve an open dispute on a poll under admin / multi-sig control.
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        poll_id: u64,
        final_outcome: VoteChoice,
    ) -> Result<(), PredictXError> {
        voting::resolve_dispute(&env, admin, poll_id, final_outcome)
    /// Initiate a dispute against a resolved poll.
    ///
    /// The initiator must transfer the fixed dispute fee into the contract.
    /// A `Dispute` record is persisted and the poll status transitions to
    /// `Disputed`.
    /// Open a dispute against a settled poll, escrowing `dispute_fee`.
    /// Open a dispute against a resolved poll within the dispute window.
    /// Open a dispute against `poll_id`.
    ///
    /// Rejects a second dispute while an unresolved one is already open with
    /// `DisputeAlreadyOpen`. Re-disputing after a dispute is resolved is out of
    /// scope for this change (see [`voting::initiate_dispute`]).
    pub fn initiate_dispute(
        env: Env,
        initiator: Address,
        poll_id: u64,
        evidence_hash: String,
    ) -> Result<Dispute, PredictXError> {
        voting::initiate_dispute(&env, initiator, poll_id, evidence_hash)
    }

    /// Read the dispute record for a poll, if one exists.
    pub fn get_dispute(env: Env, poll_id: u64) -> Result<Dispute, PredictXError> {
        storage::read_dispute(&env, poll_id).ok_or(PredictXError::PollNotFound)
    /// Set (fund) the voter reward reserve for `poll_id`. Admin only.
    ///
    /// The policy for how large the reserve should be is deliberately out of
    /// scope here; this only records the amount that `claim_reward` divides
    /// among the eligible (winning) voters.
    pub fn set_reward_pool(
        env: Env,
        caller: Address,
        poll_id: u64,
        amount: i128,
    ) -> Result<(), PredictXError> {
        ensure_not_paused(&env)?;
        voting::set_reward_pool(&env, caller, poll_id, amount)
    }

    /// Claim the caller's voter reward for `poll_id`.
    ///
    /// Only voters who backed the resolved winning outcome may claim; the pool
    /// is split evenly across those eligible voters.
    pub fn claim_reward(env: Env, voter: Address, poll_id: u64) -> Result<i128, PredictXError> {
        ensure_not_paused(&env)?;
        voting::claim_reward(&env, voter, poll_id)
    }

    /// The choice `voter` recorded on `poll_id`, if they voted.
    pub fn get_voter_choice(env: Env, poll_id: u64, voter: Address) -> Option<VoteChoice> {
        storage::read_vote_choice(&env, poll_id, &voter)
    }

    /// The voter reward reserve set for `poll_id` (0 when unset).
    pub fn get_reward_pool(env: Env, poll_id: u64) -> i128 {
        storage::read_reward_pool(&env, poll_id)
    }

    /// Whether `voter` has already claimed their `poll_id` reward.
    pub fn has_claimed_reward(env: Env, poll_id: u64, voter: Address) -> bool {
        storage::has_claimed_reward(&env, poll_id, &voter)
    }

    /// Opens a two-hour community voting window for a finished poll.
    ///
    /// Delegates to [`voting::initiate_voting`].
    pub fn initiate_voting(
        env: Env,
        admin: Address,
        poll_id: u64,
        evidence_hash: String,
    ) -> Result<(), PredictXError> {
        voting::initiate_voting(env, admin, poll_id, evidence_hash)
    /// Read the aggregated community vote tally for `poll_id`.
    ///
    /// Returns [`PredictXError::PollNotFound`] when no tally has been recorded
    /// for the poll yet.
    pub fn get_vote_tally(env: Env, poll_id: u64) -> Result<VoteTally, PredictXError> {
        storage::read_tally(&env, poll_id).ok_or(PredictXError::PollNotFound)
    }

    pub fn get_poll_outcome(env: Env, poll_id: u64) -> Option<bool> {
        let stored: Option<StoredPollStatus> = env
            .storage()
            .persistent()
            .get(&DataKey::PollStatus(poll_id));

        stored.and_then(|s| s.outcome)
    }

    pub fn get_poll_reasoning(env: Env, poll_id: u64) -> String {
        let stored: Option<StoredPollStatus> = env
            .storage()
            .persistent()
            .get(&DataKey::PollStatus(poll_id));

        stored
            .map(|s| s.reasoning)
            .unwrap_or_else(|| String::from_str(&env, ""))
    /// Stores the evidence hash for a poll (e.g. an IPFS CID or stats-API reference).
    /// Reject empty evidence strings. This is meant to be called during `initiate_voting`
    /// or as a helper until it's merged.
    pub fn set_evidence(
        env: Env,
        poll_id: u64,
        evidence_hash: soroban_sdk::String,
    ) -> Result<(), PredictXError> {
        let admin = get_admin(&env)?;
        admin.require_auth();

        if evidence_hash.len() == 0 {
            return Err(PredictXError::InvalidEvidence);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Evidence(poll_id), &evidence_hash);
        Ok(())
    }

    /// Retrieves the evidence hash for a given poll.
    /// Returns `PollNotFound` if the evidence does not exist (meaning no vote opened or no evidence).
    pub fn get_evidence(env: Env, poll_id: u64) -> Result<soroban_sdk::String, PredictXError> {
        env.storage()
            .persistent()
            .get(&DataKey::Evidence(poll_id))
            .ok_or(PredictXError::PollNotFound)
    /// Record an admin's approval of an outcome for a contested poll.
    pub fn approve(
    /// Record an admin approval toward resolving a `Disputed` poll.
    ///
    /// The poll resolves to `outcome` once [`predictx_shared::MULTI_SIG_REQUIRED`]
    /// agreeing approvals have been recorded.
    pub fn approve_dispute(
        env: Env,
        admin: Address,
        poll_id: u64,
        outcome: VoteChoice,
    ) -> Result<PollStatus, PredictXError> {
        voting::approve_dispute(&env, admin, poll_id, outcome)
    }

    /// Resolve a `Disputed` poll once the multi-sig approval threshold is met.
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        poll_id: u64,
        outcome: VoteChoice,
    ) -> Result<(), PredictXError> {
        voting::approve(&env, admin, poll_id, outcome)
    }

    /// Read the total number of admin approvals for `poll_id`.
    pub fn get_approval_count(env: Env, poll_id: u64) -> u32 {
        storage::read_approval_count(&env, poll_id)
    }

    /// Read the outcome approved by `admin` for `poll_id`, if any.
    pub fn get_approval(env: Env, poll_id: u64, admin: Address) -> Option<VoteChoice> {
        storage::read_approval(&env, poll_id, &admin)
    /// Claim a voter's share of a resolved poll's reserved reward pool.
    ///
    /// Pull-based by design: each voter claims their own equal share instead of
    /// the contract pushing a payout to every voter at once.
    pub fn claim_voter_reward(
        env: Env,
        voter: Address,
        poll_id: u64,
    ) -> Result<i128, PredictXError> {
        voting::claim_voter_reward(&env, voter, poll_id)
    }

    /// Preview the reward `voter` could claim for `poll_id`.
    ///
    /// Returns `0` for every ineligible case instead of erroring, so the SDK
    /// can show a claimable amount before the claim is attempted.
    pub fn get_voter_reward(env: Env, poll_id: u64, voter: Address) -> i128 {
        voting::get_voter_reward(&env, poll_id, &voter)
        dispute_fee: i128,
    ) -> Result<(), PredictXError> {
        dispute::initiate_dispute(&env, initiator, poll_id, evidence_hash, dispute_fee)
    }

    /// Rule on an open dispute, refunding or forfeiting the escrowed fee.
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        poll_id: u64,
        final_outcome: VoteChoice,
    ) -> Result<(), PredictXError> {
        dispute::resolve_dispute(&env, admin, poll_id, final_outcome)
    ) -> Result<VoteChoice, PredictXError> {
        voting::resolve_dispute(&env, admin, poll_id, outcome)
    }

    /// Number of admin approvals recorded for resolving `poll_id` to `outcome`.
    pub fn get_dispute_approvals(env: Env, poll_id: u64, outcome: VoteChoice) -> u32 {
        storage::read_dispute_approvals(&env, poll_id, outcome)
    ) -> Result<(), PredictXError> {
        voting::initiate_dispute(&env, initiator, poll_id, evidence_hash)
    }

    /// Return the dispute raised against `poll_id`, if any.
    pub fn get_dispute(env: Env, poll_id: u64) -> Result<Dispute, PredictXError> {
        storage::read_dispute(&env, poll_id).ok_or(PredictXError::PollNotFound)
        dispute_fee: i128,
    ) -> Result<Dispute, PredictXError> {
        voting::initiate_dispute(&env, initiator, poll_id, evidence_hash, dispute_fee)
    }

    /// Read the dispute recorded for `poll_id`.
    ///
    /// Returns `PollNotFound` when no dispute has ever been opened for the poll.
    pub fn get_dispute(env: Env, poll_id: u64) -> Result<Dispute, PredictXError> {
        voting::get_dispute(&env, poll_id)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn setup_env() -> (soroban_sdk::Env, Address, VotingOracleClient<'static>) {
        let env = soroban_sdk::Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        // SAFETY: the Env outlives this test frame; the client borrows it.
        let client: VotingOracleClient<'static> = unsafe { core::mem::transmute(client) };
        (env, admin, client)
    }
    use super::*; 
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::testutils::{Address as _, Ledger as _, MockAuth, MockAuthInvoke};
    use soroban_sdk::testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke};
    use soroban_sdk::IntoVal;

    #[test]
    fn set_and_get_status() {
        let env = soroban_sdk::Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        client.initialize(&admin, &token);

        client.set_poll_status(&admin, &42_u64, &PollStatus::Resolved);
        assert_eq!(client.get_poll_status(&42_u64), PollStatus::Resolved);
    }

    #[test]
    fn admin_verify_resolves_review_and_persists_reasoning() {
    fn set_and_get_evidence() {
    fn paused_oracle_rejects_mutations_and_recovers_after_unpause() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);
        client.set_poll_status(&7_u64, &PollStatus::AdminReview);

        let reasoning = String::from_str(&env, "Community vote was inconclusive, but the evidence supports Yes.");
        client.admin_verify(&admin, &7_u64, &true, &reasoning);

        assert_eq!(client.get_poll_status(&7_u64), PollStatus::Resolved);
        assert_eq!(client.get_poll_outcome(&7_u64), Some(true));
        assert_eq!(client.get_poll_reasoning(&7_u64), reasoning);
    }

    #[test]
    fn admin_verify_rejects_non_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let attacker = Address::generate(&env);
        client.initialize(&admin);
        client.set_poll_status(&11_u64, &PollStatus::AdminReview);

        let reasoning = String::from_str(&env, "Not allowed");
        let err = client
            .try_admin_verify(&attacker, &11_u64, &false, &reasoning)
            .unwrap_err();

        assert_eq!(err, Err(soroban_sdk::InvokeError::Abort), "non-admin verification should be rejected");
    }

    #[test]
    fn admin_verify_rejects_non_admin_review_poll() {

        let poll_id = 1_u64;
        let evidence_hash = soroban_sdk::String::from_str(&env, "ipfs://Qm123");

        // Set evidence
        client.set_evidence(&poll_id, &evidence_hash);

        // Get evidence, should round-trip unchanged
        let retrieved = client.get_evidence(&poll_id);
        assert_eq!(retrieved, evidence_hash);
    }

    #[test]
    fn get_evidence_not_found() {
        let env = Env::default();
        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        // Get evidence for a non-existent poll should return PollNotFound
        let res = client.try_get_evidence(&99_u64);
        assert_eq!(res, Err(Ok(PredictXError::PollNotFound)));
    }

    #[test]
    fn set_evidence_empty_rejected() {
    // ── initiate_voting tests ─────────────────────────────────────────────────

    /// Happy path: tally is stored with the correct end-time and zeroed counts,
    /// and poll status transitions to `Voting`.
    #[test]
    fn test_initiate_voting_happy_path() {
        let env = soroban_sdk::Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        env.ledger().set_timestamp(1_000_000);

        client.initiate_voting(
            &admin,
            &1_u64,
            &soroban_sdk::String::from_str(&env, "ipfs://QmEvidence1"),
        );

        // Tally must exist in temporary storage with correct fields.
        let tally: predictx_shared::VoteTally = env
            .storage()
            .temporary()
            .get(&DataKey::VoteTally(1))
            .expect("VoteTally should be stored");

        assert_eq!(
            tally.voting_end_time,
            1_000_000 + predictx_shared::VOTING_WINDOW_SECS,
            "voting_end_time must be exactly now + VOTING_WINDOW_SECS"
        );
        assert_eq!(tally.yes_votes, 0);
        assert_eq!(tally.no_votes, 0);
        assert_eq!(tally.unclear_votes, 0);
        assert_eq!(tally.total_voters, 0);
        assert_eq!(tally.reward_pool, 0);

        // Poll status must be Voting.
        assert_eq!(client.get_poll_status(&1_u64), PollStatus::Voting);
    }

    /// `voting_end_time` must be exactly `start + 7200` (two hours).
    #[test]
    fn test_voting_end_time_is_exactly_two_hours() {
        let env = soroban_sdk::Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let start_ts: u64 = 5_000_000;
        env.ledger().set_timestamp(start_ts);

        client.initiate_voting(
            &admin,
            &2_u64,
            &soroban_sdk::String::from_str(&env, "ipfs://evidence2"),
        );

        let tally: predictx_shared::VoteTally = env
            .storage()
            .temporary()
            .get(&DataKey::VoteTally(2))
            .unwrap();

        assert_eq!(
            tally.voting_end_time,
            start_ts + 7_200,
            "window must be exactly 7200 seconds (2 hours)"
        );
    }

    /// A non-admin caller must be rejected with `Unauthorized`.
    #[test]
    fn test_non_admin_gets_unauthorized() {
        let env = soroban_sdk::Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        env.ledger().set_timestamp(1_000_000);

        let imposter = Address::generate(&env);

        let result = client.try_initiate_voting(
            &imposter,
            &3_u64,
            &soroban_sdk::String::from_str(&env, "fake"),
        );

        assert_eq!(
            result,
            Err(Ok(PredictXError::Unauthorized)),
            "non-admin must get Unauthorized"
        );
    }

    /// Calling `initiate_voting` twice for the same poll must return
    /// `PollAlreadyResolved` on the second call.
    #[test]
    fn test_double_initiate_gets_poll_already_resolved() {
        let env = soroban_sdk::Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        env.ledger().set_timestamp(1_000_000);

        // First call — must succeed.
        client.initiate_voting(
            &admin,
            &4_u64,
            &soroban_sdk::String::from_str(&env, "ipfs://first"),
        );

        // Second call for the same poll — must be rejected.
        let result = client.try_initiate_voting(
            &admin,
            &4_u64,
            &soroban_sdk::String::from_str(&env, "ipfs://second"),
        );

        assert_eq!(
            result,
            Err(Ok(PredictXError::PollAlreadyResolved)),
            "second initiate_voting for the same poll must get PollAlreadyResolved"
        );
    #[test]
    fn get_vote_tally_returns_poll_not_found_for_unknown_poll() {

        client.set_poll_status(&1_u64, &PollStatus::Voting);
        assert_eq!(client.get_poll_status(&1_u64), PollStatus::Voting);

        assert!(!client.is_paused());
        client.pause(&admin);
        assert!(client.is_paused());
        assert_eq!(client.get_poll_status(&1_u64), PollStatus::Voting);

        let voter = Address::generate(&env);
        let err_set_status = client
            .try_set_poll_status(&2_u64, &PollStatus::Voting)
            .expect_err("paused oracle should reject status updates");
        assert_eq!(err_set_status, Ok(PredictXError::ContractPaused));

        let err_cast_vote = client
            .try_cast_vote(&voter, &1_u64, &VoteChoice::Yes)
            .expect_err("paused oracle should reject votes");
        assert_eq!(err_cast_vote, Ok(PredictXError::ContractPaused));

        client.unpause(&admin);
        assert!(!client.is_paused());

        client.set_poll_status(&2_u64, &PollStatus::Voting);
        assert_eq!(client.get_poll_status(&2_u64), PollStatus::Voting);

        let tally = client.cast_vote(&voter, &2_u64, &VoteChoice::Yes);
        assert_eq!(tally.yes_votes, 1);
        let (_env, _admin, client) = setup();

        let err = client
            .try_get_vote_tally(&7_u64)
            .expect_err("unknown poll should error");
        assert_eq!(err, Ok(PredictXError::PollNotFound));
        client.set_poll_status(&admin, &42_u64, &PollStatus::Resolved);
        assert_eq!(client.get_poll_status(&42_u64), PollStatus::Resolved);
    }

    #[test]
    fn voting_views_return_false_for_unknown_poll() {
        let (env, _admin, client) = setup();
        let voter = Address::generate(&env);

        assert!(!client.has_voted(&99_u64, &voter));
        assert!(!client.can_vote(&99_u64, &voter));
    }

    #[test]
    fn voting_views_track_vote_and_duplicate_eligibility() {
        let (env, admin, client) = setup();
        let voter = Address::generate(&env);
        client.set_poll_status(&admin, &1_u64, &PollStatus::Voting);

        assert!(!client.has_voted(&1_u64, &voter));
        assert!(client.can_vote(&1_u64, &voter));

        client.cast_vote(&voter, &1_u64, &VoteChoice::Yes);

        assert!(client.has_voted(&1_u64, &voter));
        assert!(!client.can_vote(&1_u64, &voter));
    }

    #[test]
    fn can_vote_rejects_unopened_and_expired_polls() {
        let (env, admin, client) = setup();
        let voter = Address::generate(&env);

        client.set_poll_status(&admin, &2_u64, &PollStatus::Active);
        assert!(!client.can_vote(&2_u64, &voter));

        client.set_poll_status(&admin, &3_u64, &PollStatus::Voting);
        env.ledger()
            .with_mut(|ledger| ledger.timestamp += VOTING_WINDOW_SECS);
        assert!(!client.can_vote(&3_u64, &voter));
    }

    fn setup() -> (Env, Address, VotingOracleClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        client.initialize(&admin, &token);
        client.initialize(&admin);
        client.set_poll_status(&18_u64, &PollStatus::Resolved);

        let reasoning = String::from_str(&env, "This poll already resolved.");
        let err = client
            .try_admin_verify(&admin, &18_u64, &false, &reasoning)
            .unwrap_err();

        assert_eq!(err, Err(soroban_sdk::InvokeError::Abort), "non-admin-review polls should fail");
    }

    #[test]
    fn admin_verify_cannot_override_high_consensus_resolution() {
        let err = client
            .try_get_vote_tally(&7_u64)
            .expect_err("unknown poll should error");
        assert_eq!(err, Ok(PredictXError::PollNotFound));
        client.initialize(&admin);

        (env, admin, client)
    }

    #[test]
    fn get_vote_tally_reads_back_stored_tally_field_for_field() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        let tally = VoteTally {
            poll_id: 99,
            yes_votes: 12,
            no_votes: 5,
            unclear_votes: 3,
            total_voters: 20,
            voting_end_time: 1_700_000_000,
            reward_pool: 1_500_000,
        };

        env.as_contract(&contract_id, || {
            storage::write_tally(&env, &tally);
        });

        let stored = client.get_vote_tally(&99_u64);
        assert_eq!(stored, tally);
        assert_eq!(stored.poll_id, 99);
        assert_eq!(stored.yes_votes, 12);
        assert_eq!(stored.no_votes, 5);
        assert_eq!(stored.unclear_votes, 3);
        assert_eq!(stored.total_voters, 20);
        assert_eq!(stored.voting_end_time, 1_700_000_000);
        assert_eq!(stored.reward_pool, 1_500_000);
    }

    #[test]
    fn write_tally_overwrites_previous_tally_for_same_poll() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);
        client.set_poll_status(&24_u64, &PollStatus::Resolved);

        let reasoning = String::from_str(&env, "Attempted override of completed consensus poll.");
        let err = client
            .try_admin_verify(&admin, &24_u64, &true, &reasoning)
            .unwrap_err();

        assert_eq!(err, Err(soroban_sdk::InvokeError::Abort), "admin cannot override already-resolved consensus");

        let poll_id = 2_u64;
        let empty_evidence = soroban_sdk::String::from_str(&env, "");

        // Set empty evidence should fail with InvalidEvidence
        let res = client.try_set_evidence(&poll_id, &empty_evidence);
        assert_eq!(res, Err(Ok(PredictXError::InvalidEvidence)));
        let first = VoteTally {
            poll_id: 1,
            yes_votes: 1,
            no_votes: 0,
            unclear_votes: 0,
            total_voters: 1,
            voting_end_time: 100,
            reward_pool: 10,
        };
        let second = VoteTally {
            poll_id: 1,
            yes_votes: 4,
            no_votes: 2,
            unclear_votes: 1,
            total_voters: 7,
            voting_end_time: 200,
            reward_pool: 70,
        };

        env.as_contract(&contract_id, || {
            storage::write_tally(&env, &first);
            storage::write_tally(&env, &second);
        });

        assert_eq!(client.get_vote_tally(&1_u64), second);
    }

    #[test]
    fn initialize_seeds_admin_registry() {
        let (env, admin, client) = setup();

        assert!(client.is_admin(&admin));

        let mut expected: Vec<Address> = Vec::new(&env);
        expected.push_back(admin);
        assert_eq!(client.list_admins(), expected);
    }

    #[test]
    fn add_admin_registers_new_admin() {
        let (env, admin, client) = setup();
        let new_admin = Address::generate(&env);

        client.add_admin(&admin, &new_admin);

        assert!(client.is_admin(&new_admin));
        assert_eq!(client.list_admins().len(), 2);
    }

    #[test]
    fn add_admin_rejects_existing_admin() {
        let (_env, admin, client) = setup();

        let err = client
            .try_add_admin(&admin, &admin)
            .expect_err("re-adding an existing admin must fail");

        assert_eq!(err, Ok(PredictXError::AdminAlreadyRegistered));
    }

    #[test]
    fn add_admin_rejects_non_admin_caller() {
        let (env, _admin, client) = setup();
        let stranger = Address::generate(&env);
        let new_admin = Address::generate(&env);

        let err = client
            .try_add_admin(&stranger, &new_admin)
            .expect_err("non-admin caller must be rejected");

        assert_eq!(err, Ok(PredictXError::Unauthorized));
        assert!(!client.is_admin(&new_admin));
    }

    #[test]
    fn remove_admin_removes_registered_admin() {
        let (env, admin, client) = setup();
        let second = Address::generate(&env);
        client.add_admin(&admin, &second);

        client.remove_admin(&admin, &second);

        assert!(!client.is_admin(&second));
        assert_eq!(client.list_admins().len(), 1);
    }

    #[test]
    fn remove_admin_rejects_last_remaining_admin() {
        let (_env, admin, client) = setup();

        let err = client
            .try_remove_admin(&admin, &admin)
            .expect_err("the last remaining admin cannot be removed");

        assert_eq!(err, Ok(PredictXError::Unauthorized));
        assert!(client.is_admin(&admin));
    }

    #[test]
    fn add_admin_and_cast_vote_enforce_auth() {
        let env = Env::default();
        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client
            .mock_auths(&[MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "initialize",
                    args: (&admin,).into_val(&env),
                    sub_invokes: &[],
                },
            }])
            .initialize(&admin);

        let new_admin = Address::generate(&env);

        // add_admin without caller auth must fail
        let err = client
            .mock_auths(&[])
            .try_add_admin(&admin, &new_admin);
        assert!(err.is_err(), "add_admin without caller auth must fail");

        // add_admin with targeted admin auth succeeds
        client
            .mock_auths(&[MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "add_admin",
                    args: (&admin, &new_admin).into_val(&env),
                    sub_invokes: &[],
                },
            }])
            .add_admin(&admin, &new_admin);

        assert!(client.is_admin(&new_admin));

        // Open voting window with admin auth
        client
            .mock_auths(&[MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "set_poll_status",
                    args: (10_u64, PollStatus::Voting).into_val(&env),
                    sub_invokes: &[],
                },
            }])
            .set_poll_status(&10_u64, &PollStatus::Voting);

        let voter = Address::generate(&env);

        // cast_vote without voter auth must fail
        let err = client
            .mock_auths(&[])
            .try_cast_vote(&voter, &10_u64, &VoteChoice::Yes);
        assert!(err.is_err(), "cast_vote without voter auth must fail");

        // cast_vote with targeted voter auth succeeds
        client
            .mock_auths(&[MockAuth {
                address: &voter,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "cast_vote",
                    args: (&voter, 10_u64, VoteChoice::Yes).into_val(&env),
                    sub_invokes: &[],
                },
            }])
            .cast_vote(&voter, &10_u64, &VoteChoice::Yes);

        assert!(client.has_voted(&10_u64, &voter));
    fn registry_admin_can_set_poll_status_without_being_the_singleton() {
        let (env, admin, client) = setup();
        let contract = client.address.clone();

        // A second registry admin that is NOT the singleton.
        let second = Address::generate(&env);
        client.add_admin(&admin, &second);
        assert_ne!(second, admin);

        // Only `second` authorizes this call — no blanket `mock_all_auths`.
        env.mock_auths(&[MockAuth {
            address: &second,
            invoke: &MockAuthInvoke {
                contract: &contract,
                fn_name: "set_poll_status",
                args: (second.clone(), 9_u64, PollStatus::Resolved).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        client.set_poll_status(&second, &9_u64, &PollStatus::Resolved);
        assert_eq!(client.get_poll_status(&9_u64), PollStatus::Resolved);
    }

    #[test]
    fn removed_singleton_cannot_set_poll_status_and_views_agree() {
        let (env, original, client) = setup();
        let contract = client.address.clone();

        let second = Address::generate(&env);
        client.add_admin(&original, &second);

        // `second` evicts the original singleton from the registry.
        client.remove_admin(&second, &original);
        assert!(!client.is_admin(&original));

        // The singleton view must still name a registered admin — the two
        // authorities can never disagree after a removal.
        assert!(client.is_admin(&client.admin()));

        env.mock_auths(&[MockAuth {
            address: &original,
            invoke: &MockAuthInvoke {
                contract: &contract,
                fn_name: "set_poll_status",
                args: (original.clone(), 1_u64, PollStatus::Resolved).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        let err = client
            .try_set_poll_status(&original, &1_u64, &PollStatus::Resolved)
            .expect_err("a registry-evicted address must not set poll status");
        assert_eq!(err, Ok(PredictXError::Unauthorized));
    }
}
