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
    creator: Address,
) -> Result<Match, Error> {
    creator.require_auth();

    // Acceptance Criteria: Require kickoff_time > env.ledger().timestamp()
    let current_timestamp = env.ledger().timestamp();
    if kickoff_time <= current_timestamp {
        return Err(Error::InvalidLockTime);
    }

    let new_match = Match {
        id,
        home_team,
        away_team,
        kickoff_time,
        creator,
    };

    // Storage logic would normally go here
    Ok(new_match)
}

pub fn update_match(
    env: &Env,
    mut existing_match: Match,
    new_kickoff_time: u64,
) -> Result<Match, Error> {
    existing_match.creator.require_auth();

    let current_timestamp = env.ledger().timestamp();
    if existing_match.kickoff_time <= current_timestamp {
        return Err(Error::MatchAlreadyStarted);
    }

    if new_kickoff_time <= current_timestamp {
        return Err(Error::InvalidLockTime);
    }

    existing_match.kickoff_time = new_kickoff_time;
    Ok(existing_match)
}

#[cfg(test)]
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

        assert_eq!(result, Err(Error::InvalidLockTime));
    }
}
