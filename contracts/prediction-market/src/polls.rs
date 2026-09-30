use predictx_shared::{Poll, PollStatus, PredictXError};
use soroban_sdk::{Env, Vec};

use crate::DataKey;

/// Maximum number of poll IDs returned per `get_polls_by_status` page.
pub const MAX_PAGE_SIZE: u32 = 50;

/// Read the full bucket of poll IDs currently in `status`.
pub(crate) fn read_index(env: &Env, status: PollStatus) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::PollsByStatus(status))
        .unwrap_or(Vec::new(env))
}

/// Add `poll_id` to the `status` bucket (idempotent).
pub(crate) fn add_to_index(env: &Env, poll_id: u64, status: PollStatus) {
    let mut index = read_index(env, status);
    if !index.contains(&poll_id) {
        index.push_back(poll_id);
        env.storage()
            .persistent()
            .set(&DataKey::PollsByStatus(status), &index);
    }
}

/// Remove `poll_id` from the `status` bucket (idempotent).
pub(crate) fn remove_from_index(env: &Env, poll_id: u64, status: PollStatus) {
    let index = read_index(env, status);
    let mut updated: Vec<u64> = Vec::new(env);
    let mut removed = false;
    for i in 0..index.len() {
        let id = index.get(i).unwrap();
        if id == poll_id {
            removed = true;
        } else {
            updated.push_back(id);
        }
    }
    if removed {
        env.storage()
            .persistent()
            .set(&DataKey::PollsByStatus(status), &updated);
    }
}

/// The single place a poll's status changes: moves the poll from its current
/// status bucket into `new_status` and persists the updated `Poll` record.
///
/// Every status transition in the contract must go through here so the index
/// stays consistent (a poll appears in exactly one bucket at a time).
pub(crate) fn transition_status(
    env: &Env,
    poll_id: u64,
    new_status: PollStatus,
) -> Result<(), PredictXError> {
    let mut poll: Poll = env
        .storage()
        .persistent()
        .get(&DataKey::Poll(poll_id))
        .ok_or(PredictXError::PollNotFound)?;

    if poll.status != new_status {
        remove_from_index(env, poll_id, poll.status);
        add_to_index(env, poll_id, new_status);
        poll.status = new_status;
        env.storage()
            .persistent()
            .set(&DataKey::Poll(poll_id), &poll);
    }
    Ok(())
}

/// Page through the poll IDs currently in `status`.
///
/// `start` is the zero-based offset into the bucket; `limit` caps the number of
/// results returned and is clamped to [`MAX_PAGE_SIZE`]. Requests past the end
/// of the bucket return an empty list.
pub(crate) fn get_polls_by_status(env: &Env, status: PollStatus, start: u32, limit: u32) -> Vec<u64> {
    let index = read_index(env, status);
    let total = index.len();
    if start >= total || limit == 0 {
        return Vec::new(env);
    }
    let capped = limit.min(MAX_PAGE_SIZE);
    let end = (start as u64 + capped as u64).min(total as u64) as u32;
    index.slice(start..end)
}

