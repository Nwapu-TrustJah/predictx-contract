#![no_std]

mod matches;
mod payouts;
mod staking;
pub(crate) mod token_utils;

use predictx_shared::{
    Match, PlatformStats, Poll, PollCategory, PollStatus, PredictXError, Stake, StakeSide,
    MAX_POLLS_PER_MATCH,
};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol, Vec};

mod voting_oracle {
    soroban_sdk::contractimport!(file = "wasm/voting_oracle.wasm");
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
    VotingOracle,
    Paused,
    TokenAddress,
    TreasuryAddress,
    PlatformFeeBps,
    Stake(u64, Address),
    EmergencyClaimed(u64, Address),
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
        let stored_admin = get_admin(&env)?;
        if admin != stored_admin { return Err(PredictXError::Unauthorized); }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish((Symbol::new(&env, "ContractPaused"),), true);
        Ok(())
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), PredictXError> {
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
        ensure_not_paused(&env)?;
        let stored_admin = get_admin(&env)?;
        if admin != stored_admin { return Err(PredictXError::Unauthorized); }
        admin.require_auth();
        let oracle_id = get_oracle(&env)?;
        let client = voting_oracle::Client::new(&env, &oracle_id);
        client.set_poll_status(&poll_id, &voting_oracle::PollStatus::Cancelled);
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
        env: Env,
        caller: Address,
        poll_id: u64,
        outcome: bool,
    ) -> Result<(), PredictXError> {
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
        staking::stake(&env, staker, poll_id, amount, side)
    }

    pub fn get_stake_info(env: Env, poll_id: u64, user: Address) -> Result<Stake, PredictXError> {
        staking::get_stake_info(&env, poll_id, &user)
    }

    pub fn get_user_stakes(env: Env, user: Address) -> Vec<u64> {
        staking::get_user_stakes(&env, &user)
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

    pub fn get_platform_stats(env: Env) -> PlatformStats {
        get_platform_stats(&env)
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
        matches::create_match(&env, admin, home_team, away_team, league, venue, kickoff_time)
    }

    pub fn update_match(
        env: Env, admin: Address, match_id: u64,
        home_team: Option<String>, away_team: Option<String>,
        league: Option<String>, venue: Option<String>,
        kickoff_time: Option<u64>,
    ) -> Result<Match, PredictXError> {
        matches::update_match(&env, admin, match_id, home_team, away_team, league, venue, kickoff_time)
    }

    pub fn finish_match(env: Env, admin: Address, match_id: u64) -> Result<(), PredictXError> {
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
        payouts::claim_winnings(&env, claimant, poll_id)
    }

    pub fn calculate_winnings(
        env: Env,
        poll_id: u64,
        user: Address,
    ) -> Result<i128, PredictXError> {
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
        oracle_client.initialize(&admin);
        oracle_client.set_poll_status(&7_u64, &voting_oracle::PollStatus::Resolved);
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
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
        oracle_client.initialize(&admin);
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let tok = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle_id, &tok, &treasury, &TEST_FEE_BPS);
        client.cancel_poll(&admin, &1_u64);
        assert_eq!(oracle_client.get_poll_status(&1_u64), voting_oracle::PollStatus::Cancelled);
    }

    // Helper to set up a real-token environment for emergency withdrawal tests
    fn setup_emergency_env() -> (Env, Address, Address, Address, PredictionMarketClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        let oracle_id = env.register(voting_oracle::WASM, ());
        let oracle_client = voting_oracle::Client::new(&env, &oracle_id);
        oracle_client.initialize(&admin);

        // Real token for transfers
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_addr = token_contract.address();
        let treasury = Address::generate(&env);

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
        let err = client.try_resolve_poll(&stranger, &1_u64, &true).expect_err("non-oracle");
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
        let err = client.try_resolve_poll(&oracle, &99_u64, &false).expect_err("missing");
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
        client.resolve_poll(&oracle, &7_u64, &true);
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
        client.resolve_poll(&oracle, &3_u64, &false);
        let err = client.try_resolve_poll(&oracle, &3_u64, &true).expect_err("already");
        assert_eq!(err, Ok(PredictXError::PollAlreadyResolved));
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
    }

}