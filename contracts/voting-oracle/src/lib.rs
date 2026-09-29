#![no_std]

mod storage;
mod voting;

use predictx_shared::{PollStatus, PredictXError, VoteChoice, VoteTally, VOTING_WINDOW_SECS};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

/// Maximum number of admins that may be registered at once.
///
/// Keeps `list_admins` bounded so it cannot grow without limit.
pub const MAX_ADMINS: u32 = 10;
/// Maximum voters retained per poll; keeping this low bounds full-vector reads.
pub const MAX_VOTERS: u32 = 64;

#[contract]
pub struct VotingOracle;

#[contracttype]
#[derive(Clone)]
struct StoredPollStatus {
    status: PollStatus,
    updated_at: u64,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    /// Registered admins `Vec<Address>`. (Instance)
    AdminList,
    PollStatus(u64),
    /// `poll_id` → vote tally. (Temporary — only needed during the voting window)
    VoteTally(u64),
    /// `poll_id` → automatically resolved outcome.
    PollOutcome(u64),
    /// `poll_id` → persistent roster of voters who cast a vote.
    Voters(u64),
    /// `(poll_id, voter)` → `bool` — has this voter cast a vote? (Temporary)
    HasVoted(u64, Address),
    /// `(poll_id, voter)` → the choice the voter recorded. (Persistent)
    VoterChoice(u64, Address),
    /// `poll_id` → voter reward reserve (unclaimed incentive pool). (Persistent)
    RewardPool(u64),
    /// `(poll_id, voter)` → `i128` reward paid to an eligible voter. (Persistent)
    VoterReward(u64, Address),
    /// `(poll_id, voter)` → `bool` — has the voter claimed their reward? (Persistent)
    RewardClaimed(u64, Address),
}

fn get_admin(env: &Env) -> Result<Address, PredictXError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(PredictXError::NotInitialized)
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

fn is_legal_status_transition(current: PollStatus, next: PollStatus) -> bool {
    match current {
        PollStatus::Active => matches!(
            next,
            PollStatus::Locked | PollStatus::Voting | PollStatus::Cancelled
        ),
        PollStatus::Locked => matches!(next, PollStatus::Voting | PollStatus::Cancelled),
        PollStatus::Voting => matches!(
            next,
            PollStatus::AdminReview
                | PollStatus::Disputed
                | PollStatus::Resolved
                | PollStatus::Cancelled
        ),
        PollStatus::AdminReview => matches!(
            next,
            PollStatus::Disputed | PollStatus::Resolved | PollStatus::Cancelled
        ),
        PollStatus::Disputed => matches!(next, PollStatus::Resolved | PollStatus::Cancelled),
        PollStatus::Resolved | PollStatus::Cancelled => false,
    }
}

