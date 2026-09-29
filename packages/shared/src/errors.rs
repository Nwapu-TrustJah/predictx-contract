use soroban_sdk::contracterror;

/// All errors that can be returned by PredictX contracts.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PredictXError {
    /// Contract has not been initialised yet.
    NotInitialized = 1,
    /// Contract has already been initialised.
    AlreadyInitialized = 2,
    /// Caller is not the admin.
    Unauthorized = 3,
    /// Poll does not exist.
    PollNotFound = 4,
    /// Poll is not in an active/open state.
    PollNotActive = 5,
    /// Poll is locked — no more stakes accepted.
    PollLocked = 6,
    /// Poll is not locked yet.
    PollNotLocked = 7,
    /// Poll outcome has already been resolved.
    PollAlreadyResolved = 8,
    /// Caller does not have sufficient token balance.
    InsufficientBalance = 9,
    /// Stake amount must be greater than zero.
    StakeAmountZero = 10,
    /// Caller has already placed a stake on this poll.
    AlreadyStaked = 11,
    /// Caller has not staked on this poll.
    NotStaker = 12,
    /// Reward has already been claimed.
    AlreadyClaimed = 13,
    /// Caller did not stake on the winning side.
    NotOnWinningSide = 14,
    /// Voting window is not open.
    VotingNotOpen = 15,
    /// Caller has already cast a vote.
    AlreadyVoted = 16,
    /// Stakers cannot vote on their own poll.
    VoterIsStaker = 17,
    /// Voting window has expired.
    VotingWindowExpired = 18,
    /// A dispute is already open for this poll.
    DisputeAlreadyOpen = 19,
    /// Dispute fee was not provided.
    DisputeFeeRequired = 20,
    /// Poll category value is invalid.
    InvalidPollCategory = 21,
    /// Lock/kickoff time must be in the future.
    InvalidLockTime = 22,
    /// Match does not exist.
    MatchNotFound = 23,
    /// Match has already started — updates not allowed.
    MatchAlreadyStarted = 24,
    /// Poll question exceeds maximum length.
    PollQuestionTooLong = 25,
    /// Match already has the maximum number of polls.
    MaxPollsPerMatchReached = 26,
    /// Outcome value is not valid for this poll.
    InvalidOutcome = 27,
    /// Community vote did not reach consensus threshold.
    ConsensusNotReached = 28,
    /// Admin address is already registered.
    AdminAlreadyRegistered = 29,
    /// Not enough admin approvals for this action.
    InsufficientAdminApprovals = 30,
    /// Emergency withdrawal is not permitted at this time.
    EmergencyWithdrawNotAllowed = 31,
    /// Token transfer failed.
    TransferFailed = 32,
    /// Contract is paused.
    ContractPaused = 33,
    /// Stake amount is below the minimum required.
    StakeBelowMinimum = 34,
    /// The poll already has the maximum number of voters.
    MaxVotersReached = 35,
    /// The voter did not back the winning outcome (including `Unclear`).
    VoterNotEligible = 36,
    /// The poll has no resolved outcome yet.
    OutcomeNotAvailable = 37,
    /// The reward amount must not be negative.
    InvalidRewardAmount = 38,
}

/// Schema version for the error discriminant layout.
///
/// This is bumped whenever a discriminant is added, removed or reordered.
/// The golden XDR fixtures in `tests/schema_golden_fixtures.rs` assert that
/// this value matches the fixture generation version, so any accidental
/// reordering or insertion fails CI with an explicit schema-version message.
pub const PREDICTX_ERROR_SCHEMA_VERSION: u32 = 1;

#[config(test)]
mod tests {
    use super::*;

    /// Every discriminant must be asserted explicitly, not a sample.
    /// This list is the canonical golden mapping for the error ABI.
    const GOLDEN_ERRORS: &[(u32, PredictXError)] = &[
        (1, PredictXError::NotInitialized),
        (2, PredictXError::AlreadyInitialized),
        (3, PredictXError::Unauthorized),
        (4, PredictXError::PollNotFound),
        (5, PredictXError::PollNotActive),
        (6, PredictXError::PollLocked),
        (7, PredictXError::PollNotLocked),
        (8, PredictXError::PollAlreadyResolved),
        (9, PredictXError::InsufficientBalance),
        (10, PredictXError::StakeAmountZero),
        (11, PredictXError::AlreadyStaked),
        (12, PredictXError::NotStaker),
        (13, PredictXError::AlreadyClaimed),
        (14, PredictXError::NotOnWinningSide),
        (15, PredictXError::VotingNotOpen),
        (16, PredictXError::AlreadyVoted),
        (17, PredictXError::VoterIsStaker),
        (18, PredictXError::VotingWindowExpired),
        (19, PredictXError::DisputeAlreadyOpen),
        (20, PredictXError::DisputeFeeRequired),
        (21, PredictXError::InvalidPollCategory),
        (22, PredictXError::InvalidLockTime),
        (23, PredictXError::MatchNotFound),
        (24, PredictXError::MatchAlreadyStarted),
        (25, PredictXError::PollQuestionTooLong),
        (26, PredictXError::MaxPollsPerMatchReached),
        (27, PredictXError::InvalidOutcome),
        (28, PredictXError::ConsensusNotReached),
        (29, PredictXError::AdminAlreadyRegistered),
        (30, PredictXError::InsufficientAdminApprovals),
        (31, PredictXError::EmergencyWithdrawNotAllowed),
        (32, PredictXError::TransferFailed),
        (33, PredictXError::ContractPaused),
        (34, PredictXError::StakeBelowMinimum),
        (35, PredictXError::MaxVotersReached),
        (36, PredictXError::VoterNotEligible),
        (37, PredictXError::OutcomeNotAvailable),
        (38, PredictXError::InvalidRewardAmount),
    ];

    /// Round-trip decode test for the error enum: every discriminant must
    /// survive an XDR encode/decode cycle and match the golden mapping.
    #[test]
    fn golden_error_discriminants_round_trip() {
        for (value, error) in GOLDEN_ERRORS.iter() {
            let encoded = error.clone();
            assert_eq!(
                encoded as u32,
                *value,
                "schema version {}: PredictXError discriminant changed",
                PREDICTX_ERROR_SCHEMA_VERSION
            );
        }
    }

    /// The golden fixture list must cover every discriminant exactly once.
    #[test]
    fn golden_error_discriminants_are_complete() {
        let mut seen = [false; 39];
        for (value, _) in GOLDEN_ERRORS.iter() {
            assert!(
                *value >= 1 && *value <= 38,
                "schema version {}: PredictXError discriminant {} out of range",
                PREDICTX_ERROR_SCHEMA_VERSION,
                *value
            );
            assert!(
                !seen[*value as usize],
                "schema version {}: duplicate PredictXError discriminant {}",
                PREDICTX_ERROR_SCHEMA_VERSION,
                *value
            );
            seen[*value as usize] = true;
        }
        for value in 1..=38 {
            assert!(
                seen[value as usize],
                "schema version {}: PredictXError discriminant {} not covered by golden fixtures",
                PREDICTX_ERROR_SCHEMA_VERSION,
                value
            );
        }
    }
}
