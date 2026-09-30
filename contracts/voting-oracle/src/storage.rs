use predictx_shared::{PollStatus, PredictXError, VoteTally};
use soroban_sdk::{contracttype, Address, Env, Vec};

// ── Keys & stored types ───────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub(crate) struct StoredPollStatus {
    pub(crate) status: PollStatus,
    pub(crate) updated_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub(crate) enum DataKey {
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
}

// ── Poll status storage ───────────────────────────────────────────────────────

pub(crate) fn get_admin(env: &Env) -> Result<Address, PredictXError> {
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
use crate::DataKey;
use predictx_shared::{Dispute, PredictXError, VoteTally};
use predictx_shared::DataKey;
use predictx_shared::{PredictXError, VoteChoice, VoteTally};
use soroban_sdk::{Address, Env, Vec};

// ── Admin registry storage ────────────────────────────────────────────────────

/// Read the registered admins, defaulting to an empty list.
pub fn read_admins(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::AdminList)
        .unwrap_or(Vec::new(env))
}

/// Persist the registered admins.
pub fn write_admins(env: &Env, admins: &Vec<Address>) {
    env.storage().instance().set(&DataKey::AdminList, admins);
}

/// Whether `addr` is a registered admin.
pub fn is_admin(env: &Env, addr: &Address) -> bool {
    read_admins(env).contains(addr.clone())
}

/// Ensure `caller` is a registered admin, else `Unauthorized`.
pub fn require_admin(env: &Env, caller: &Address) -> Result<(), PredictXError> {
    if is_admin(env, caller) {
        Ok(())
    } else {
        Err(PredictXError::Unauthorized)
    }
}

// ── Vote tally storage ────────────────────────────────────────────────────────

/// Read the vote tally for a poll, if one has been stored yet.
///
/// Tally data lives in *temporary* storage: it is only needed during the
/// voting window (matching the tier guidance in the shared `DataKey` layout).
pub fn read_tally(env: &Env, poll_id: u64) -> Option<VoteTally> {
    env.storage().temporary().get(&DataKey::VoteTally(poll_id))
}

/// Store the vote tally for a poll.
pub fn write_tally(env: &Env, tally: &VoteTally) {
    env.storage()
        .temporary()
        .set(&DataKey::VoteTally(tally.poll_id), tally);
}

// ── Voter roster storage ─────────────────────────────────────────────────────

/// The voter roster is *window-scoped*: it is only authoritative while the
/// voting window is open. Once the window closes, the roster is no longer
/// consulted for admission decisions and the poll settles from the tally that
/// was accumulated during the window.
///
/// Abuse model: an attacker can fill the roster with up to `MAX_VOTERS`
/// distinct (possibly sybil) addresses to exhaust the cap. Because the cap is
/// window-scoped and the poll can still be settled from the tally recorded
/// before the cap was hit, exhausting the roster cannot permanently freeze the
/// poll. It only limits how many additional distinct voters can be admitted
/// during the current window; the poll remains settleable by community vote.
///
/// Recovery path: when the cap is reached, the poll is not permanently
/// unsettleable. The window can be closed (or has already closed), at which
/// point settlement proceeds from the stored tally rather than requiring new
/// voters. There is no permanent freeze because the cap does not block
/// resolution.

/// Read the persistent voter roster for a poll, defaulting to an empty list.
pub fn read_voters(env: &Env, poll_id: u64) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::Voters(poll_id))
        .unwrap_or(Vec::new(env))
}

/// Persist the voter roster for a poll.
pub fn write_voters(env: &Env, poll_id: u64, voters: &Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::Voters(poll_id), voters);
}

// ── Vote-dedup storage ────────────────────────────────────────────────────────

/// Whether `voter` has already cast a vote on `poll_id`.
pub fn has_voted(env: &Env, poll_id: u64, voter: &Address) -> bool {
    env.storage()
        .temporary()
        .get(&DataKey::HasVoted(poll_id, voter.clone()))
        .unwrap_or(false)
}

/// Record that `voter` cast a vote on `poll_id`.
///
/// The marker lives in *temporary* storage so it expires with the tally when
/// the voting window closes.
pub fn write_voted(env: &Env, poll_id: u64, voter: &Address) {
    env.storage()
        .temporary()
        .set(&DataKey::HasVoted(poll_id, voter.clone()), &true);
}

// ── Dispute storage ───────────────────────────────────────────────────────────

/// Read the dispute for a poll, if one exists.
/// Read the dispute raised against `poll_id`, if any.
/// Read the dispute recorded for `poll_id`, if any.
pub fn read_dispute(env: &Env, poll_id: u64) -> Option<Dispute> {
    env.storage().persistent().get(&DataKey::Dispute(poll_id))
}

