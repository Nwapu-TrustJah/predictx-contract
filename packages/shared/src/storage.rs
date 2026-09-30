use soroban_sdk::{contracttype, Address};

use crate::PollStatus;

/// Storage keys for all PredictX contracts.
///
/// Storage tier guidance:
/// - **Instance**  : admin config, counters, flags — lives as long as the contract.
/// - **Persistent**: polls, matches, stakes, user data — must survive TTL extensions.
/// - **Temporary** : vote tallies — only needed during the voting window.
#[contracttype]
pub enum DataKey {
    // ── Instance storage ──────────────────────────────────────────────────────
    /// Admin `Address`. (Instance)
    Admin,
    /// Soroban token contract `Address` used for staking. (Instance)
    TokenAddress,
    /// Cross-contract `Address` of the VotingOracle contract. (Instance)
    VotingOracle,
    /// Circuit-breaker flag `bool` — `true` while the contract is paused. (Instance)
    Paused,
    /// Treasury contract `Address` that receives platform fees. (Instance)
    TreasuryAddress,
    /// Platform fee in basis points. (Instance)
    PlatformFeeBps,
    /// Duration of the voting window in seconds. (Instance)
    VotingWindowSecs,
    /// Duration of the dispute window in seconds. (Instance)
    DisputeWindowSecs,
    /// Consensus threshold in basis points. (Instance)
    ConsensusThresholdBps,
    /// Auto-incrementing poll ID counter. (Instance)
    NextPollId,
    /// Auto-incrementing match ID counter. (Instance)
    NextMatchId,
    /// Initialisation flag. (Instance)
    Initialized,
    /// Registered admins list `Vec<Address>`. (Instance)
    AdminList,
    /// Platform-wide aggregate stats `PlatformStats`. (Instance)
    PlatformStats,
    /// Treasury token balance `i128`. (Instance)
    TreasuryBalance,
    /// Registered PredictionMarket contract `Address` allowed to deposit fees
    /// into the treasury. (Instance)
    TreasuryMarket,
    /// VotingOracle contract `Address` wired to the prediction market. (Instance)
    MarketVotingOracle,
    /// Emergency-pause flag for the prediction market. (Instance)
    MarketPaused,
    /// Treasury contract `Address` receiving platform fees. (Instance)
    MarketTreasuryAddress,

    // ── Persistent storage ────────────────────────────────────────────────────
    /// `match_id` → `Match`. (Persistent)
    Match(u64),
    /// `poll_id` → `Poll`. (Persistent)
    Poll(u64),
    /// `status` → `Vec<u64>` poll IDs currently in that status bucket. (Persistent)
    PollsByStatus(PollStatus),
    /// `(poll_id, user)` → `Stake`. (Persistent)
    Stake(u64, Address),
    /// `user` → `Vec<u64>` poll IDs the user has staked on. (Persistent)
    UserStakes(Address),
    /// `match_id` → `Vec<u64>` poll IDs attached to the match. (Persistent)
    MatchPolls(u64),
    /// `(poll_id, user)` → `bool` — has this user staked? (Persistent)
    HasStaked(u64, Address),
    /// `poll_id` → `Dispute`. (Persistent)
    Dispute(u64),
    /// `(poll_id, admin)` → `bool` — has this admin approved? (Persistent)
    AdminApproval(u64, Address),
    /// `user` → `UserStats`. (Persistent)
    UserStats(Address),
    /// `(poll_id, voter)` → `i128` unclaimed voter reward. (Persistent)
    VoterReward(u64, Address),
    /// `poll_id` → `VotingOracle`-specific poll metadata. (Persistent)
    OraclePoll(u64),
    /// `(poll_id, user)` → `bool` — has this user already done an emergency
    /// withdrawal for this poll? (Persistent)
    EmergencyClaimed(u64, Address),
    /// `poll_id` → oracle-side poll status snapshot. (Persistent)
    PollStatus(u64),
    /// `poll_id` → outcome the oracle auto-resolved to. (Persistent)
    PollOutcome(u64),
    /// `poll_id` → roster of voters who cast a vote. (Persistent)
    Voters(u64),
    /// `(poll_id, voter)` → `VoteChoice` the voter recorded. (Persistent)
    VoterChoice(u64, Address),
    /// `poll_id` → unclaimed voter reward reserve. (Persistent)
    RewardPool(u64),
    /// `(poll_id, voter)` → `bool` — has this voter claimed their reward? (Persistent)
    RewardClaimed(u64, Address),
    /// `poll_id` → `i128` total escrowed stake for the poll. (Persistent)
    PollEscrow(u64),
    /// `(poll_id, depositor)` → `i128` recorded treasury deposit balance. (Persistent)
    TreasuryDepositorBalance(Address),
    /// `(poll_id, user)` → `bool` — emergency withdrawal already claimed. (Persistent)
    EmergencyClaimed(u64, Address),
    /// `poll_id` → `StoredPollStatus` (status + last update time). (Persistent)
    OraclePollStatus(u64),
    /// `poll_id` → automatically resolved outcome. (Persistent)
    OraclePollOutcome(u64),
    /// `poll_id` → persistent roster of voters who cast a vote. (Persistent)
    OracleVoters(u64),
    /// `(poll_id, voter)` → the choice the voter recorded. (Persistent)
    VoterChoice(u64, Address),
    /// `poll_id` → voter reward reserve (unclaimed incentive pool). (Persistent)
    VoterRewardPool(u64),
    /// `(poll_id, voter)` → `bool` — has the voter claimed their reward? (Persistent)
    VoterRewardClaimed(u64, Address),

    // ── Temporary storage ─────────────────────────────────────────────────────
    /// `poll_id` → `VoteTally`. (Temporary — only needed during voting window)
    VoteTally(u64),
    /// `(poll_id, voter)` → `bool` — has this voter cast a vote? (Temporary)
    HasVoted(u64, Address),
    /// `poll_id` → `VotingOracle`-specific tally snapshot. (Temporary)
    OracleTally(u64),
}