#[cfg(test)]
mod test {
    extern crate std;

    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Address, Env, String, Vec,
    };
    use predictx_shared::{PollCategory, PollStatus};

    use crate::{PredictionMarket, PredictionMarketClient, polls};

    struct TestSetup<'a> {
        env: Env,
        admin: Address,
        oracle_id: Address,
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
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_addr = token_contract.address();

        let contract_id = env.register(PredictionMarket, ());
        let client = PredictionMarketClient::new(&env, &contract_id);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &oracle_id, &token_addr, &treasury, &500_u32);

        env.ledger().with_mut(|l| l.timestamp = 1_000_000);

        TestSetup { env, admin, oracle_id, contract_id, client }
    }

    fn create_poll(s: &TestSetup, lock_time: u64) -> u64 {
        let match_id = s.client.create_match(
            &s.admin,
            &String::from_str(&s.env, "Arsenal"),
            &String::from_str(&s.env, "Chelsea"),
            &String::from_str(&s.env, "Premier League"),
            &String::from_str(&s.env, "Emirates"),
            &(lock_time + 3_600),
        );
        s.client.create_poll(
            &s.admin,
            &match_id,
            &String::from_str(&s.env, "Will Palmer score?"),
            &PollCategory::PlayerEvent,
            &lock_time,
        )
    }

    fn count_bucket_membership(s: &TestSetup, poll_id: u64) -> u32 {
        let statuses = [
            PollStatus::Active,
            PollStatus::Locked,
            PollStatus::Voting,
            PollStatus::AdminReview,
            PollStatus::Disputed,
            PollStatus::Resolved,
            PollStatus::Cancelled,
        ];
        let mut count = 0;
        for status in statuses {
            let bucket: Vec<u64> = s.env
                .as_contract(&s.contract_id, || {
                    polls::read_index(&s.env, status)
                });
            if bucket.contains(&poll_id) {
                count += 1;
            }
        }
        count
    }

    #[test]
    fn created_polls_are_indexed_as_active() {
        let s = setup();
        let p1 = create_poll(&s, 2_000_000);
        let p2 = create_poll(&s, 2_000_000);

        let active = s.client.get_polls_by_status(&PollStatus::Active, &0, &10);
        assert_eq!(active.len(), 2);
        assert!(active.contains(&p1));
        assert!(active.contains(&p2));
        assert_eq!(count_bucket_membership(&s, p1), 1);
        assert_eq!(count_bucket_membership(&s, p2), 1);
    }

    #[test]
    fn resolution_moves_poll_between_buckets() {
        let s = setup();
        let p1 = create_poll(&s, 2_000_000);
        let p2 = create_poll(&s, 2_000_000);

        s.client.resolve_poll(&s.oracle_id, &p1, &true);

        let active = s.client.get_polls_by_status(&PollStatus::Active, &0, &10);
        assert!(!active.contains(&p1));
        assert!(active.contains(&p2));

        let resolved = s.client.get_polls_by_status(&PollStatus::Resolved, &0, &10);
        assert_eq!(resolved.len(), 1);
        assert!(resolved.contains(&p1));
        assert_eq!(count_bucket_membership(&s, p1), 1);
    }

    #[test]
    fn cancellation_moves_poll_between_buckets() {
        let s = setup();
        let p1 = create_poll(&s, 2_000_000);

        s.client.cancel_poll(&s.admin, &p1);

        let active = s.client.get_polls_by_status(&PollStatus::Active, &0, &10);
        assert!(!active.contains(&p1));
        let cancelled = s.client.get_polls_by_status(&PollStatus::Cancelled, &0, &10);
        assert!(cancelled.contains(&p1));
        assert_eq!(count_bucket_membership(&s, p1), 1);
    }

    #[test]
    fn get_polls_by_status_paginates_with_start_and_limit() {
        let s = setup();
        let mut expected = Vec::new(&s.env);
        for _ in 0..5u64 {
            expected.push_back(create_poll(&s, 2_000_000));
            // offset lock times so each poll differs from the last
            s.env.ledger().with_mut(|l| l.timestamp += 1);
        }

        let page = s.client.get_polls_by_status(&PollStatus::Active, &2, &2);
        assert_eq!(page.len(), 2);
        assert_eq!(page.get(0).unwrap(), expected.get(2).unwrap());
        assert_eq!(page.get(1).unwrap(), expected.get(3).unwrap());

        let whole = s.client.get_polls_by_status(&PollStatus::Active, &0, &100);
        assert_eq!(whole.len(), 5);
    }

    #[test]
    fn get_polls_by_status_clamps_page_size() {
        let s = setup();

        s.env.as_contract(&s.contract_id, || {
            let mut ids = Vec::new(&s.env);
            for i in 0..(polls::MAX_PAGE_SIZE + 20) {
                ids.push_back(i as u64);
            }
            s.env.storage().persistent().set(
                &crate::DataKey::PollsByStatus(PollStatus::Locked),
                &ids,
            );
        });

        let page = s.client.get_polls_by_status(&PollStatus::Locked, &0, &10_000);
        assert_eq!(page.len(), polls::MAX_PAGE_SIZE);
    }

    #[test]
    fn get_polls_by_status_past_end_returns_empty() {
        let s = setup();
        create_poll(&s, 2_000_000);
        create_poll(&s, 2_000_000);

        assert_eq!(s.client.get_polls_by_status(&PollStatus::Active, &2, &10).len(), 0);
        assert_eq!(s.client.get_polls_by_status(&PollStatus::Voting, &0, &10).len(), 0);
        assert_eq!(s.client.get_polls_by_status(&PollStatus::Active, &0, &0).len(), 0);
    }
}