/// Persist the dispute for a poll.
/// Whether a dispute record already exists for `poll_id`.
pub fn has_dispute(env: &Env, poll_id: u64) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Dispute(poll_id))
}

/// Persist a dispute record.
/// Persist the dispute raised against `poll_id`.
/// Persist the dispute for `poll_id`.
pub fn write_dispute(env: &Env, dispute: &Dispute) {
    env.storage()
        .persistent()
        .set(&DataKey::Dispute(dispute.poll_id), dispute);
// ── Voter reward storage ──────────────────────────────────────────────────────

/// The choice `voter` recorded on `poll_id`, if they voted.
pub fn read_vote_choice(env: &Env, poll_id: u64, voter: &Address) -> Option<VoteChoice> {
    env.storage()
        .persistent()
        .get(&DataKey::VoterChoice(poll_id, voter.clone()))
}

/// Persist the choice `voter` recorded on `poll_id`.
///
/// Stored in *persistent* storage (unlike the temporary tally and dedup marker)
/// because the choice must outlive the voting window so eligible voters can
/// still be identified when rewards are claimed.
pub fn write_vote_choice(env: &Env, poll_id: u64, voter: &Address, choice: VoteChoice) {
    env.storage()
        .persistent()
        .set(&DataKey::VoterChoice(poll_id, voter.clone()), &choice);
}

/// The voter reward reserve for `poll_id` (0 when unset).
pub fn read_reward_pool(env: &Env, poll_id: u64) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::RewardPool(poll_id))
        .unwrap_or(0)
}

/// Persist the voter reward reserve for `poll_id`.
pub fn write_reward_pool(env: &Env, poll_id: u64, amount: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::RewardPool(poll_id), &amount);
}

/// Whether `voter` has already claimed their reward on `poll_id`.
// ── Voter-reward claim storage ───────────────────────────────────────────────

/// Whether `voter` has already claimed their reward for `poll_id`.
///
/// Reward claims outlive the voting window, so the marker is *persistent*
/// rather than temporary (unlike the vote-dedup marker above).
pub fn has_claimed_reward(env: &Env, poll_id: u64, voter: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::RewardClaimed(poll_id, voter.clone()))
        .unwrap_or(false)
}

/// Record that `voter` claimed their reward on `poll_id`.
/// Record that `voter` has claimed their reward for `poll_id`.
///
/// Written *before* the token transfer (checks-effects-interactions) so a
/// repeated or re-entrant claim cannot pass the entry check and drain the
/// reward pool.
pub fn write_reward_claimed(env: &Env, poll_id: u64, voter: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::RewardClaimed(poll_id, voter.clone()), &true);
}
// ── Admin approval storage ───────────────────────────────────────────────────

/// Whether `admin` has already approved an outcome for `poll_id`.
pub fn has_approved(env: &Env, poll_id: u64, admin: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Approval(poll_id, admin.clone()))
}

/// Read the outcome approved by `admin` on `poll_id`, if any.
pub fn read_approval(env: &Env, poll_id: u64, admin: &Address) -> Option<VoteChoice> {
    env.storage()
        .persistent()
        .get(&DataKey::Approval(poll_id, admin.clone()))
}

/// Record that `admin` approved `outcome` on `poll_id`.
pub fn write_approval(env: &Env, poll_id: u64, admin: &Address, outcome: VoteChoice) {
    env.storage()
        .persistent()
        .set(&DataKey::Approval(poll_id, admin.clone()), &outcome);
}

/// Read the total number of admin approvals for `poll_id`. Defaults to 0.
pub fn read_approval_count(env: &Env, poll_id: u64) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::ApprovalCount(poll_id))
        .unwrap_or(0)
}

/// Persist the total number of admin approvals for `poll_id`.
pub fn write_approval_count(env: &Env, poll_id: u64, count: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::ApprovalCount(poll_id), &count);
}

// ── Dispute approval storage ──────────────────────────────────────────────────

/// Admin approvals recorded for resolving `poll_id` to `outcome`.
pub fn read_dispute_approvals(env: &Env, poll_id: u64, outcome: VoteChoice) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::DisputeApprovals(poll_id, outcome))
        .unwrap_or(0)
}

/// Persist the approval count for resolving `poll_id` to `outcome`.
pub fn write_dispute_approvals(env: &Env, poll_id: u64, outcome: VoteChoice, approvals: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::DisputeApprovals(poll_id, outcome), &approvals);
}
