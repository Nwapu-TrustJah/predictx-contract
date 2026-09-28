use soroban_sdk::{Address, Env, String, Symbol, Vec};
use predictx_shared::{Match, PredictXError};
use crate::{DataKey, MatchStats};

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
}

// ── Match functions ───────────────────────────────────────────────────────────

pub fn create_match(
    env: &Env,
    admin: Address,
    home_team: String,
    away_team: String,
    league: String,
    venue: String,
    kickoff_time: u64,
) -> Result<u64, PredictXError> {
    require_admin(env, &admin)?;

    let now = env.ledger().timestamp();
    if kickoff_time <= now {
        return Err(PredictXError::InvalidLockTime);
    }

    let match_id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextMatchId)
        .unwrap_or(1);

    let new_match = Match {
        match_id,
        home_team,
        away_team,
        league,
        venue,
        kickoff_time,
        created_by: admin,
        is_finished: false,
    };

    env.storage().persistent().set(&DataKey::Match(match_id), &new_match);

    let empty: Vec<u64> = Vec::new(env);
    env.storage().persistent().set(&DataKey::MatchPolls(match_id), &empty);

    env.storage().instance().set(&DataKey::NextMatchId, &(match_id + 1));

    env.events().publish(
        (Symbol::new(env, "MatchCreated"), match_id),
        new_match,
    );

    Ok(match_id)
}

pub fn update_match(
    env: &Env,
    admin: Address,
    match_id: u64,
    home_team: Option<String>,
    away_team: Option<String>,
    league: Option<String>,
    venue: Option<String>,
    kickoff_time: Option<u64>,
) -> Result<Match, PredictXError> {
    require_admin(env, &admin)?;

    let mut m: Match = env
        .storage()
        .persistent()
        .get(&DataKey::Match(match_id))
        .ok_or(PredictXError::MatchNotFound)?;

    let now = env.ledger().timestamp();
    if now >= m.kickoff_time {
        return Err(PredictXError::MatchAlreadyStarted);
    }

    if let Some(v) = home_team  { m.home_team = v; }
    if let Some(v) = away_team  { m.away_team = v; }
    if let Some(v) = league     { m.league    = v; }
    if let Some(v) = venue      { m.venue     = v; }
    if let Some(kt) = kickoff_time {
        if kt <= now { return Err(PredictXError::InvalidLockTime); }
        m.kickoff_time = kt;
    }

    env.storage().persistent().set(&DataKey::Match(match_id), &m);

    env.events().publish(
        (Symbol::new(env, "MatchUpdated"), match_id),
        m.clone(),
    );

    Ok(m)
}

pub fn finish_match(
    env: &Env,
    admin: Address,
    match_id: u64,
) -> Result<(), PredictXError> {
    require_admin(env, &admin)?;

    let mut m: Match = env
        .storage()
        .persistent()
        .get(&DataKey::Match(match_id))
        .ok_or(PredictXError::MatchNotFound)?;

    m.is_finished = true;
    env.storage().persistent().set(&DataKey::Match(match_id), &m);

    env.events().publish(
        (Symbol::new(env, "MatchFinished"), match_id),
        (),
    );

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
    }
    Ok(env
        .storage()
        .persistent()
        .get(&DataKey::MatchPolls(match_id))
        .unwrap_or(Vec::new(env)))
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
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    extern crate std;

    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env, String,
    };
    use predictx_shared::{PollCategory, PredictXError, StakeSide, MAX_POLLS_PER_MATCH};
    use crate::{PredictionMarket, PredictionMarketClient};

    // setup now passes a dummy oracle address and token address to match the real initialize signature
    fn setup() -> (Env, Address, PredictionMarketClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let cid = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &cid);
        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);   // dummy — not used by match functions
        let token = Address::generate(&env);    // dummy — not used by match functions
        let treasury = Address::generate(&env); // dummy — not used by match functions
        client.initialize(&admin, &oracle, &token, &treasury, &500_u32);
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);
        (env, admin, client)
    }

    fn s(env: &Env, t: &str) -> String { String::from_str(env, t) }

    const KICKOFF: u64 = 1_003_600;

    fn default_match(env: &Env, client: &PredictionMarketClient, admin: &Address) -> u64 {
        client.create_match(
            admin,
            &s(env, "Arsenal"), &s(env, "Chelsea"),
            &s(env, "Premier League"), &s(env, "Emirates"),
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
        let err = client.try_create_match(
            &admin,
            &s(&env, "A"), &s(&env, "B"),
            &s(&env, "L"), &s(&env, "V"),
            &999_999u64,
        ).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::InvalidLockTime);
    }

    #[test]
    fn test_create_match_rejects_non_admin() {
        let (env, _, client) = setup();
        let err = client.try_create_match(
            &Address::generate(&env),
            &s(&env, "A"), &s(&env, "B"),
            &s(&env, "L"), &s(&env, "V"),
            &KICKOFF,
        ).unwrap_err().unwrap();
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
            &admin, &id,
            &Some(s(&env, "Liverpool")), &None, &None, &None, &None,
        );
        assert_eq!(updated.home_team, s(&env, "Liverpool"));
        assert_eq!(updated.away_team, s(&env, "Chelsea"));
    }

    #[test]
    fn test_update_match_after_kickoff_fails() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        env.ledger().with_mut(|l| l.timestamp = KICKOFF + 1);
        let err = client.try_update_match(
            &admin, &id,
            &Some(s(&env, "X")), &None, &None, &None, &None,
        ).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::MatchAlreadyStarted);
    }

    #[test]
    fn test_update_nonexistent_match_fails() {
        let (_, admin, client) = setup();
        let err = client.try_update_match(
            &admin, &999u64,
            &None, &None, &None, &None, &None,
        ).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::MatchNotFound);
    }

    #[test]
    fn test_update_match_rejects_non_admin() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        let err = client.try_update_match(
            &Address::generate(&env), &id,
            &Some(s(&env, "X")), &None, &None, &None, &None,
        ).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
    }

    #[test]
    fn test_finish_match_sets_flag() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        client.finish_match(&admin, &id);
        assert!(client.get_match(&id).is_finished);
    }

    #[test]
    fn test_finish_match_emits_event() {
        use soroban_sdk::{testutils::Events, Symbol, TryIntoVal};
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        client.finish_match(&admin, &id);
        let events = env.events().all();
        assert_eq!(events.len(), 2);
        let (_, topics, _) = events.get(1).unwrap();
        let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        assert_eq!(name, Symbol::new(&env, "MatchFinished"));
    }

    #[test]
    fn test_finish_nonexistent_match_fails() {
        let (_, admin, client) = setup();
        let err = client.try_finish_match(&admin, &999u64).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::MatchNotFound);
    }

    #[test]
    fn test_finish_match_rejects_non_admin() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        let err = client.try_finish_match(&Address::generate(&env), &id).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::Unauthorized);
    }

    #[test]
    fn test_get_match_not_found() {
        let (_, _, client) = setup();
        let err = client.try_get_match(&999u64).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::MatchNotFound);
    }

    #[test]
    fn test_get_match_polls_empty_on_creation() {
        let (env, admin, client) = setup();
        let id = default_match(&env, &client, &admin);
        assert_eq!(client.get_match_polls(&id).len(), 0);
    }

    #[test]
    fn test_get_match_polls_nonexistent_fails() {
        let (_, _, client) = setup();
        let err = client.try_get_match_polls(&999u64).unwrap_err().unwrap();
        assert_eq!(err, PredictXError::MatchNotFound);
    }

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
}