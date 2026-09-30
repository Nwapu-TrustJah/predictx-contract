use soroban_sdk::{Address, Env, String, Symbol, Vec};
use predictx_shared::{Match, PredictXError, MAX_MATCH_STRING_LENGTH};
use crate::DataKey;   // ← uses prediction-market's local DataKey, not shared one
use crate::ensure_not_paused;
use predictx_shared::{DataKey, Match, PredictXError};
use crate::DataKey;
use predictx_shared::{Match, PredictXError};
use soroban_sdk::{Address, Env, String, Symbol, Vec};
use predictx_shared::{Match, PredictXError};
use crate::{DataKey, MatchStats};
use crate::{DataKey, MatchUpdate};
use predictx_shared::{Match, PredictXError};
use soroban_sdk::{Address, Env, String, Symbol, Vec}; // ← uses prediction-market's local DataKey, not shared one
use predictx_shared::{Match, PredictXError};
use predictx_shared::DataKey;

// ── Internal helper ───────────────────────────────────────────────────────────

pub fn require_admin(env: &Env, caller: &Address) -> Result<(), PredictXError> {
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(PredictXError::NotInitialized)?;
    if *caller != admin {
        return Err(PredictXError::Unauthorized);
    }
    Ok(())
use soroban_sdk::{contracttype, Address, Env, String, Symbol};

#[derive(Clone)]
@contracttype
pub struct Match {
    pub id: u64,
    pub home_team: String,
    pub away_team: String,
    pub kickoff_time: u64,
    pub creator: Address,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
@contracttype
pub enum Error {
    InvalidLockTime = 1,
    MatchAlreadyStarted = 2,
}

pub fn create_match(
    env: &Env,
    id: u64,
    home_team: String,
    away_team: String,
    kickoff_time: u64,
) -> Result<u64, PredictXError> {
    ensure_not_paused(env)?;
    require_admin(env, &admin)?;
    creator: Address,
) -> Result<Match, Error> {
    creator.require_auth();

    // Acceptance Criteria: Require kickoff_time > env.ledger().timestamp()
    let current_timestamp = env.ledger().timestamp();
    if kickoff_time <= current_timestamp {
        return Err(Error::InvalidLockTime);
    }

    if home_team.len() == 0 || away_team.len() == 0 {
        return Err(PredictXError::EmptyTeamName);
    }

    if home_team.len() > MAX_MATCH_STRING_LENGTH
        || away_team.len() > MAX_MATCH_STRING_LENGTH
        || league.len() > MAX_MATCH_STRING_LENGTH
        || venue.len() > MAX_MATCH_STRING_LENGTH
    {
        return Err(PredictXError::MatchStringTooLong);
    }

    let match_id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextMatchId)
        .unwrap_or(1);

    let new_match = Match {
        id,
        home_team,
        away_team,
        kickoff_time,
        creator,
    };

    env.storage()
        .persistent()
        .set(&DataKey::Match(match_id), &new_match);

    let empty: Vec<u64> = Vec::new(env);
    env.storage()
        .persistent()
        .set(&DataKey::MatchPolls(match_id), &empty);

    env.storage()
        .instance()
        .set(&DataKey::NextMatchId, &(match_id + 1));

    env.events()
        .publish((Symbol::new(env, "MatchCreated"), match_id), new_match);

    Ok(match_id)
    // Storage logic would normally go here
    Ok(new_match)
}

pub fn update_match(
    env: &Env,
    admin: Address,
    match_id: u64,
    updates: MatchUpdate,
) -> Result<Match, PredictXError> {
    ensure_not_paused(env)?;
    require_admin(env, &admin)?;
    mut existing_match: Match,
    new_kickoff_time: u64,
) -> Result<Match, Error> {
    existing_match.creator.require_auth();

    let current_timestamp = env.ledger().timestamp();
    if existing_match.kickoff_time <= current_timestamp {
        return Err(Error::MatchAlreadyStarted);
    }

    if let Some(v) = home_team {
        if v.len() == 0 {
            return Err(PredictXError::EmptyTeamName);
        }
        if v.len() > MAX_MATCH_STRING_LENGTH {
            return Err(PredictXError::MatchStringTooLong);
        }
        m.home_team = v;
    }
    if let Some(v) = away_team {
        if v.len() == 0 {
            return Err(PredictXError::EmptyTeamName);
        }
        if v.len() > MAX_MATCH_STRING_LENGTH {
            return Err(PredictXError::MatchStringTooLong);
        }
        m.away_team = v;
    }
    if let Some(v) = league {
        if v.len() > MAX_MATCH_STRING_LENGTH {
            return Err(PredictXError::MatchStringTooLong);
        }
        m.league = v;
    }
    if let Some(v) = venue {
        if v.len() > MAX_MATCH_STRING_LENGTH {
            return Err(PredictXError::MatchStringTooLong);
        }
        m.home_team = v;
    }
    if let Some(v) = away_team {
        m.away_team = v;
    }
    if let Some(v) = league {
        m.league = v;
    }
    if let Some(v) = venue {
        m.venue = v;
    }
    if let Some(kt) = kickoff_time {
    if let Some(v) = updates.home_team {
        m.home_team = v;
    }
    if let Some(v) = updates.away_team {
        m.away_team = v;
    }
    if let Some(v) = updates.league {
        m.league = v;
    }
    if let Some(v) = updates.venue {
        m.venue = v;
    }
    if let Some(kt) = updates.kickoff_time {
        if kt <= now {
            return Err(PredictXError::InvalidLockTime);
        }
        m.kickoff_time = kt;
    }

    env.storage()
        .persistent()
        .set(&DataKey::Match(match_id), &m);

    env.events()
        .publish((Symbol::new(env, "MatchUpdated"), match_id), m.clone());

    Ok(m)
}

pub fn finish_match(
    env: &Env,
    admin: Address,
    match_id: u64,
) -> Result<(), PredictXError> {
    ensure_not_paused(env)?;
pub fn finish_match(env: &Env, admin: Address, match_id: u64) -> Result<(), PredictXError> {
    require_admin(env, &admin)?;

    let mut m: Match = env
        .storage()
        .persistent()
        .get(&DataKey::Match(match_id))
        .ok_or(PredictXError::MatchNotFound)?;

    m.is_finished = true;
    env.storage()
        .persistent()
        .set(&DataKey::Match(match_id), &m);

    env.events()
        .publish((Symbol::new(env, "MatchFinished"), match_id), ());

    Ok(())
}

pub fn get_match(env: &Env, match_id: u64) -> Result<Match, PredictXError> {
    env.storage()
        .persistent()
        .get(&DataKey::Match(match_id))
        .ok_or(PredictXError::MatchNotFound)
}

pub fn get_match_polls(env: &Env, match_id: u64) -> Result<Vec<u64>, PredictXError> {
    if !env.storage().persistent().has(&DataKey::Match(match_id)) {
        return Err(PredictXError::MatchNotFound);
    if new_kickoff_time <= current_timestamp {
        return Err(Error::InvalidLockTime);
    }

pub fn get_match_stats(env: &Env, match_id: u64) -> Result<MatchStats, PredictXError> {
    let poll_ids = get_match_polls(env, match_id)?;
    let mut total_staked = 0_i128;

    for poll_id in poll_ids.iter() {
        let poll: predictx_shared::Poll = env
            .storage()
            .persistent()
            .get(&DataKey::Poll(poll_id))
            .ok_or(PredictXError::PollNotFound)?;
        total_staked += poll.yes_pool + poll.no_pool;
    }

    Ok(MatchStats {
        poll_count: poll_ids.len(),
        total_staked,
        distinct_stakers: env
            .storage()
            .persistent()
            .get(&DataKey::MatchStakerCount(match_id))
            .unwrap_or(0),
    })
}

pub fn get_match_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextMatchId)
        .unwrap_or(1u64)
        .saturating_sub(1)
    existing_match.kickoff_time = new_kickoff_time;
    Ok(existing_match)
}

/// Maximum number of matches returned by a single `list_matches` call.
pub const MAX_MATCH_PAGE_SIZE: u32 = 50;

/// Returns a page of matches starting at `start` (1-based match id) with at
/// most `limit` entries. The page size is capped at `MAX_MATCH_PAGE_SIZE`.
/// Paging past the end returns an empty vector. Matches are returned in
/// ascending id order and every match is covered exactly once across pages.
pub fn list_matches(env: &Env, start: u64, limit: u32) -> Vec<Match> {
    let mut out: Vec<Match> = Vec::new(env);

    let capped = if limit > MAX_MATCH_PAGE_SIZE {
        MAX_MATCH_PAGE_SIZE
    } else {
        limit
    };

    if capped == 0 {
        return out;
    }

    let total = get_match_count(env);
    if start == 0 || start > total {
        return out;
    }

    let end = start.saturating_add(capped as u64).saturating_sub(1).min(total);
    let mut id = start;
    while id <= end {
        if let Some(m) = env
            .storage()
            .persistent()
            .get::<DataKey, Match>(&DataKey::Match(id))
        {
            out.push_back(m);
        }
        id += 1;
    }

    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    extern crate std;

    use crate::{PredictionMarket, PredictionMarketClient};
    use crate::{MatchUpdate, PredictionMarket, PredictionMarketClient};
    use predictx_shared::PredictXError;
    use soroban_sdk::{
        contract, contractimpl,
        testutils::{Address as _, Ledger},
        token, Address, Env, String,
    };

    #[contract]
    struct DummyToken;

    #[contractimpl]
    impl DummyToken {
        pub fn decimals(_env: Env) -> u32 {
            7
        }
    }
    use predictx_shared::{PollCategory, PredictXError, StakeSide, MAX_POLLS_PER_MATCH};
    use crate::{PredictionMarket, PredictionMarketClient};

    // setup now passes a dummy oracle address and token address to match the real initialize signature
    fn setup() -> (Env, Address, PredictionMarketClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let cid = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &cid);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env); // dummy — not used by match functions
        let token = Address::generate(&env); // dummy — not used by match functions
        let oracle = Address::generate(&env);   // dummy — not used by match functions
        let token = env.register(DummyToken, ());
        let treasury = Address::generate(&env); // dummy — not used by match functions
        client.initialize(&admin, &oracle, &token, &treasury, &500_u32);
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);
        (env, admin, client)
    }

    fn s(env: &Env, t: &str) -> String {
        String::from_str(env, t)
    }

    const KICKOFF: u64 = 1_003_600;

    fn default_match(env: &Env, client: &PredictionMarketClient, admin: &Address) -> u64 {
        client.create_match(
            admin,
            &s(env, "Arsenal"),
            &s(env, "Chelsea"),
            &s(env, "Premier League"),
            &s(env, "Emirates"),
            &KICKOFF,
        )
    }

    fn setup_stats() -> (Env, Address, Address, PredictionMarketClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let oracle_id = env.register(crate::voting_oracle::WASM, ());
        crate::voting_oracle::Client::new(&env, &oracle_id).initialize(&admin);
        let token_addr = env
            .register_stellar_asset_contract_v2(Address::generate(&env))
            .address();
        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        client.initialize(
            &admin,
            &oracle_id,
            &token_addr,
            &Address::generate(&env),
            &500_u32,
        );
        env.ledger().with_mut(|ledger| ledger.timestamp = 1_000_000);
        (env, admin, token_addr, client)
    }

    fn create_poll(
        env: &Env,
        client: &PredictionMarketClient,
        admin: &Address,
        match_id: u64,
    ) -> u64 {
        client.create_poll(
            admin,
            &match_id,
            &s(env, "Will the event happen?"),
            &PollCategory::PlayerEvent,
            &1_002_000,
        )
    }

    fn mint_and_stake(
        env: &Env,
        client: &PredictionMarketClient,
        token_addr: &Address,
        staker: &Address,
        poll_id: u64,
        amount: i128,
    ) {
        token::StellarAssetClient::new(env, token_addr).mint(staker, &amount);
        client.stake(staker, &poll_id, &amount, &StakeSide::Yes);
    }

    #[test]
    fn test_create_match_returns_id() {
        let (env, admin, client) = setup();
        assert_eq!(default_match(&env, &client, &admin), 1);
    }

    #[test]
    fn test_create_match_stores_correct_data() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        let m = client.get_match(&id);
        assert_eq!(m.match_id, 1);
        assert_eq!(m.home_team, s(&env, "Arsenal"));
        assert!(!m.is_finished);
    }

    #[test]
    fn test_create_match_auto_increments() {
        let (env, admin, client) = setup();
        assert_eq!(default_match(&env, &client, &admin), 1);
        assert_eq!(default_match(&env, &client, &admin), 2);
        assert_eq!(client.get_match_count(), 2);
    }

    #[test]
    fn test_create_match_rejects_past_kickoff() {
        let (env, admin, client) = setup();
        let err = client
            .try_create_match(
                &admin,
                &s(&env, "A"),
                &s(&env, "B"),
                &s(&env, "L"),
                &s(&env, "V"),
                &999_999u64,
            )
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::InvalidLockTime);
    }

    #[test]
    fn test_create_match_rejects_non_admin() {
        let (env, _, client) = setup();
        let err = client
            .try_create_match(
                &Address::generate(&env),
                &s(&env, "A"),
                &s(&env, "B"),
                &s(&env, "L"),
                &s(&env, "V"),
                &KICKOFF,
            )
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
    }

    #[test]
    fn test_create_match_emits_event() {
        use soroban_sdk::{testutils::Events, Symbol, TryIntoVal};
        let (env, admin, client) = setup();
        default_match(&env, &client, &admin);
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        let (_, topics, _) = events.get(0).unwrap();
        let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        assert_eq!(name, Symbol::new(&env, "MatchCreated"));
    }

    #[test]
    fn test_update_match_partial() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        let updated = client.update_match(
            &admin,
            &id,
            &Some(s(&env, "Liverpool")),
            &None,
            &None,
            &None,
            &None,
            &MatchUpdate {
                home_team: Some(s(&env, "Liverpool")),
                away_team: None,
                league: None,
                venue: None,
                kickoff_time: None,
            },
        );
        assert_eq!(updated.home_team, s(&env, "Liverpool"));
        assert_eq!(updated.away_team, s(&env, "Chelsea"));
    }

    #[test]
    fn test_update_match_after_kickoff_fails() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        env.ledger().with_mut(|l| l.timestamp = KICKOFF + 1);
        let err = client
            .try_update_match(&admin, &id, &Some(s(&env, "X")), &None, &None, &None, &None)
            .try_update_match(
                &admin,
                &id,
                &MatchUpdate {
                    home_team: Some(s(&env, "X")),
                    away_team: None,
                    league: None,
                    venue: None,
                    kickoff_time: None,
                },
            )
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::MatchAlreadyStarted);
    }

    #[test]
    fn test_update_nonexistent_match_fails() {
        let (_, admin, client) = setup();
        let err = client
            .try_update_match(&admin, &999u64, &None, &None, &None, &None, &None)
            .try_update_match(
                &admin,
                &999u64,
                &MatchUpdate {
                    home_team: None,
                    away_team: None,
                    league: None,
                    venue: None,
                    kickoff_time: None,
                },
            )
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::MatchNotFound);
    }

    #[test]
    fn test_update_match_rejects_non_admin() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        let err = client
            .try_update_match(
                &Address::generate(&env),
                &id,
                &Some(s(&env, "X")),
                &None,
                &None,
                &None,
                &None,
                &MatchUpdate {
                    home_team: Some(s(&env, "X")),
                    away_team: None,
                    league: None,
                    venue: None,
                    kickoff_time: None,
                },
            )
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
    }

    #[test]
    fn test_finish_match_sets_flag() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        client.finish_match(&admin, &id);
        assert!(client.get_match(&id).is_finished);
    }
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_create_match_future_accepted() {
        let env = Env::default();
        env.ledger().set_timestamp(1000);
        let creator = Address::generate(&env);

        let result = create_match(
            &env,
            1,
            String::from_str(&env, "Team A"),
            String::from_str(&env, "Team B"),
            1500, // Future
            creator,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_finish_nonexistent_match_fails() {
        let (_, admin, client) = setup();
        let err = client
            .try_finish_match(&admin, &999u64)
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::MatchNotFound);
    }

    #[test]
    fn test_finish_match_rejects_non_admin() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        let err = client
            .try_finish_match(&Address::generate(&env), &id)
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
    }

    #[test]
    fn test_get_match_not_found() {
        let (_, _, client) = setup();
        let err = client.try_get_match(&999u64).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::MatchNotFound);
    }
    fn test_create_match_past_rejected() {
        let env = Env::default();
        env.ledger().set_timestamp(1000);
        let creator = Address::generate(&env);

        let result = create_match(
            &env,
            1,
            String::from_str(&env, "Team A"),
            String::from_str(&env, "Team B"),
            900, // Past
            creator,
        );

        assert_eq!(result, Err(Error::InvalidLockTime));
    }

    #[test]
    fn test_create_match_exact_timestamp_rejected() {
        let env = Env::default();
        env.ledger().set_timestamp(1000);
        let creator = Address::generate(&env);

        let result = create_match(
            &env,
            1,
            String::from_str(&env, "Team A"),
            String::from_str(&env, "Team B"),
            1000, // Exact current timestamp
            creator,
        );

    #[test]
    fn test_get_match_stats_empty() {
        let (env, admin, client) = setup();
        let match_id = default_match(&env, &client, &admin);

        let stats = client.get_match_stats(&match_id);
        assert_eq!(stats.poll_count, 0);
        assert_eq!(stats.total_staked, 0);
        assert_eq!(stats.distinct_stakers, 0);
    }

    #[test]
    fn test_get_match_stats_aggregates_multiple_polls_and_stakers() {
        let (env, admin, token_addr, client) = setup_stats();
        let match_id = default_match(&env, &client, &admin);
        let poll_one = create_poll(&env, &client, &admin, match_id);
        let poll_two = create_poll(&env, &client, &admin, match_id);
        let poll_three = create_poll(&env, &client, &admin, match_id);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        mint_and_stake(&env, &client, &token_addr, &alice, poll_one, 10_000_000);
        mint_and_stake(&env, &client, &token_addr, &bob, poll_one, 20_000_000);
        mint_and_stake(&env, &client, &token_addr, &alice, poll_two, 30_000_000);
        mint_and_stake(&env, &client, &token_addr, &bob, poll_three, 40_000_000);

        let stats = client.get_match_stats(&match_id);
        assert_eq!(stats.poll_count, 3);
        assert_eq!(stats.total_staked, 100_000_000);
        assert_eq!(stats.distinct_stakers, 2);
    }

    #[test]
    fn test_get_match_stats_at_max_polls_counts_unique_stakers() {
        let (env, admin, token_addr, client) = setup_stats();
        let match_id = default_match(&env, &client, &admin);
        let repeated_staker = Address::generate(&env);
        let other_staker = Address::generate(&env);
        let mut first_poll = 0;
        let mut last_poll = 0;

        for index in 0..MAX_POLLS_PER_MATCH {
            let poll_id = create_poll(&env, &client, &admin, match_id);
            if index == 0 {
                first_poll = poll_id;
            }
            if index == MAX_POLLS_PER_MATCH - 1 {
                last_poll = poll_id;
            }
        }

        mint_and_stake(&env, &client, &token_addr, &repeated_staker, first_poll, 20_000_000);
        mint_and_stake(&env, &client, &token_addr, &repeated_staker, last_poll, 30_000_000);
        mint_and_stake(&env, &client, &token_addr, &other_staker, last_poll, 40_000_000);

        let stats = client.get_match_stats(&match_id);
        assert_eq!(stats.poll_count, MAX_POLLS_PER_MATCH);
        assert_eq!(stats.total_staked, 90_000_000);
        assert_eq!(stats.distinct_stakers, 2);
    }

    #[allow(unused_variables)]
    #[test]
    fn test_get_match_count_starts_zero() {
        let (_, _, client) = setup();
        assert_eq!(client.get_match_count(), 0);
    }

    #[test]
    fn test_propose_admin_does_not_change_active_admin() {
        let (env, admin, client) = setup();
        let candidate = Address::generate(&env);
        client.propose_admin(&admin, &candidate);
        // old admin still works
        default_match(&env, &client, &admin);
        // candidate cannot act yet
        let err = client
            .try_create_match(
                &candidate,
                &s(&env, "A"),
                &s(&env, "B"),
                &s(&env, "L"),
                &s(&env, "V"),
                &KICKOFF,
            )
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
    }

    #[test]
    fn test_only_proposed_candidate_can_accept() {
        let (env, admin, client) = setup();
        let candidate = Address::generate(&env);
        let stranger = Address::generate(&env);
        client.propose_admin(&admin, &candidate);
        let err = client.try_accept_admin(&stranger).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
        client.accept_admin(&candidate);
    }

    #[test]
    fn test_proposal_can_be_overwritten_or_cancelled() {
        let (env, admin, client) = setup();
        let first = Address::generate(&env);
        let second = Address::generate(&env);
        client.propose_admin(&admin, &first);
        client.propose_admin(&admin, &second);
        // first can no longer accept
        let err = client.try_accept_admin(&first).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
        // cancel removes pending
        client.cancel_admin_proposal(&admin);
        let err = client.try_accept_admin(&second).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
    }

    #[test]
    fn test_old_admin_loses_access_after_acceptance() {
        let (env, admin, client) = setup();
        let candidate = Address::generate(&env);
        client.propose_admin(&admin, &candidate);
        client.accept_admin(&candidate);
        // old admin rejected
        let err = client
            .try_create_match(
                &admin,
                &s(&env, "A"),
                &s(&env, "B"),
                &s(&env, "L"),
                &s(&env, "V"),
                &KICKOFF,
            )
            .unwrap_err()
            .unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
        // new admin works
        default_match(&env, &client, &candidate);
    fn make_string(env: &Env, len: usize) -> String {
        let s = "a".repeat(len);
        String::from_str(env, &s)
    }

    #[test]
    fn test_create_match_rejects_empty_team_name() {
        let (env, admin, client) = setup();

        // 1. Empty home team
        let err_home = client.try_create_match(
            &admin,
            &s(&env, ""),
            &s(&env, "Chelsea"),
            &s(&env, "Premier League"),
            &s(&env, "Emirates"),
            &KICKOFF,
        ).unwrap_err().unwrap();
        assert_eq!(err_home, PredictXError::EmptyTeamName);

        // 2. Empty away team
        let err_away = client.try_create_match(
            &admin,
            &s(&env, "Arsenal"),
            &s(&env, ""),
            &s(&env, "Premier League"),
            &s(&env, "Emirates"),
            &KICKOFF,
        ).unwrap_err().unwrap();
        assert_eq!(err_away, PredictXError::EmptyTeamName);

        // 3. Both teams empty
        let err_both = client.try_create_match(
            &admin,
            &s(&env, ""),
            &s(&env, ""),
            &s(&env, "Premier League"),
            &s(&env, "Emirates"),
            &KICKOFF,
        ).unwrap_err().unwrap();
        assert_eq!(err_both, PredictXError::EmptyTeamName);
    }

    #[test]
    fn test_create_match_bounds_each_field_independently() {
        let (env, admin, client) = setup();

        // 1. Home team exceeding 256 bytes
        let err_home = client.try_create_match(
            &admin,
            &make_string(&env, 257),
            &s(&env, "Chelsea"),
            &s(&env, "Premier League"),
            &s(&env, "Emirates"),
            &KICKOFF,
        ).unwrap_err().unwrap();
        assert_eq!(err_home, PredictXError::MatchStringTooLong);

        // 2. Away team exceeding 256 bytes
        let err_away = client.try_create_match(
            &admin,
            &s(&env, "Arsenal"),
            &make_string(&env, 257),
            &s(&env, "Premier League"),
            &s(&env, "Emirates"),
            &KICKOFF,
        ).unwrap_err().unwrap();
        assert_eq!(err_away, PredictXError::MatchStringTooLong);

        // 3. League exceeding 256 bytes
        let err_league = client.try_create_match(
            &admin,
            &s(&env, "Arsenal"),
            &s(&env, "Chelsea"),
            &make_string(&env, 257),
            &s(&env, "Emirates"),
            &KICKOFF,
        ).unwrap_err().unwrap();
        assert_eq!(err_league, PredictXError::MatchStringTooLong);

        // 4. Venue exceeding 256 bytes
        let err_venue = client.try_create_match(
            &admin,
            &s(&env, "Arsenal"),
            &s(&env, "Chelsea"),
            &s(&env, "Premier League"),
            &make_string(&env, 257),
            &KICKOFF,
        ).unwrap_err().unwrap();
        assert_eq!(err_venue, PredictXError::MatchStringTooLong);
    }

    #[test]
    fn test_create_match_accepts_strings_at_exact_256_byte_bound() {
        let (env, admin, client) = setup();

        let s256_home = make_string(&env, 256);
        let s256_away = make_string(&env, 256);
        let s256_league = make_string(&env, 256);
        let s256_venue = make_string(&env, 256);

        let match_id = client.create_match(
            &admin,
            &s256_home,
            &s256_away,
            &s256_league,
            &s256_venue,
            &KICKOFF,
        );
        assert_eq!(match_id, 1);

        let m = client.get_match(&match_id);
        assert_eq!(m.home_team.len(), 256);
        assert_eq!(m.away_team.len(), 256);
        assert_eq!(m.league.len(), 256);
        assert_eq!(m.venue.len(), 256);
    }

    #[test]
    fn test_update_match_validates_bounds_and_empty_teams() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);

        // Reject empty home team update
        let err_empty_home = client.try_update_match(
            &admin, &id,
            &Some(s(&env, "")), &None, &None, &None, &None,
        ).unwrap_err().unwrap();
        assert_eq!(err_empty_home, PredictXError::EmptyTeamName);

        // Reject empty away team update
        let err_empty_away = client.try_update_match(
            &admin, &id,
            &None, &Some(s(&env, "")), &None, &None, &None,
        ).unwrap_err().unwrap();
        assert_eq!(err_empty_away, PredictXError::EmptyTeamName);

        // Reject home team > 256
        let err_long_home = client.try_update_match(
            &admin, &id,
            &Some(make_string(&env, 257)), &None, &None, &None, &None,
        ).unwrap_err().unwrap();
        assert_eq!(err_long_home, PredictXError::MatchStringTooLong);

        // Reject venue > 256
        let err_long_venue = client.try_update_match(
            &admin, &id,
            &None, &None, &None, &Some(make_string(&env, 257)), &None,
        ).unwrap_err().unwrap();
        assert_eq!(err_long_venue, PredictXError::MatchStringTooLong);

        // Accept update at exactly 256 bytes
        let updated = client.update_match(
            &admin, &id,
            &Some(make_string(&env, 256)), &None, &None, &None, &None,
        );
        assert_eq!(updated.home_team.len(), 256);
    }
        assert_eq!(result, Err(Error::InvalidLockTime));
    }
}
    fn test_list_matches_pages_cover_every_match_once() {
        let (env, admin, client) = setup();
        for _ in 0..5 {
            default_match(&env, &client, &admin);
        }

        let page1 = client.list_matches(&1u64, &2u32);
        let page2 = client.list_matches(&3u64, &2u32);
        let page3 = client.list_matches(&5u64, &2u32);

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 1);

        let mut ids: Vec<u64> = Vec::new(&env);
        for m in page1.iter() { ids.push_back(m.match_id); }
        for m in page2.iter() { ids.push_back(m.match_id); }
        for m in page3.iter() { ids.push_back(m.match_id); }

        assert_eq!(ids.len(), 5);
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(id, (i as u64) + 1);
        }
    }

    #[test]
    fn test_list_matches_caps_page_size() {
        let (env, admin, client) = setup();
        for _ in 0..3 {
            default_match(&env, &client, &admin);
        }
        // Requesting more than the cap still returns at most the cap.
        let page = client.list_matches(&1u64, &10_000u32);
        assert_eq!(page.len(), 3);
        assert!(page.len() <= 50);
    }

    #[test]
    fn test_list_matches_past_end_returns_empty() {
        let (env, admin, client) = setup();
        default_match(&env, &client, &admin);
        default_match(&env, &client, &admin);

        let past = client.list_matches(&3u64, &10u32);
        assert_eq!(past.len(), 0);

        let zero_start = client.list_matches(&0u64, &10u32);
        assert_eq!(zero_start.len(), 0);

        let zero_limit = client.list_matches(&1u64, &0u32);
        assert_eq!(zero_limit.len(), 0);
    }
}