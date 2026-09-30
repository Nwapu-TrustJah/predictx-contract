/// Platform fee in basis points (BPS). `500` = 5%.
pub const PLATFORM_FEE_BPS: u32 = 500;

/// Maximum voter reward in basis points. `100` = 1%.
pub const VOTER_REWARD_BPS: u32 = 100;

/// Duration of the voting window in seconds. `7_200` = 2 hours.
pub const VOTING_WINDOW_SECS: u64 = 7_200;

/// Duration of the dispute window in seconds. `86_400` = 24 hours.
pub const DISPUTE_WINDOW_SECS: u64 = 86_400;

/// Vote share threshold for automatic resolution in BPS. `8_500` = 85%.
pub const AUTO_RESOLVE_THRESHOLD_BPS: u32 = 8_500;

/// Vote share threshold for admin review in BPS. `6_000` = 60%.
pub const ADMIN_REVIEW_THRESHOLD_BPS: u32 = 6_000;

/// Number of admin signatures required for multi-sig actions.
pub const MULTI_SIG_REQUIRED: u32 = 3;

/// Maximum length (in Unicode scalar values) for a poll question.
pub const MAX_QUESTION_LENGTH: u32 = 256;

/// Maximum length (in bytes) for match string fields (home team, away team, league, venue).
pub const MAX_MATCH_STRING_LENGTH: u32 = 256;

/// Maximum number of polls that can be attached to a single match.
pub const MAX_POLLS_PER_MATCH: u32 = 50;

/// Basis points denominator. Used as: `amount * fee_bps / BPS_DENOMINATOR`.
pub const BPS_DENOMINATOR: u32 = 10_000;

/// Timeout in seconds after which emergency withdrawal may be permitted. `604_800` = 7 days.
pub const EMERGENCY_TIMEOUT_SECS: u64 = 604_800;

/// Minimum stake amount in token base units. `10_000_000` = 10 tokens (7 decimal places).
pub const MIN_STAKE_AMOUNT: i128 = 10_000_000;

/// Fixed fee (in token base units) required to initiate a dispute. `1_000_000_000` = 100 tokens.
pub const DISPUTE_FEE: i128 = 1_000_000_000;
/// Maximum amount for a single stake on a poll, in token base units.
/// `100_000_000` = 100 tokens (7 decimal places). Caps any one stake so a
/// single account cannot dominate a pool, and keeps the payout arithmetic
/// inside a predictable range. The bound is per stake, not per user: several
/// smaller stakes on the same poll still add up.
pub const MAX_STAKE_AMOUNT: i128 = 100_000_000;

/// Upper bound on how many stakes [`get_user_stakes`] returns in one page.
/// Keeps the view's response size and host execution cost predictable when a
/// user has staked many times on many polls.
pub const MAX_USER_STAKES_PAGE_SIZE: u32 = 50;
/// TTL threshold for temporary storage entries (VoteTally, HasVoted).
/// Must exceed VOTING_WINDOW_SECS to prevent premature expiry.
pub const TEMPORARY_STORAGE_TTL_SECS: u64 = 14_400; // 4 hours

/// Target TTL extend-to value for temporary storage entries.
/// Used to re-arm the TTL before it expires.
pub const TEMPORARY_STORAGE_EXTEND_TO_SECS: u64 = 17_280; // 4.8 hours > 2 hours voting window
/// Delay (in seconds) between proposing a parameter change and being able to execute it.
///
/// Set to 24 hours (`86_400` seconds).  The goal is visibility, not friction: any user who
/// watches contract events or polls `get_param_proposal` has a full day to react before the
/// new value takes effect.  A longer window (e.g. 7 days) would be more appropriate for a
/// fully decentralised protocol but adds unnecessary friction during the current MVP phase.
pub const PARAM_TIMELOCK_DELAY: u64 = 86_400;
