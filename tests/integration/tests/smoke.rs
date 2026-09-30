use poll_factory::{PollFactory, PollFactoryClient};
use predictx_shared::PollStatus;
use soroban_sdk::{testutils::Address as _, Address, Env, String};
use voting_oracle::{VotingOracle, VotingOracleClient};

#[test]
fn poll_factory_and_voting_oracle_run_together() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);

    let factory_id = env.register(PollFactory, ());
    let factory = PollFactoryClient::new(&env, &factory_id);
    factory.initialize(&admin);

    let poll_id = factory.create_poll(
        &creator,
        &String::from_str(&env, "Will the home team win?"),
        &1_000,
    );
    assert_eq!(poll_id, 1);

    let poll = factory.get_poll(&poll_id);
    assert_eq!(poll.status, PollStatus::Active);
    assert_eq!(poll.creator, creator);

    let oracle_id = env.register(VotingOracle, ());
    let oracle = VotingOracleClient::new(&env, &oracle_id);
    oracle.initialize(&admin);
    oracle.set_poll_status(&poll_id, &PollStatus::Resolved);

    assert_eq!(oracle.get_poll_status(&poll_id), PollStatus::Resolved);
    assert_eq!(factory.get_poll(&poll_id).status, PollStatus::Active);
}
