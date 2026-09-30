use predictx_factory::PollFactoryClient;
use predictx_mock_token::MockTokenClient;
use predictx_oracle::VotingOracleClient;
use predictx_shared::PollCategory;
use predictx_market::PredictionMarketClient;
use predictx_treasury::TreasuryClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Env, Address};

pub struct TestEnv {
    pub env: Env,
    pub admin: Address,
    pub token_address: Address,
    pub treasury_client: TreasuryClient<'static>,
    pub oracle_client: VotingOracleClient<'static>,
    pub market_client: PredictionMarketClient<'static>,
    pub factory_client: PollFactoryClient<'static>,
}

pub fn setup() -> TestEnv {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    // 1. Deploy mock token
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_contract.address();

    // 2. Deploy treasury and initialize
    let treasury_id = env.register(predictx_treasury::Treasury, ());
    let treasury_client = TreasuryClient::new(&env, &treasury_id);
    treasury_client.initialize(admin.clone());
    treasury_client.set_token(&admin, &token_addr);

    // 3. Deploy voting oracle and initialize
    let oracle_id = env.register(predictx_oracle::VotingOracle, ());
    let oracle_client = VotingOracleClient::new(&env, &oracle_id);
    oracle_client.initialize(admin.clone());

    // 4. Deploy prediction market and initialize
    let market_id = env.register(predictx_market::PredictionMarket, ());
    let market_client = PredictionMarketClient::new(&env, &market_id);
    market_client.initialize(
        admin.clone(),
        &oracle_id,
        &token_addr,
        &treasury_id,
        &500_u32,
    );

    // 5. Deploy poll factory and initialize
    let factory_id = env.register(predictx_factory::PollFactory, ());
    let factory_client = PollFactoryClient::new(&env, &factory_id);
    factory_client.initialize(admin.clone());

    TestEnv {
        env,
        admin,
        token_address: token_addr,
        treasury_client,
        oracle_client,
        market_client,
        factory_client,
    }
}