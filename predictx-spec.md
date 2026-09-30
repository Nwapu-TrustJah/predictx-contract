# Football Prediction Market Platform
## Product Specification Document

---

## 📋 Table of Contents
1. [Product Overview](#product-overview)
2. [How It Works](#how-it-works)
3. [User Roles](#user-roles)
4. [Core Features](#core-features)
5. [Technical Architecture](#technical-architecture)
6. [Smart Contract Specifications](#smart-contract-specifications)
7. [Frontend Requirements](#frontend-requirements)
8. [User Flows](#user-flows)
9. [Security & Compliance](#security-compliance)
10. [Development Phases](#development-phases)

---

## 🎯 Product Overview

### What We're Building
A Web3-based prediction market platform where users stake cryptocurrency on specific football match events. This is **NOT a traditional betting platform** - it's a community-driven prediction game with transparent, blockchain-based resolution.

### The Problem We Solve
- Traditional sports betting is opaque and controlled by bookmakers
- Fans want more granular, engaging ways to predict match outcomes
- Existing prediction markets lack social/community elements
- Trust issues with centralized platforms

### Our Solution
A decentralized platform where:
- Users create prediction polls about specific match events
- Community stakes crypto on opposing sides
- Winners split the pool proportionally
- Outcomes verified by community voting + admin oversight
- Full transparency via blockchain

### Key Differentiators
1. **User-Generated Content**: Anyone can create prediction polls
2. **Pool-Based**: Multiple users on each side, proportional rewards
3. **Hybrid Oracle**: Community voting + admin verification for accuracy
4. **Granular Predictions**: Not just "who wins", but specific in-game events
5. **Gaming UI**: Engaging, game-like interface (not boring finance app)

---

## 🔄 How It Works

### Simple Example

**Scenario**: Chelsea vs Manchester United match

1. **Poll Creation**
   - User creates poll: "Will Palmer score a goal?"
   - Sets lock time: Match kickoff (3:00 PM)

2. **Staking Phase** (Before 3:00 PM)
   - Users stake on "Yes" or "No"
   - Example stakes (in token base units):
     - Alice: 100 tokens on Yes
     - Bob: 200 tokens on Yes
     - Carol: 150 tokens on Yes
     - Dave: 250 tokens on No
     - Eve: 50 tokens on No
   - **Total Pool**: 750 tokens
     - Yes Pool: 450 tokens (60%)
     - No Pool: 300 tokens (40%)

3. **Match Happens**
   - Poll locks at 3:00 PM (no more stakes)
   - Match plays out
   - Palmer scores in the 67th minute! ⚽

4. **Resolution Phase** (After match ends)
   - Voting opens for 2 hours
   - Users who didn't stake on this poll vote on outcome
   - 45 voters say "Yes", 2 say "No" (96% consensus)
   - Automatically approved (>85% threshold)

5. **Payout**
   - Platform takes 5% fee: 750 × 0.05 = **37.5 tokens**
   - Winners split: 750 - 37.5 = **712.5 tokens**
   - Distribution (proportional to stake):
     - Alice: 100/450 × 712.5 = **158.33 tokens** (Profit: 58.33 tokens)
     - Bob: 200/450 × 712.5 = **316.67 tokens** (Profit: 116.67 tokens)
     - Carol: 150/450 × 712.5 = **237.50 tokens** (Profit: 87.50 tokens)
   - Dave and Eve lose their stakes (300 tokens total to winners)

### Mathematical Formula

```
Individual Payout = (User Stake / Winning Pool Total) × Total Pool × 0.95

Where:
- User Stake = Amount user put on winning side
- Winning Pool Total = All stakes on winning side
- Total Pool = All stakes from both sides
- 0.95 = After 5% platform fee
```

---

## 👥 User Roles

### 1. **Stakers** (Primary Users)
**What they do:**
- Browse upcoming matches
- Stake crypto on prediction polls
- Monitor their active predictions
- Claim winnings after resolution

**Permissions:**
- Create stakes on any active poll
- View their prediction history
- Withdraw winnings

**Restrictions:**
- Cannot stake after lock time
- Cannot change stake once placed
- Cannot vote on polls they staked on

---

### 2. **Poll Creators** (Also Stakers)
**What they do:**
- Create new prediction questions for matches
- Set poll parameters (question, lock time, category)
- Can stake on their own polls

**Permissions:**
- Create unlimited polls (may add limits later)
- Edit polls before first stake (optional feature)

**Restrictions:**
- Must be registered user
- Cannot create duplicate polls
- Polls must meet minimum standards (clear question, valid lock time)

---

### 3. **Voters/Judges** (Community Members)
**What they do:**
- Vote on poll outcomes after matches
- Review evidence (stats, video clips)
- Earn rewards for voting participation

**Permissions:**
- Vote on any poll they didn't stake on
- Challenge outcomes during dispute window

**Restrictions:**
- Cannot vote on polls they participated in
- Must vote within 2-hour window
- One vote per poll

**Rewards:**
- 0.5-1% of total pool divided among all voters
- Example: 1,000 token pool, 50 voters = ~0.1-0.2 tokens per voter

---

### 4. **Administrators** (Platform Team)
**What they do:**
- Verify outcomes when consensus is 60-85%
- Resolve disputes (multi-sig required for <60% consensus)
- Monitor platform for abuse
- Manage featured matches/polls

**Permissions:**
- Override community vote in contentious cases
- Pause polls if issues detected
- Ban malicious users
- Adjust platform parameters (with time-lock)

**Restrictions:**
- Must provide evidence for decisions
- Cannot unilaterally resolve >85% consensus polls
- Actions logged publicly on blockchain
- Multi-sig required for major decisions

---

## 🎮 Core Features

### Feature 1: Browse & Discover
**User Story**: As a user, I want to find interesting predictions to stake on.

**Components:**
- Home page with upcoming matches
- Match detail pages with all polls
- Trending polls (highest stakes, most participants)
- Search/filter by team, league, date
- Categories: Player Events, Team Events, Score Predictions, Fun/Wild

**UI Elements:**
- Match cards with team logos, date/time
- Poll cards showing question, pools, countdown
- Live pool distribution bars
- Participant counts

---

### Feature 2: Stake on Predictions
**User Story**: As a user, I want to put crypto on my prediction.

**Flow:**
1. Click poll → Opens staking modal
2. Choose side (Yes/No)
3. Enter stake amount
4. See potential winnings calculation (real-time)
5. Review pool distribution
6. Connect wallet (if not connected)
7. Confirm transaction
8. See success animation + confirmation

**Calculations to Show:**
- Current pool ratio (e.g., 65% Yes, 35% No)
- Your potential winnings if you win
- ROI percentage
- Platform fee (5%)
- All amounts displayed in token base units (i128) with proper decimals

**Validations:**
- Sufficient wallet balance
- Poll not locked yet
- Minimum stake amount (e.g., 10 tokens in base units)
- Maximum stake (e.g., 10,000 tokens in base units per poll)

---

### Feature 3: Create Polls
**User Story**: As a user, I want to create prediction questions for matches.

**Flow:**
1. Click "Create Poll"
2. Select match from upcoming matches
3. Choose category (dropdown)
4. Write question (text input, 10-150 characters)
5. Set lock time (dropdown: Kickoff, Halftime, 60min, Custom)
6. Preview poll appearance
7. Submit (small transaction fee)

**Categories:**
- Player Event (goals, assists, cards, substitutions)
- Team Event (possession, corners, shots on target)
- Score Prediction (final score, halftime score, goal difference)
- Fun/Wild (manager reactions, weather, random events)

**Validations:**
- Question must be clear and binary (Yes/No answerable)
- Lock time must be before match end
- No duplicate questions for same match

---

### Feature 4: My Dashboard
**User Story**: As a user, I want to track all my predictions in one place.

**Tabs:**

**Active Stakes**
- Polls I've staked on that haven't locked/resolved
- Shows: Match, question, my stake, my side, current pool status, time remaining
- Actions: View details

**Pending Resolution**
- Matches ended, waiting for voting/verification
- Shows: Status (Voting, Admin Review, Dispute)
- Actions: Vote (if eligible), View evidence

**Voting Opportunities**
- Polls I can vote on (didn't participate)
- Shows: Match, question, reward amount, evidence links
- Actions: Cast vote, View evidence

**Completed**
- Historical predictions
- Shows: Win/Loss badge, amount won/lost, ROI, outcome
- Filter: All, Wins, Losses
- Sort: Date, Profit

---

### Feature 5: Community Voting
**User Story**: As a voter, I want to help verify outcomes and earn rewards.

**Flow:**
1. Navigate to "Vote" section
2. See list of matches needing resolution
3. Click poll → Opens voting interface
4. Review:
   - Match context (final score, key events)
   - Poll question
   - Evidence section (stats, video clips, official sources)
   - Current vote tally (optional: show or hide to avoid bias)
5. Cast vote: Yes / No / Unclear
6. Confirm vote
7. See reward confirmation

**Evidence Sources:**
- Official league stats APIs
- Video clip embeds (YouTube, Twitter)
- Match reports from trusted sources
- User-submitted evidence (with moderation)

**Voting Reward Distribution:**
- Fixed percentage of pool (0.5-1%)
- Divided equally among all voters
- Paid out immediately after resolution

---

### Feature 6: Claim Winnings
**User Story**: As a winner, I want to receive my payout.

**Flow:**
1. Poll resolved → Notification appears
2. Navigate to "Completed" tab
3. See winning predictions with "Claim" button
4. Click "Claim"
5. Transaction processes
6. See success animation (confetti, trophy)
7. Winnings added to wallet

**UI Elements:**
- Big "You Won!" celebration screen
- Breakdown: Your stake → Your winnings → Profit
- Transaction receipt
- Share button (social media)

---

### Feature 7: Wallet Integration
**User Story**: As a user, I want to connect my crypto wallet securely.

**Supported Wallets:**
- Freighter (primary Stellar wallet)
- LOBSTR Wallet
- Albedo (browser-based)
- Other Stellar-compatible wallets via WalletConnect

**Wallet UI:**
- "Connect Wallet" button (prominent, top-right)
- Modal with wallet options
- Connected state shows:
  - Truncated address (GABC...XYZ9)
  - Balance (XLM + platform token balance)
  - Network indicator (Stellar Mainnet, Testnet, Futurenet)
- Dropdown menu: Profile, History, Disconnect

**Security:**
- Never store private keys
- Sign transactions only (no direct transfers without user consent)
- Network validation (warn if wrong network)
- Transaction preview before signing
- All token amounts displayed in base units (i128) with proper decimal formatting

---

## 🏗️ Technical Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────┐
│                      FRONTEND                            │
│  (React + Tailwind + Stellar SDK)                       │
│  - User Interface                                        │
│  - Wallet Connection (Freighter)                         │
│  - Smart Contract Interaction                            │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ Soroban RPC Calls
                 │
┌────────────────▼────────────────────────────────────────┐
│                STELLAR SOROBAN LAYER                     │
│  (Stellar Blockchain - Soroban Smart Contracts)         │
│                                                          │
│  ┌──────────────────┐  ┌──────────────────┐            │
│  │ PredictionMarket │  │  VotingOracle    │            │
│  │   Contract       │◄─┤   Contract       │            │
│  │   (Rust)         │  │   (Rust)         │            │
│  │                  │  │                  │            │
│  │ - Create Polls   │  │ - Community Vote │            │
│  │ - Stake Funds    │  │ - Admin Verify   │            │
│  │ - Claim Wins     │  │ - Resolve Polls  │            │
│  └──────────────────┘  └──────────────────┘            │
│                                                          │
│  ┌──────────────────┐                                   │
│  │   Treasury       │                                   │
│  │   Contract       │                                   │
│  │   (Rust)         │                                   │
│  │                  │                                   │
│  │ - Hold Fees      │                                   │
│  │ - Distribute     │                                   │
│  └──────────────────┘                                   │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ Events & State
                 │
┌────────────────▼────────────────────────────────────────┐
│                   BACKEND / INDEXER                      │
│  (Optional - for better UX)                             │
│  - Stellar Horizon API (index blockchain events)        │
│  - Cache poll data                                       │
│  - Match data API integration                            │
│  - Push notifications                                    │
└──────────────────────────────────────────────────────────┘
```

### Tech Stack

**Frontend:**
- **Framework**: React 18+ with TypeScript
- **Styling**: Tailwind CSS + custom gaming UI components
- **Web3**: 
  - @stellar/stellar-sdk (Stellar network interaction)
  - soroban-react (React hooks for Soroban)
  - Freighter Wallet integration (Stellar wallet)
- **State Management**: 
  - React Context API (global state)
  - TanStack Query (server state/caching)
- **Charts**: Recharts (pool distribution, stats)
- **Animations**: Framer Motion
- **Icons**: Lucide React
- **Forms**: React Hook Form + Zod validation

**Smart Contracts:**
- **Language**: Rust with soroban-sdk
- **Framework**: Soroban SDK for Stellar blockchain
- **Platform**: Stellar Soroban (smart contract platform)
- **Libraries**: 
  - soroban-sdk (core contract functionality)
  - predictx-shared (shared types and utilities across contracts)
- **Testing**: Rust unit and integration tests

**Backend (Optional but Recommended):**
- **Indexer**: Stellar Horizon API + custom event indexing
- **API**: Node.js + Express (match data, caching)
- **Database**: PostgreSQL (cache poll data for faster queries)
- **Match Data**: 
  - API-Football or similar (match schedules, results)
  - Web scraping fallback

**Infrastructure:**
- **Hosting**: Vercel or Netlify (frontend)
- **RPC Provider**: Stellar Horizon public or custom RPC nodes
- **IPFS**: Store evidence links, poll metadata (optional)
- **CDN**: Cloudflare (fast global access)

**DevOps:**
- **Version Control**: Git + GitHub
- **CI/CD**: GitHub Actions
- **Build Tools**: Cargo (Rust package manager), soroban-cli
- **Monitoring**: 
  - Sentry (error tracking)
  - Stellar Expert (blockchain explorer and contract monitoring)
- **Analytics**: Mixpanel or Amplitude (user behavior)

---

## 📜 Smart Contract Specifications

### Contract 1: PredictionMarket (Rust/Soroban)

**Purpose**: Main contract for poll creation, staking, and payouts on Stellar Soroban.

**Key Storage:**
```rust
// Contract data keys
enum DataKey {
    Admin,
    VotingOracle,
    TokenAddress,
    TreasuryAddress,
    PlatformFeeBps,
    Match(u64),
    Poll(u64),
    Stake(u64, Address),
    UserStakes(Address),
    HasStaked(u64, Address),
    PlatformStats,
    // ... additional keys for emergency handling
}

// Platform fee in basis points (500 = 5%)
const PLATFORM_FEE_BPS: u32 = 500;
```

**Core Data Types:**
```rust
pub struct Poll {
    pub poll_id: u64,
    pub match_id: u64,
    pub creator: Address,
    pub question: String,
    pub category: PollCategory,
    pub lock_time: u64,
    pub yes_pool: i128,      // Token amounts in i128 base units
    pub no_pool: i128,       // Token amounts in i128 base units
    pub yes_count: u32,
    pub no_count: u32,
    pub status: PollStatus,
    pub outcome: Option<bool>,
    pub resolution_time: u64,
    pub created_at: u64,
}

pub struct Stake {
    pub user: Address,
    pub poll_id: u64,
    pub amount: i128,        // Token amount in i128 base units
    pub side: StakeSide,
    pub claimed: bool,
    pub staked_at: u64,
}

pub enum PollStatus {
    Active,
    Locked,
    Voting,
    AdminReview,
    Disputed,
    Resolved,
    Cancelled,
}

pub enum StakeSide {
    Yes,
    No,
}
```

**Key Functions:**

```rust
// INITIALIZATION
pub fn initialize(
    env: Env,
    admin: Address,
    voting_oracle: Address,
    token_address: Address,
    treasury_address: Address,
    platform_fee_bps: u32,
) -> Result<(), PredictXError>

// CREATE POLL
pub fn create_poll(
    env: Env,
    creator: Address,
    match_id: u64,
    question: String,
    category: PollCategory,
    lock_time: u64,
) -> Result<u64, PredictXError>

// STAKE (with token transfer)
pub fn stake(
    env: Env,
    staker: Address,
    poll_id: u64,
    amount: i128,
    side: StakeSide,
) -> Result<Stake, PredictXError>

// RESOLVE (called by VotingOracle)
pub fn resolve_poll(
    env: Env,
    caller: Address,
    poll_id: u64,
    outcome: bool,
) -> Result<(), PredictXError>

// CLAIM WINNINGS
pub fn claim_winnings(
    env: Env,
    claimant: Address,
    poll_id: u64,
) -> Result<i128, PredictXError>

// VIEW FUNCTIONS
pub fn calculate_winnings(
    env: Env,
    poll_id: u64,
    user: Address,
) -> Result<i128, PredictXError>

pub fn get_poll(
    env: Env,
    poll_id: u64,
) -> Result<Poll, PredictXError>

pub fn get_stake_info(
    env: Env,
    poll_id: u64,
    user: Address,
) -> Result<Stake, PredictXError>

pub fn get_pool_info(
    env: Env,
    poll_id: u64,
) -> Result<PoolInfo, PredictXError>

// EMERGENCY WITHDRAWAL
pub fn emergency_withdraw(
    env: Env,
    user: Address,
    poll_id: u64,
) -> Result<i128, PredictXError>
```

**Events:**
Events are published using Soroban's event system:
```rust
env.events().publish((Symbol::new(&env, "PollCreated"), poll_id), ());
env.events().publish((Symbol::new(&env, "StakePlaced"), poll_id, user), amount);
env.events().publish((Symbol::new(&env, "PollResolved"), poll_id), outcome);
env.events().publish((Symbol::new(&env, "WinningsClaimed"), poll_id, user), amount);
```

---

### Contract 2: VotingOracle (Rust/Soroban)

**Purpose**: Manages community voting and admin verification for poll resolution.

**Key Storage:**
```rust
enum DataKey {
    Admin,
    AdminList,              // Vec<Address> of registered admins
    PollStatus(u64),
    VoteTally(u64),
    PollOutcome(u64),
    Voters(u64),
    HasVoted(u64, Address),
    VoterChoice(u64, Address),
    RewardPool(u64),
    VoterReward(u64, Address),
    RewardClaimed(u64, Address),
}

// Constants
const VOTING_WINDOW_SECS: u64 = 7200; // 2 hours
const MAX_VOTERS: u32 = 64;
```

**Core Data Types:**
```rust
pub struct VoteTally {
    pub poll_id: u64,
    pub yes_votes: u32,
    pub no_votes: u32,
    pub unclear_votes: u32,
    pub total_voters: u32,
    pub voting_end_time: u64,
    pub reward_pool: i128,    // Voter incentives in i128 base units
}

pub enum VoteChoice {
    Yes,
    No,
    Unclear,
}
```

**Key Functions:**

```rust
// INITIALIZATION
pub fn initialize(
    env: Env,
    admin: Address,
) -> Result<(), PredictXError>

// MULTI-ADMIN MANAGEMENT
pub fn add_admin(
    env: Env,
    caller: Address,
    new_admin: Address,
) -> Result<(), PredictXError>

pub fn remove_admin(
    env: Env,
    caller: Address,
    admin: Address,
) -> Result<(), PredictXError>

// POLL STATUS MANAGEMENT
pub fn set_poll_status(
    env: Env,
    poll_id: u64,
    status: PollStatus,
) -> Result<(), PredictXError>

pub fn get_poll_status(
    env: Env,
    poll_id: u64,
) -> PollStatus

// VOTING
pub fn cast_vote(
    env: Env,
    voter: Address,
    poll_id: u64,
    choice: VoteChoice,
) -> Result<VoteTally, PredictXError>

pub fn auto_resolve(
    env: Env,
    poll_id: u64,
) -> Result<VoteChoice, PredictXError>

pub fn can_vote(
    env: Env,
    poll_id: u64,
    voter: Address,
) -> bool

// VOTER REWARDS
pub fn set_reward_pool(
    env: Env,
    caller: Address,
    poll_id: u64,
    amount: i128,
) -> Result<(), PredictXError>

pub fn claim_reward(
    env: Env,
    voter: Address,
    poll_id: u64,
) -> Result<i128, PredictXError>

pub fn get_reward_pool(
    env: Env,
    poll_id: u64,
) -> i128
```

---

### Contract 3: Treasury (Rust/Soroban)

**Purpose**: Holds platform fees and manages fund distribution.

**Key Storage:**
```rust
enum DataKey {
    Admin,
    TokenAddress,
    TotalFeesCollected,
    // Additional keys for fee tracking and distribution
}
```

**Key Functions:**

```rust
// INITIALIZATION
pub fn initialize(
    env: Env,
    admin: Address,
    token_address: Address,
) -> Result<(), PredictXError>

// RECEIVE FEES (via token transfer)
pub fn deposit_fees(
    env: Env,
    from: Address,
    amount: i128,
) -> Result<(), PredictXError>

// DISTRIBUTE VOTER REWARDS
pub fn distribute_voter_rewards(
    env: Env,
    poll_id: u64,
    voters: Vec<Address>,
    total_reward: i128,
) -> Result<(), PredictXError>

// WITHDRAW FEES (admin only)
pub fn withdraw_fees(
    env: Env,
    admin: Address,
    amount: i128,
) -> Result<(), PredictXError>

// CLAIM VOTER REWARD
pub fn claim_voter_reward(
    env: Env,
    voter: Address,
) -> Result<i128, PredictXError>
```

---

## 🎨 Frontend Requirements

### Design System (Gaming UI)

**Color Palette:**
```css
/* Background */
--bg-primary: linear-gradient(135deg, #0a0e27 0%, #1a1f3a 100%);
--bg-card: rgba(26, 31, 58, 0.6);
--bg-card-hover: rgba(26, 31, 58, 0.8);

/* Accents */
--accent-cyan: #00d9ff;
--accent-green: #39ff14;
--accent-magenta: #ff006e;
--accent-gold: #ffd700;

/* Status Colors */
--success: #39ff14;
--warning: #ffaa00;
--danger: #ff006e;
--info: #00d9ff;

/* Text */
--text-primary: #ffffff;
--text-secondary: #b8c5d6;
--text-muted: #6b7a8f;
```

**Typography:**
```css
/* Headers */
font-family: 'Rajdhani', 'Orbitron', sans-serif;
text-transform: uppercase;
letter-spacing: 0.1em;

/* Body */
font-family: 'Barlow', 'Inter', sans-serif;

/* Numbers */
font-family: 'Roboto Mono', monospace;
```

**Component Patterns:**

**Buttons:**
```css
/* Primary CTA */
.btn-primary {
  background: linear-gradient(135deg, #00d9ff, #00a3cc);
  border: 2px solid #00d9ff;
  box-shadow: 0 0 20px rgba(0, 217, 255, 0.5);
  text-transform: uppercase;
  letter-spacing: 0.1em;
  font-weight: 700;
}

.btn-primary:hover {
  transform: scale(1.05);
  box-shadow: 0 0 30px rgba(0, 217, 255, 0.8);
}
```

**Cards:**
```css
.poll-card {
  background: rgba(26, 31, 58, 0.6);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(0, 217, 255, 0.3);
  border-radius: 12px;
  clip-path: polygon(
    0% 12px, 12px 0%, 
    100% 0%, 100% calc(100% - 12px), 
    calc(100% - 12px) 100%, 0% 100%
  ); /* Clipped corners */
}
```

---

### Page Specifications

#### 1. Home Page

**Sections:**
- Hero (full-screen)
  - Animated particles background
  - Tagline: "PREDICT. STAKE. WIN."
  - Subtitle: "Community-Powered Football Predictions"
  - CTA: "Connect Wallet" + "Browse Matches"
  - Platform stats (animated counters)

- Upcoming Matches (scrollable horizontal)
  - Match cards with team logos
  - Click → Goes to match detail page

- Trending Polls (grid layout)
  - Top 6 polls by total pool value
  - Shows question, pool amounts, participants

- How It Works (3-step visual)
  - Icons with animations
  - Brief explanations

**Animations:**
- Page load: Fade in with stagger
- Stats counters: Count up on scroll into view
- Cards: Float/hover effects
- Background: Slow particle movement

---

#### 2. Match Detail Page

**URL**: `/match/:matchId`

**Layout:**
```
┌─────────────────────────────────────────┐
│          MATCH HEADER                    │
│  Chelsea vs Man United                   │
│  Dec 20, 2024 - 3:00 PM                  │
│  Stamford Bridge                         │
└─────────────────────────────────────────┘
┌─────────────────────────────────────────┐
│     [Create Poll Button]                 │
└─────────────────────────────────────────┘
┌─────────────────────────────────────────┐
│  ACTIVE POLLS (Grid)                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐│
│  │ Poll 1   │ │ Poll 2   │ │ Poll 3   ││
│  │          │ │          │ │          ││
│  └──────────┘ └──────────┘ └──────────┘│
└─────────────────────────────────────────┘
```

**Poll Card Contains:**
- Question text
- Yes/No pools (with amounts and percentages)
- Pool distribution bar (visual)
- Countdown timer
- Participant count
- "Stake Now" button

---

#### 3. Staking Modal

**Triggered**: Click "Stake Now" on any poll

**Layout:**
```
┌─────────────────────────────────────────┐
│  Will Palmer score a goal?               │
│  Chelsea vs Man United                   │
├─────────────────────────────────────────┤
│  Choose Side:                            │
│  [ YES ]    [ NO ]                       │
│  (toggle buttons)                        │
├─────────────────────────────────────────┤
│  Stake Amount:                           │
│  [___________] Tokens                    │
│  Balance: 2,500 Tokens                   │
│  [50] [100] [500] [MAX]                  │
├─────────────────────────────────────────┤
│  Current Pool:                           │
│  ███████████░░░░░ 65% Yes, 35% No        │
│                                          │
│  Your Potential Winnings:                │
│  150 Tokens → 230 Tokens                 │
│  Profit: +80 Tokens (+53%)               │
│                                          │
│  Platform Fee: 5% on winnings            │
├─────────────────────────────────────────┤
│         [CONFIRM STAKE]                  │
└─────────────────────────────────────────┘
```

**Calculations Update In Real-Time** as user types amount.

---

#### 4. My Dashboard

**Tabs:**
- Active Stakes
- Pending Resolution
- Voting Opportunities
- Completed

**Active Stakes Tab:**
- List of polls user has staked on
- Each item shows:
  - Match + Poll question
  - Your stake + side
  - Current pool status
  - Time until lock
  - "View Details" button

**Pending Resolution Tab:**
- Shows matches that ended, waiting for resolution
- Status badges: "Voting", "Admin Review", "Dispute"

**Voting Opportunities Tab:**
- List of polls user can vote on
- Shows reward amount
- "Vote Now" button

**Completed Tab:**
- Historical predictions
- Win/Loss badges
- Profit/Loss amounts
- Filter: All, Wins, Losses

---

#### 5. Voting Interface

**URL**: `/vote/:pollId`

**Layout:**
```
┌─────────────────────────────────────────┐
│  POLL TO RESOLVE                         │
│  Will Palmer score a goal?               │
│  Chelsea vs Man United (Ended)           │
├─────────────────────────────────────────┤
│  MATCH RESULT                            │
│  Chelsea 2 - 1 Man United                │
│  Palmer scored in 67' ⚽                  │
├─────────────────────────────────────────┤
│  EVIDENCE                                │
│  [Video Clip] [Official Stats] [Report]  │
├─────────────────────────────────────────┤
│  COMMUNITY VOTES (optional)              │
│  45 voted Yes, 2 voted No                │
├─────────────────────────────────────────┤
│  CAST YOUR VOTE                          │
│  [ YES ]  [ NO ]  [ UNCLEAR ]            │
│                                          │
│  Your Reward: 2 tokens                   │
├─────────────────────────────────────────┤
│         [SUBMIT VOTE]                    │
└─────────────────────────────────────────┘
```

---

## 🔄 User Flows

### Flow 1: First-Time User Staking

1. User lands on home page (not connected)
2. Sees trending polls, intrigued
3. Clicks "Browse Matches"
4. Sees upcoming Chelsea vs Man United match
5. Clicks match → Match detail page
6. Sees poll: "Will Palmer score?"
7. Clicks "Stake Now"
8. Modal opens → Prompted to connect wallet
9. Clicks "Connect Wallet"
10. Wallet selection modal appears
11. Selects MetaMask
12. MetaMask popup → Confirms connection
13. Returns to staking modal (now connected)
14. Chooses "Yes" side
15. Enters 100 token stake
16. Sees potential winnings: 150 tokens
17. Clicks "Confirm Stake"
18. MetaMask popup → Confirms transaction
19. Transaction processing (spinner)
20. Success! Confetti animation
21. Redirected to "My Dashboard" → Active Stakes
22. Sees new stake listed

---

### Flow 2: Experienced User Voting

1. User logged in (wallet connected)
2. Receives notification: "New voting opportunity"
3. Navigates to "My Dashboard" → Voting Opportunities
4. Sees 3 polls ready to vote on
5. Clicks one: "Will Rashford start?"
6. Voting interface opens
7. Reviews match result (Rashford did start)
8. Watches video clip evidence
9. Checks official lineup stats
10. Community vote shows 40 Yes, 1 No
11. User votes "Yes"
12. Clicks "Submit Vote"
13. Transaction confirms
14. Success message: "Vote recorded! Reward: 2 tokens"
15. Returns to dashboard
16. Reward pending in balance

---

### Flow 3: Creating a Poll

1. User on Match Detail page
2. Clicks "Create Poll" button
3. Modal opens with form
4. Step 1: Match auto-selected (Chelsea vs Man United)
5. Step 2: Chooses category "Player Event"
6. Step 3: Types question: "Will Mudryk get a yellow card?"
7. Step 4: Sets lock time: "At Kickoff"
8. Preview shows how poll will appear
9. Clicks "Create Poll"
10. MetaMask popup (small gas fee)
11. Transaction confirms
12. Success! Poll now visible on match page
13. User can immediately stake on their own poll

---

### Flow 4: Claiming Winnings

1. Match ends, poll resolved
2. User receives notification: "You won!"
3. Navigates to "My Dashboard" → Completed
4. Sees winning prediction with green badge
5. Shows: Stake: 100 tokens → Winnings: 150 tokens (Profit: +50 tokens)
6. "Claim" button glowing
7. Clicks "Claim"
8. MetaMask confirms withdrawal transaction
9. Success screen with trophy animation
10. Balance updates in wallet
11. Transaction receipt shown
12. Share button: "Share your win on Twitter!"

---

## 🔒 Security & Compliance

### Smart Contract Security

**Critical Protections:**

1. **Reentrancy Guard**
   - Use OpenZeppelin's `ReentrancyGuard` on all withdrawal functions
   - Prevents attackers from draining funds

2. **Access Control**
   - Use `Ownable` and role-based access
   - Admin functions require multi-sig for critical actions
   - Time-locks on parameter changes

3. **Integer Overflow**
   - Solidity 0.8+ has built-in checks
   - Use SafeMath for extra safety if needed

4. **Frontrunning Protection**
   - Strict lock times enforced on-chain
   - No stakes accepted after lock time (checked via block.timestamp)

5. **Oracle Manipulation**
   - Hybrid system (community + admin) prevents single point of failure
   - Require non-participants to vote
   - Dispute mechanism for contentious cases

6. **Emergency Mechanisms**
   - Pause functionality for detected issues
   - Emergency withdrawal if poll cancelled
   - Dispute window for user recourse

**Audit Requirements:**
- Professional security audit before mainnet launch
- Use audit firms: Trail of Bits, OpenZeppelin, ConsenSys Diligence
- Bug bounty program post-launch

---

### Legal Considerations

**Regulatory Concerns:**
- Prediction markets may be classified as gambling in some jurisdictions
- Need legal opinion on classification in target markets

**Mitigation Strategies:**
1. **Skill-Based Framing**: Emphasize knowledge/skill over chance
2. **No House Edge**: Platform doesn't take opposing positions
3. **Community Resolution**: Peer-to-peer, not platform vs user
4. **Geographic Restrictions**: Block users from prohibited jurisdictions
5. **Age Verification**: Require 18+ (integrate with wallet verification)
6. **Terms of Service**: Clear disclaimers, user assumes risk

**Recommended Actions:**
- Consult with crypto-focused law firm
- Research: Polymarket, Augur, PredictIt case studies
- Consider DAO structure to decentralize control
- Get licenses if operating in regulated markets (UK, US states)

---

### Privacy & Data

**User Data:**
- Wallet addresses are pseudonymous (not anonymous)
- Don't collect personal info unless required by law
- Use IPFS for evidence storage (decentralized)

**GDPR Compliance** (if EU users):
- Right to be forgotten (difficult with blockchain)
- Minimize off-chain data collection
- Privacy policy clearly states blockchain permanence

---

## 🚀 Development Phases

### Phase 1: MVP (3-4 months)

**Goal**: Launch basic functional platform on testnet

**Deliverables:**
1. **Smart Contracts (v1)**
   - PredictionMarket contract (basic functionality)
   - Admin-only resolution (no voting yet)
   - Deploy to Stellar Testnet or Futurenet

2. **Frontend (Core)**
   - Home page
   - Match detail page
   - Staking interface
   - My Dashboard (Active + Completed tabs)
   - Wallet connection (Freighter wallet)

3. **Backend**
   - Match data API integration
   - Basic indexing of contract events via Stellar Horizon

4. **Testing**
   - Unit tests for contracts (90%+ coverage)
   - Frontend E2E tests (Cypress)
   - Internal user testing (10-20 users)

**Success Metrics:**
- 50+ test stakes placed
- 10+ matches with polls
- Zero critical bugs
- Positive user feedback

---

### Phase 2: Community Features (2-3 months)

**Goal**: Add voting system and improve UX

**Deliverables:**
1. **Smart Contracts (v2)**
   - VotingOracle contract implementation
   - Community voting mechanics
   - Dispute resolution
   - Treasury contract

2. **Frontend (Enhanced)**
   - Voting interface
   - Poll creation by users
   - Voting Opportunities tab
   - Evidence display
   - Improved animations/UI polish
   - Support for additional Stellar wallets (LOBSTR, Albedo)

3. **Backend**
   - Video clip embedding
   - Stats API integration
   - Push notifications (browser)

4. **Testing**
   - Security audit (preliminary)
   - Closed beta (100 users)

**Success Metrics:**
- 80%+ auto-resolution rate (consensus)
- <5% disputed polls
- Avg 20+ voters per poll

---

### Phase 3: Public Beta (2 months)

**Goal**: Launch on Stellar Mainnet with real tokens

**Deliverables:**
1. **Security**
   - Professional smart contract audit (Rust/Soroban focused)
   - Fix all findings
   - Bug bounty program launch

2. **Frontend**
   - Multi-wallet support (Freighter, LOBSTR, Albedo, WalletConnect)
   - Mobile app (React Native or PWA)
   - Advanced analytics dashboard
   - Social features (leaderboards, profiles)

3. **Marketing**
   - Landing page
   - Documentation/FAQ
   - Community Discord/Telegram
   - Influencer partnerships

4. **Operations**
   - Customer support system
   - Admin dashboard for moderation
   - Legal compliance (terms, privacy)

**Launch Strategy:**
- Soft launch: 1-2 featured matches
- Invite-only first 1,000 users
- Gradual rollout over 4 weeks
- Monitor for issues, quick fixes

**Success Metrics:**
- 1,000+ registered wallets
- 50K+ tokens total volume
- <1% error rate
- 4.0+ star user rating

---

### Phase 4: Scale & Optimize (Ongoing)

**Goal**: Grow user base and add features

**Roadmap Ideas:**
1. **Multi-Sport Expansion**
   - Basketball (NBA)
   - American Football (NFL)
   - Cricket, Tennis, etc.

2. **Advanced Features**
   - Combo predictions (multiple events)
   - Live in-game predictions
   - Peer-to-peer challenges
   - Prediction pools (group stakes)

3. **Gamification**
   - User levels/XP
   - Achievement badges
   - Seasonal leaderboards
   - Referral rewards

4. **DAO Governance**
   - Issue governance token
   - Community votes on features
   - Fee distribution to token holders

5. **Layer 2 / Scalability**
   - Optimize for Stellar's built-in scalability
   - Explore Soroban performance optimizations
   - Cross-chain bridges if needed

6. **Mobile Apps**
   - Native iOS/Android apps
   - Push notifications for match starts, results

---

## 📊 Success Metrics & KPIs

### Platform Health
- **Total Value Locked (TVL)**: $ staked across all active polls
- **Active Users**: Monthly active wallets
- **Poll Volume**: # of polls created per week
- **Average Pool Size**: Mean total stakes per poll
- **Platform Fees Collected**: Monthly revenue

### User Engagement
- **Retention Rate**: % users who stake 2+ times
- **Voting Participation**: % eligible voters who vote
- **Poll Creation Rate**: % users who create at least one poll
- **Time on Platform**: Average session duration
- **Referral Rate**: % users who invite others

### Resolution Accuracy
- **Auto-Resolution Rate**: % polls resolved via consensus (>85%)
- **Dispute Rate**: % polls that go to dispute
- **Admin Override Rate**: % polls requiring admin decision
- **Average Resolution Time**: Hours from match end to payout

### Quality Metrics
- **Error Rate**: % transactions that fail
- **Uptime**: % platform availability
- **User Satisfaction**: NPS score
- **Support Tickets**: # issues reported per week

---

## 📝 Glossary

**Terms for Developers:**

- **Poll**: A prediction question with binary outcome (Yes/No)
- **Stake**: Amount of crypto a user puts on one side of a poll
- **Pool**: Total amount staked on one side (Yes Pool vs No Pool)
- **Lock Time**: Deadline when poll closes to new stakes
- **Resolution**: Process of determining poll outcome
- **Oracle**: System that determines truth (in our case: voting + admin)
- **Consensus**: Agreement threshold for auto-resolution (85%)
- **Dispute**: Formal challenge to a poll outcome
- **Platform Fee**: Percentage taken from winning pool (5%)
- **Proportional Payout**: Winners split pot based on stake size
- **Voter Reward**: Small payment to community members who vote

**Example Calculation:**
```
Total Pool: 1,000 tokens
- Yes Pool: 700 tokens (70%)
- No Pool: 300 tokens (30%)

Outcome: Yes wins

Platform Fee: 1,000 × 5% = 50 tokens
Winners Split: 1,000 - 50 = 950 tokens

Alice staked 140 tokens on Yes (20% of Yes Pool):
Alice's Payout: (140 / 700) × 950 = 190 tokens
Alice's Profit: 190 - 140 = 50 tokens
```

---

## 🎯 Next Steps for Development Team

### Immediate Actions (Week 1):
1. Set up development environment
   - Initialize Git repository
   - Install Rust and soroban-cli
   - Set up React project with Tailwind and Stellar SDK

2. Define project structure
   - Frontend folder structure
   - Smart contract workspace with Cargo
   - Testing framework setup (Rust tests)

3. Create mock data
   - Sample matches
   - Sample polls
   - Test user data

4. Design database schema (if using backend)
   - Tables for cached data
   - Indexer structure for Horizon API events

### Sprint 1 (Weeks 2-4):
- **Smart Contracts**: Write PredictionMarket contract in Rust with soroban-sdk (basic version)
- **Frontend**: Build home page + wallet connection (Freighter integration)
- **Testing**: Unit tests for contract functions using Rust test framework

### Sprint 2 (Weeks 5-7):
- **Smart Contracts**: Add staking and payout logic with token transfers
- **Frontend**: Build match detail page + staking modal
- **Integration**: Connect frontend to Stellar testnet contracts using Soroban RPC

### Sprint 3 (Weeks 8-10):
- **Smart Contracts**: Implement VotingOracle contract
- **Frontend**: Build My Dashboard + voting interface
- **Testing**: Integration tests, user testing

---

## 📞 Questions for Stakeholders

Before development begins, clarify:

1. **Target Blockchain**: Stellar Soroban (testnet and mainnet deployment)
2. **Budget**: Development costs, audit costs (Rust/Soroban specific), infrastructure costs?
3. **Timeline**: Hard launch date? Phased rollout?
4. **Legal**: Do we have legal counsel? Which jurisdictions are we targeting?
5. **Team**: Who's on the team? Rust developers, designers, marketers?
6. **Competitive Analysis**: Who are our main competitors? What's our differentiation?
7. **Revenue Model**: Just platform fees? Future token launch? Other monetization?

---

## 📚 Additional Resources

**Learn Rust & Soroban:**
- Soroban Documentation: https://soroban.stellar.org/docs
- Soroban by Example: https://soroban.stellar.org/docs/learn/examples
- Rust Book: https://doc.rust-lang.org/book/
- Soroban Quest (Interactive Tutorials): https://quest.stellar.org/soroban

**Stellar & Web3 Frontend:**
- Stellar SDK for JavaScript: https://stellar.github.io/js-stellar-sdk/
- Freighter Wallet Docs: https://docs.freighter.app/
- soroban-react (React hooks): https://github.com/esteblock/soroban-react

**Security:**
- Soroban Smart Contract Security Best Practices: https://soroban.stellar.org/docs/learn/security
- Rust Security Guidelines: https://anssi-fr.github.io/rust-guide/

**Inspiration:**
- Polymarket (prediction market): https://polymarket.com
- Augur (decentralized oracle): https://augur.net
- PredictIt (regulated prediction market): https://www.predictit.org

---

**END OF SPECIFICATION**

*Last Updated: [Current Date]*  
*Version: 1.0*  
*Contributors: Product Team*