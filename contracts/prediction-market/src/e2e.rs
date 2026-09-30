//! End-to-end test for the full user-facing flow (issue #160).
//!
//! Every other test in this workspace exercises a single contract in
//! isolation; this is the only one that walks the path a user actually takes,
//! across three deployed contracts (prediction market + voting oracle + a real
//! Stellar asset contract for the token):
//!
//! ```text
//! create match → create poll → stake from several accounts on both sides
//!   → lock → resolve → every winner claims → assert final balances
//! ```
//!
//! ## Fixture
//!
//! The fixture is the product spec's worked example (`predictx-spec.md`,
//! "Simple Example"): Chelsea vs Manchester United, "Will Palmer score a
//! goal?", with a 450-token Yes pool and a 300-token No pool. Working in
//! whole tokens (this asset has 7 decimals, so 1 token = 10_000_000 base
//! units) gives numbers that can be checked by hand:
//!
//! | staker | side | stake (tokens) | stake (base units) |
//! |--------|------|----------------|--------------------|
//! | Alice  | Yes  | 100            | 1_000_000_000      |
//! | Bob    | Yes  | 200            | 2_000_000_000      |
//! | Carol  | Yes  | 150            | 1_500_000_000      |
//! | Dave   | No   | 250            | 2_500_000_000      |
//! | Eve    | No   | 50             | 500_000_000        |
//!
//! Yes pool = 450 tokens, No pool = 300 tokens, total pot = 750 tokens.
//!
//! ## Hand-computed payouts
//!
//! `claim_winnings` uses integer arithmetic:
//!
//! ```text
//! gross = stake * total_pool / winning_pool          (rounds down)
//! net   = gross * (10_000 - fee_bps) / 10_000        (rounds down)
//! fee   = gross - net                                (sent to the treasury)
//! ```
//!
//! with `total_pool = 7_500_000_000`, `winning_pool = 4_500_000_000` and
//! `fee_bps = 500` (5%):
//!
//! ```text
//! Alice: gross = 1_000_000_000 * 7_500_000_000 / 4_500_000_000 = 1_666_666_666 (floor)
//!        net   = 1_666_666_666 * 9_500 / 10_000               = 1_583_333_332 (floor)
//!        fee   = 1_666_666_666 - 1_583_333_332                 =    83_333_334
//!
//! Bob:   gross = 2_000_000_000 * 7_500_000_000 / 4_500_000_000 = 3_333_333_333 (floor)
//!        net   = 3_333_333_333 * 9_500 / 10_000               = 3_166_666_666 (floor)
//!        fee   = 3_333_333_333 - 3_166_666_666                 =   166_666_667
//!
//! Carol: gross = 1_500_000_000 * 7_500_000_000 / 4_500_000_000 = 2_500_000_000 (exact)
//!        net   = 2_500_000_000 * 9_500 / 10_000               = 2_375_000_000
//!        fee   = 2_500_000_000 - 2_375_000_000                 =   125_000_000
//! ```
//!
//! Totals:
//!
//! ```text
//! winners  = 1_583_333_332 + 3_166_666_666 + 2_375_000_000 = 7_124_999_998
//! treasury =    83_333_334 +   166_666_667 +   125_000_000 =   375_000_001
//!                                                             ─────────────
//!                                                   out     = 7_499_999_999
//! contract in (750 tokens)                                 = 7_500_000_000
//! leftover (documented dust, 1 stroop = 1e-7 tokens)       =             1
//! ```
//!
//! The 1 base unit of dust is the unavoidable remainder of the per-claim
//! floor division: the winners' `gross` values are rounded down while each
//! `fee = gross - net` is therefore rounded up, so the three claims together
//! push one stroop of value from the winners' side into the fee side.

use crate::{voting_oracle, PredictionMarket, PredictionMarketClient};
use predictx_shared::{PollCategory, PollStatus, PredictXError, StakeSide};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env, String};

/// The market's platform fee for this fixture (5%).
const FEE_BPS: u32 = 500;

/// This asset has 7 decimals: `1 token = 10_000_000 base units`.
const ONE_TOKEN: i128 = 10_000_000;

