use predictx_shared::{
    Poll, PollStatus, PredictXError, Stake, StakeSide, BPS_DENOMINATOR,
};
use soroban_sdk::{Address, Env};

use crate::token_utils;
use crate::DataKey;

/// Compute a winner's proportional payout:
/// `stake / winning_pool * (total_pool - fee)`.
pub(crate) fn proportional_payout(
    stake_amount: i128,
    winning_pool: i128,
    total_pool: i128,
    fee_bps: u32,
) -> i128 {
    let fee = total_pool * fee_bps as i128 / BPS_DENOMINATOR as i128;
    let distributable = total_pool - fee;
    stake_amount * distributable / winning_pool
}

/// Claim winnings for a user who staked on the winning side of a resolved poll.
///
/// Transfers `stake / winning_pool * (total_pool - fee)` tokens to the user.
/// Claimed-flag guard, treasury fee routing, and empty-pool refunds are
/// intentionally out of scope for this function's issue.
pub fn claim_winnings(
    env: &Env,
    user: Address,
    poll_id: u64,
) -> Result<i128, PredictXError> {
    user.require_auth();

    let poll: Poll = env
        .storage()
        .persistent()
        .get(&DataKey::Poll(poll_id))
        .ok_or(PredictXError::PollNotFound)?;

    if poll.status != PollStatus::Resolved {
        return Err(PredictXError::PollNotActive);
    }

    let outcome = poll.outcome.ok_or(PredictXError::InvalidOutcome)?;
    let winning_side = if outcome {
        StakeSide::Yes
    } else {
        StakeSide::No
    };

    let stake: Stake = env
        .storage()
        .persistent()
        .get(&DataKey::Stake(poll_id, user.clone()))
        .ok_or(PredictXError::NotStaker)?;

    if stake.side != winning_side {
        return Err(PredictXError::NotOnWinningSide);
    }

    let winning_pool = if outcome { poll.yes_pool } else { poll.no_pool };
    // Empty-pool edge cases are a separate issue — assume both pools non-empty.
    let total_pool = poll.yes_pool + poll.no_pool;
    let fee_bps = token_utils::get_platform_fee_bps(env);
    let payout = proportional_payout(stake.amount, winning_pool, total_pool, fee_bps);

    token_utils::transfer_from_contract(env, &user, payout)?;

    Ok(payout)
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use predictx_shared::{PollCategory, PollStatus, StakeSide};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env, String,
    };

    use crate::{DataKey, PredictionMarket, PredictionMarketClient};

    struct TestSetup<'a> {
        env: Env,
        admin: Address,
        token_addr: Address,
        contract_id: Address,
        client: PredictionMarketClient<'a>,
    }

    fn setup() -> TestSetup<'static> {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let oracle_id = env.register(crate::voting_oracle::WASM, ());
        let oracle_client = crate::voting_oracle::Client::new(&env, &oracle_id);
        oracle_client.initialize(&admin);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin);
        let token_addr = token_contract.address();

        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle_id, &token_addr, &treasury, &500_u32);
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);

        TestSetup {
            env,
            admin,
            token_addr,
            contract_id,
            client,
        }
    }

    fn create_poll(s: &TestSetup, lock_time: u64) -> u64 {
        let match_id = s.client.create_match(
            &s.admin,
            &String::from_str(&s.env, "Arsenal"),
            &String::from_str(&s.env, "Chelsea"),
            &String::from_str(&s.env, "Premier League"),
            &String::from_str(&s.env, "Emirates"),
            &(lock_time + 3600),
        );
        s.client.create_poll(
            &s.admin,
            &match_id,
            &String::from_str(&s.env, "Will Palmer score?"),
            &PollCategory::PlayerEvent,
            &lock_time,
        )
    }

    fn mint_tokens(s: &TestSetup, to: &Address, amount: i128) {
        token::StellarAssetClient::new(&s.env, &s.token_addr).mint(to, &amount);
    }

    fn stake_user(s: &TestSetup, poll_id: u64, side: StakeSide, amount: i128) -> Address {
        let user = Address::generate(&s.env);
        mint_tokens(s, &user, amount);
        s.client.stake(&user, &poll_id, &amount, &side);
        user
    }

    /// Mark poll Resolved with the given Yes/No outcome (storage helper for tests).
    fn resolve_poll(s: &TestSetup, poll_id: u64, outcome_yes: bool) {
        s.env.as_contract(&s.contract_id, || {
            let mut poll: Poll = s
                .env
                .storage()
                .persistent()
                .get(&DataKey::Poll(poll_id))
                .unwrap();
            poll.status = PollStatus::Resolved;
            poll.outcome = Some(outcome_yes);
            poll.resolution_time = s.env.ledger().timestamp();
            s.env
                .storage()
                .persistent()
                .set(&DataKey::Poll(poll_id), &poll);
        });
    }

    #[test]
    fn winning_staker_receives_proportional_payout() {
        let s = setup();
        let poll_id = create_poll(&s, 2_000_000);
        // Yes: 100, No: 300 → total 400, fee 5% = 20, distributable 380
        // Winner payout = 100/100 * 380 = 380
        let winner = stake_user(&s, poll_id, StakeSide::Yes, 100_000_000);
        stake_user(&s, poll_id, StakeSide::No, 300_000_000);
        resolve_poll(&s, poll_id, true);

        let paid = s.client.claim_winnings(&winner, &poll_id);
        assert_eq!(paid, 380_000_000);

        let tok = token::Client::new(&s.env, &s.token_addr);
        assert_eq!(tok.balance(&winner), 380_000_000);
    }

    #[test]
    fn losing_staker_gets_not_on_winning_side() {
        let s = setup();
        let poll_id = create_poll(&s, 2_000_000);
        stake_user(&s, poll_id, StakeSide::Yes, 100_000_000);
        let loser = stake_user(&s, poll_id, StakeSide::No, 300_000_000);
        resolve_poll(&s, poll_id, true);

        let err = s
            .client
            .try_claim_winnings(&loser, &poll_id)
            .expect_err("loser must fail")
            .unwrap();
        assert_eq!(err, PredictXError::NotOnWinningSide);
    }

    #[test]
    fn non_staker_gets_not_staker() {
        let s = setup();
        let poll_id = create_poll(&s, 2_000_000);
        stake_user(&s, poll_id, StakeSide::Yes, 100_000_000);
        stake_user(&s, poll_id, StakeSide::No, 100_000_000);
        resolve_poll(&s, poll_id, true);

        let stranger = Address::generate(&s.env);
        let err = s
            .client
            .try_claim_winnings(&stranger, &poll_id)
            .expect_err("non-staker must fail")
            .unwrap();
        assert_eq!(err, PredictXError::NotStaker);
    }

    #[test]
    fn unresolved_poll_gets_poll_not_active() {
        let s = setup();
        let poll_id = create_poll(&s, 2_000_000);
        let user = stake_user(&s, poll_id, StakeSide::Yes, 100_000_000);
        stake_user(&s, poll_id, StakeSide::No, 100_000_000);
        // leave Active — do not resolve

        let err = s
            .client
            .try_claim_winnings(&user, &poll_id)
            .expect_err("unresolved must fail")
            .unwrap();
        assert_eq!(err, PredictXError::PollNotActive);
    }

    #[test]
    fn proportional_math_helper_matches_formula() {
        // stake=50, win_pool=200, total=500, fee_bps=500 (5%)
        // fee=25, distributable=475, payout=50*475/200=118
        assert_eq!(proportional_payout(50, 200, 500, 500), 118);
    }
}
