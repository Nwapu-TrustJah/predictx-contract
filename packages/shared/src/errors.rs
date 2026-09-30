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
    /// Lock/kickoff time must be in the future and not after the match kickoff.
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
    /// Evidence string is invalid or empty.
    InvalidEvidence = 35,
    /// The poll already has the maximum number of voters.
    MaxVotersReached = 35,
    /// The poll's parent match has not finished yet.
    MatchNotFinished = 36,
    /// The voter did not back the winning outcome (including `Unclear`).
    VoterNotEligible = 36,
    /// The poll has no resolved outcome yet.
    OutcomeNotAvailable = 37,
    /// The reward amount must not be negative.
    InvalidRewardAmount = 38,
    /// The requested poll status transition is not part of the legal graph.
    InvalidStateTransition = 39,
    /// Caller did not vote on this poll and cannot claim a voter reward.
    NotEligibleVoter = 36,
    /// The dispute window has closed; the poll can no longer be disputed.
    DisputeWindowClosed = 36,
    /// The address supplied as the voting oracle is not a compatible oracle.
    InvalidOracle = 39,
    /// Oracle rotation was rejected because polls are still unresolved.
    OracleRotationBlocked = 40,
    /// A team name cannot be empty.
    EmptyTeamName = 39,
    /// A match string field exceeds the maximum allowed length.
    MatchStringTooLong = 40,
    /// Stake amount is above the maximum allowed for a single stake.
    StakeAboveMaximum = 40,
    /// The stake is on the winning side but its payout rounds down to zero.
    PayoutRoundsToZero = 41,
    /// Duplicate address provided among required distinct addresses.
    DuplicateAddress = 39,
    /// Address is not a valid token contract.
    InvalidTokenAddress = 40,
    /// The token address cannot be changed while the contract holds a balance.
    ContractBalanceNotZero = 39,
}

#[cfg(test)]
mod test {
    use super::PredictXError;

    /// Discriminants are part of the contract's public interface: they are
    /// what an indexer decodes an error code against. New variants must only
    /// ever be *appended* so previously-deployed error codes keep their
    /// meaning.
    #[test]
    fn appended_discriminants_are_stable() {
        assert_eq!(PredictXError::NotInitialized as u32, 1);
        assert_eq!(PredictXError::Unauthorized as u32, 3);
        assert_eq!(PredictXError::NotOnWinningSide as u32, 14);
        assert_eq!(PredictXError::ContractPaused as u32, 33);
        assert_eq!(PredictXError::StakeBelowMinimum as u32, 34);
        assert_eq!(PredictXError::InvalidRewardAmount as u32, 38);
        assert_eq!(PredictXError::InvalidStateTransition as u32, 39);
        assert_eq!(PredictXError::StakeAboveMaximum as u32, 40);
        assert_eq!(PredictXError::PayoutRoundsToZero as u32, 41);
    }
}

/// Alias for `EmptyTeamName` matching alternative naming conventions.
#[allow(non_upper_case_globals)]
pub const TeamNameEmpty: PredictXError = PredictXError::EmptyTeamName;

/// Alias for `MatchStringTooLong` matching alternative naming conventions.
#[allow(non_upper_case_globals)]
pub const MatchFieldTooLong: PredictXError = PredictXError::MatchStringTooLong;