#[test]
fn end_to_end_create_stake_resolve_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    // ── Deploy the real voting oracle ────────────────────────────────────────
    // The market only accepts resolutions from its registered oracle, so this
    // has to be an actual contract rather than a bare address.
    let oracle_id = env.register(voting_oracle::WASM, ());
    let oracle_client = voting_oracle::Client::new(&env, &oracle_id);
    oracle_client.initialize(&admin);

    // ── Deploy a real Stellar asset contract ─────────────────────────────────
    // No mocked transfers anywhere: staking and payouts move genuine SAC
    // balances.
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_contract.address();
    let sac = token::StellarAssetClient::new(&env, &token_addr);
    let tok = token::Client::new(&env, &token_addr);

    // The treasury is a plain address: payouts send the fee straight there.
    let treasury = Address::generate(&env);

    // ── Deploy the prediction market ─────────────────────────────────────────
    let contract_id = env.register(PredictionMarket, ());
    let client = PredictionMarketClient::new(&env, &contract_id);
    client.initialize(&admin, &oracle_id, &token_addr, &treasury, &FEE_BPS);

    env.ledger().set_timestamp(1_000_000);

    // ── 1. Create match ──────────────────────────────────────────────────────
    let match_id = client.create_match(
        &admin,
        &String::from_str(&env, "Manchester United"),
        &String::from_str(&env, "Chelsea"),
        &String::from_str(&env, "Premier League"),
        &String::from_str(&env, "Old Trafford"),
        &1_007_200, // kick-off
    );
    assert_eq!(match_id, 1);

    // ── 2. Create poll ───────────────────────────────────────────────────────
    let lock_time = 1_003_600;
    let poll_id = client.create_poll(
        &admin,
        &match_id,
        &String::from_str(&env, "Will Palmer score a goal?"),
        &PollCategory::PlayerEvent,
        &lock_time,
    );
    assert_eq!(poll_id, 1);

    // ── 3. Stake from several accounts on both sides ─────────────────────────
    let alice = Address::generate(&env); // 100 tokens, Yes
    let bob = Address::generate(&env); //   200 tokens, Yes
    let carol = Address::generate(&env); // 150 tokens, Yes
    let dave = Address::generate(&env); //  250 tokens, No
    let eve = Address::generate(&env); //    50 tokens, No

    // Each staker is minted exactly their stake, so afterwards their wallet is
    // empty until they claim.
    let stake = |user: &Address, amount: i128, side: StakeSide| {
        sac.mint(user, &amount);
        client.stake(user, &poll_id, &amount, &side);
    };

    stake(&alice, 100 * ONE_TOKEN, StakeSide::Yes); // 1_000_000_000
    stake(&bob, 200 * ONE_TOKEN, StakeSide::Yes); //   2_000_000_000
    stake(&carol, 150 * ONE_TOKEN, StakeSide::Yes); // 1_500_000_000
    stake(&dave, 250 * ONE_TOKEN, StakeSide::No); //   2_500_000_000
    stake(&eve, 50 * ONE_TOKEN, StakeSide::No); //       500_000_000

    // Pools match the spec fixture exactly: Yes 450, No 300.
    let pool = client.get_pool_info(&poll_id);
    assert_eq!(pool.yes_pool, 450 * ONE_TOKEN, "Yes pool = 450 tokens");
    assert_eq!(pool.no_pool, 300 * ONE_TOKEN, "No pool = 300 tokens");
    assert_eq!(pool.yes_count, 3);
    assert_eq!(pool.no_count, 2);

    // The market holds the whole 750-token pot; every staker is spent out.
    assert_eq!(client.get_contract_balance(), 750 * ONE_TOKEN);
    for staker in [&alice, &bob, &carol, &dave, &eve] {
        assert_eq!(tok.balance(staker), 0);
    }

    // ── 4. Lock ──────────────────────────────────────────────────────────────
    env.ledger().set_timestamp(lock_time + 1);

    // Staking after lock time is rejected: the pot is sealed.
    let latecomer = Address::generate(&env);
    sac.mint(&latecomer, &(10 * ONE_TOKEN));
    assert_eq!(
        client
            .try_stake(&latecomer, &poll_id, &(10 * ONE_TOKEN), &StakeSide::Yes)
            .unwrap_err()
            .unwrap(),
        PredictXError::PollLocked,
    );
    assert_eq!(tok.balance(&latecomer), 10 * ONE_TOKEN, "no late stake taken");

    // ── 5. Resolve ───────────────────────────────────────────────────────────
    // The registered oracle reports the outcome (Yes — Palmer scored).
    client.resolve_poll(&oracle_id, &poll_id, &true);
    assert_eq!(client.get_poll(&poll_id).status, PollStatus::Resolved);

    // ── 6. Each winner claims ────────────────────────────────────────────────
    let alice_payout = client.claim_winnings(&alice, &poll_id);
    let bob_payout = client.claim_winnings(&bob, &poll_id);
    let carol_payout = client.claim_winnings(&carol, &poll_id);

    // Values computed in the module docs above.
    assert_eq!(alice_payout, 1_583_333_332);
    assert_eq!(bob_payout, 3_166_666_666);
    assert_eq!(carol_payout, 2_375_000_000);

    // Losing side cannot claim.
    assert_eq!(
        client
            .try_claim_winnings(&dave, &poll_id)
            .unwrap_err()
            .unwrap(),
        PredictXError::NotOnWinningSide,
    );

    // ── 7. Assert final balances ─────────────────────────────────────────────
    // Winners hold exactly their payout and nothing else (they were minted
    // exactly their stake).
    assert_eq!(tok.balance(&alice), alice_payout);
    assert_eq!(tok.balance(&bob), bob_payout);
    assert_eq!(tok.balance(&carol), carol_payout);

    // Losers are wiped out to zero.
    assert_eq!(tok.balance(&dave), 0);
    assert_eq!(tok.balance(&eve), 0);

    // The platform fee reached the treasury: 5% of 750 tokens, plus the
    // 1 stroop of rounding dust the winners' floor division pushed over.
    assert_eq!(
        tok.balance(&treasury),
        375_000_001,
        "treasury receives the 5% platform fee (375_000_000) + 1 stroop dust",
    );

    // The market keeps only documented dust: everything else has been paid
    // out. Recomputing from the transfers proves conservation of value.
    assert_eq!(
        tok.balance(&contract_id),
        1,
        "residual is 1 stroop of documented dust",
    );
    assert_eq!(client.get_contract_balance(), 1);
    assert_eq!(
        alice_payout + bob_payout + carol_payout + tok.balance(&treasury) + 1,
        750 * ONE_TOKEN,
        "pot in == winners out + treasury fee + dust",
    );
}