#[contractimpl]
impl VotingOracle {
    pub fn initialize(env: Env, admin: Address) -> Result<(), PredictXError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(PredictXError::AlreadyInitialized);
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);

        // Seed the multi-admin registry with the initial admin.
        let mut admins: Vec<Address> = Vec::new(&env);
        admins.push_back(admin);
        env.storage().instance().set(&DataKey::AdminList, &admins);

        Ok(())
    }

    pub fn admin(env: Env) -> Result<Address, PredictXError> {
        get_admin(&env)
    }

    /// Register `new_admin` in the multi-admin registry.
    ///
    /// Only an existing admin may call this. Returns `AdminAlreadyRegistered`
    /// if the address is already registered.
    pub fn add_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), PredictXError> {
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

    /// Placeholder oracle state setter.
    ///
    /// This exists only to validate cross-contract invocation patterns during
    /// Phase 1 scaffolding.
    pub fn set_poll_status(
        env: Env,
        poll_id: u64,
        status: PollStatus,
        outcome: Option<VoteChoice>,
    ) -> Result<(), PredictXError> {
        let admin = get_admin(&env)?;
        admin.require_auth();

        let current_status = read_poll_status(&env, poll_id);
        if !is_legal_status_transition(current_status, status) {
            return Err(PredictXError::InvalidPollStatusTransition);
        }

        if status == PollStatus::Resolved {
            let supplied_outcome = outcome.ok_or(PredictXError::OutcomeNotAvailable)?;
            let recorded_outcome: VoteChoice = env
                .storage()
                .persistent()
                .get(&DataKey::PollOutcome(poll_id))
                .ok_or(PredictXError::OutcomeNotAvailable)?;
            if supplied_outcome != recorded_outcome {
                return Err(PredictXError::InvalidOutcome);
            }
        } else if outcome.is_some() {
            return Err(PredictXError::InvalidOutcome);
        }

        let stored = StoredPollStatus {
            status,
            updated_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::PollStatus(poll_id), &stored);
        Ok(())
    }

    /// Placeholder oracle query used by `PredictionMarket`.
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

    /// Record a voter's choice on a poll.
    pub fn cast_vote(
        env: Env,
        voter: Address,
        poll_id: u64,
        choice: VoteChoice,
    ) -> Result<VoteTally, PredictXError> {
        voting::cast_vote(&env, voter, poll_id, choice)
    }

    pub fn auto_resolve(env: Env, poll_id: u64) -> Result<VoteChoice, PredictXError> {
        voting::auto_resolve(&env, poll_id)
    }

    pub fn get_poll_outcome(env: Env, poll_id: u64) -> Result<VoteChoice, PredictXError> {
        env.storage()
            .persistent()
            .get(&DataKey::PollOutcome(poll_id))
            .ok_or(PredictXError::PollNotFound)
    }

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
        voting::set_reward_pool(&env, caller, poll_id, amount)
    }

    /// Claim the caller's voter reward for `poll_id`.
    ///
    /// Only voters who backed the resolved winning outcome may claim; the pool
    /// is split evenly across those eligible voters.
    pub fn claim_reward(env: Env, voter: Address, poll_id: u64) -> Result<i128, PredictXError> {
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
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn set_and_get_status() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        let err = client
            .try_set_poll_status(&42_u64, &PollStatus::Resolved, &None)
            .expect_err("resolution without a recorded outcome must fail");
        assert_eq!(err, Ok(PredictXError::OutcomeNotAvailable));
        assert_eq!(client.get_poll_status(&42_u64), PollStatus::Active);
    }

    fn setup_with_contract_id() -> (Env, Address, Address, VotingOracleClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VotingOracle, ());
        let client = VotingOracleClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);

        (env, admin, contract_id, client)
    }

    fn seed_status(
        env: &Env,
        contract_id: &Address,
        poll_id: u64,
        status: PollStatus,
        updated_at: u64,
        outcome: Option<VoteChoice>,
    ) {
        env.as_contract(contract_id, || {
            env.storage().persistent().set(
                &DataKey::PollStatus(poll_id),
                &StoredPollStatus { status, updated_at },
            );
            if let Some(outcome) = outcome {
                env.storage()
                    .persistent()
                    .set(&DataKey::PollOutcome(poll_id), &outcome);
            }
        });
    }

    #[test]
    fn status_transition_graph_allows_each_legal_edge() {
        let (env, _admin, contract_id, client) = setup_with_contract_id();
        let legal_transitions = [
            (PollStatus::Active, PollStatus::Locked),
            (PollStatus::Active, PollStatus::Voting),
            (PollStatus::Active, PollStatus::Cancelled),
            (PollStatus::Locked, PollStatus::Voting),
            (PollStatus::Locked, PollStatus::Cancelled),
            (PollStatus::Voting, PollStatus::AdminReview),
            (PollStatus::Voting, PollStatus::Disputed),
            (PollStatus::Voting, PollStatus::Resolved),
            (PollStatus::Voting, PollStatus::Cancelled),
            (PollStatus::AdminReview, PollStatus::Disputed),
            (PollStatus::AdminReview, PollStatus::Resolved),
            (PollStatus::AdminReview, PollStatus::Cancelled),
            (PollStatus::Disputed, PollStatus::Resolved),
            (PollStatus::Disputed, PollStatus::Cancelled),
        ];

        for (index, (current, next)) in legal_transitions.iter().enumerate() {
            let poll_id = index as u64 + 10;
            let recorded_outcome = if *next == PollStatus::Resolved {
                Some(VoteChoice::Yes)
            } else {
                None
            };
            seed_status(&env, &contract_id, poll_id, *current, 123, recorded_outcome);

            client.set_poll_status(&poll_id, next, &recorded_outcome);

            assert_eq!(client.get_poll_status(&poll_id), *next);
            assert_eq!(
                client.get_poll_status_updated_at(&poll_id),
                env.ledger().timestamp()
            );
            if let Some(expected_outcome) = recorded_outcome {
                assert_eq!(client.get_poll_outcome(&poll_id), expected_outcome);
            }
        }
    }

    #[test]
    fn terminal_and_illegal_transitions_leave_storage_unchanged() {
        let (env, _admin, contract_id, client) = setup_with_contract_id();
        let all_statuses = [
            PollStatus::Active,
            PollStatus::Locked,
            PollStatus::Voting,
            PollStatus::AdminReview,
            PollStatus::Disputed,
            PollStatus::Resolved,
            PollStatus::Cancelled,
        ];

        for (state_index, current) in [PollStatus::Resolved, PollStatus::Cancelled]
            .iter()
            .enumerate()
        {
            let poll_id = state_index as u64 + 30;
            seed_status(
                &env,
                &contract_id,
                poll_id,
                *current,
                456,
                Some(VoteChoice::No),
            );
            for next in all_statuses {
                let supplied_outcome = if next == PollStatus::Resolved {
                    Some(VoteChoice::No)
                } else {
                    None
                };
                let err = client
                    .try_set_poll_status(&poll_id, &next, &supplied_outcome)
                    .expect_err("terminal statuses must not transition");
                assert_eq!(err, Ok(PredictXError::InvalidPollStatusTransition));
                assert_eq!(client.get_poll_status(&poll_id), *current);
                assert_eq!(client.get_poll_status_updated_at(&poll_id), 456);
                assert_eq!(client.get_poll_outcome(&poll_id), VoteChoice::No);
            }
        }

        let poll_id = 40_u64;
        seed_status(&env, &contract_id, poll_id, PollStatus::Voting, 789, None);
        let err = client
            .try_set_poll_status(&poll_id, &PollStatus::Active, &None)
            .expect_err("backwards transition must fail");
        assert_eq!(err, Ok(PredictXError::InvalidPollStatusTransition));
        assert_eq!(client.get_poll_status(&poll_id), PollStatus::Voting);
        assert_eq!(client.get_poll_status_updated_at(&poll_id), 789);
    }

    #[test]
    fn resolving_requires_matching_recorded_outcome() {
        let (env, _admin, contract_id, client) = setup_with_contract_id();
        let poll_id = 1_u64;
        seed_status(&env, &contract_id, poll_id, PollStatus::Voting, 123, None);

        let missing = client
            .try_set_poll_status(&poll_id, &PollStatus::Resolved, &None)
            .expect_err("resolution requires an outcome");
        assert_eq!(missing, Ok(PredictXError::OutcomeNotAvailable));
        assert_eq!(client.get_poll_status(&poll_id), PollStatus::Voting);
        assert_eq!(client.get_poll_status_updated_at(&poll_id), 123);

        seed_status(
            &env,
            &contract_id,
            poll_id,
            PollStatus::Voting,
            123,
            Some(VoteChoice::Yes),
        );
        let mismatch = client
            .try_set_poll_status(&poll_id, &PollStatus::Resolved, &Some(VoteChoice::No))
            .expect_err("the supplied outcome must match the recorded outcome");
        assert_eq!(mismatch, Ok(PredictXError::InvalidOutcome));
        assert_eq!(client.get_poll_status(&poll_id), PollStatus::Voting);
        assert_eq!(client.get_poll_status_updated_at(&poll_id), 123);
        assert_eq!(client.get_poll_outcome(&poll_id), VoteChoice::Yes);

        client.set_poll_status(&poll_id, &PollStatus::Resolved, &Some(VoteChoice::Yes));
        assert_eq!(client.get_poll_status(&poll_id), PollStatus::Resolved);
        assert_eq!(client.get_poll_outcome(&poll_id), VoteChoice::Yes);
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
        let (env, _admin, client) = setup();
        let voter = Address::generate(&env);
        client.set_poll_status(&1_u64, &PollStatus::Voting, &None);

        assert!(!client.has_voted(&1_u64, &voter));
        assert!(client.can_vote(&1_u64, &voter));

        client.cast_vote(&voter, &1_u64, &VoteChoice::Yes);

        assert!(client.has_voted(&1_u64, &voter));
        assert!(!client.can_vote(&1_u64, &voter));
    }

    #[test]
    fn can_vote_rejects_unopened_and_expired_polls() {
        let (env, _admin, client) = setup();
        let voter = Address::generate(&env);

        client.set_poll_status(&2_u64, &PollStatus::Locked, &None);
        assert!(!client.can_vote(&2_u64, &voter));

        client.set_poll_status(&3_u64, &PollStatus::Voting, &None);
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
        client.initialize(&admin);

        (env, admin, client)
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
}
