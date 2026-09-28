# Paused-State Behaviour

This document lists every state-changing entry point in the `prediction-market` contract and whether it is callable while the contract is paused.

| Entry Point                   | Paused Behaviour | Notes                                                                                |
| ----------------------------- | ---------------: | ------------------------------------------------------------------------------------ |
| `initialize`                  |         Rejected | One-time initializer; cannot be called when paused (contract should start unpaused). |
| `set_oracle`                  |         Rejected | Admin-only; blocked while paused.                                                    |
| `pause`                       |          Allowed | Admin-only; must remain callable to enter paused state.                              |
| `unpause`                     |          Allowed | Admin-only; must remain callable to exit paused state.                               |
| `cancel_poll`                 |         Rejected | Admin-only; blocked while paused.                                                    |
| `create_poll`                 |         Rejected | Creator-only; blocked while paused.                                                  |
| `stake`                       |         Rejected | Staker action; blocked while paused.                                                 |
| `resolve_poll` (oracle)       |         Rejected | Oracle resolution is blocked while paused.                                           |
| `resolve_poll` (admin/payout) |         Rejected | Admin manual resolution is blocked while paused.                                     |
| `claim_winnings`              |         Rejected | Claiming payouts is blocked while paused.                                            |
| `emergency_withdraw`          |          Allowed | Must remain callable while paused to allow fund recovery in incidents.               |
| `create_match`                |         Rejected | Admin match creation is blocked while paused.                                        |
| `update_match`                |         Rejected | Admin match updates are blocked while paused.                                        |
| `finish_match`                |         Rejected | Admin finishing matches is blocked while paused.                                     |
| `check_emergency_eligible`    |          Allowed | Read-only check for emergency eligibility.                                           |
| `get_*` / view functions      |          Allowed | All view-only entry points remain callable when paused.                              |

Implementation notes

- The pause guard is implemented via `ensure_not_paused(env)` which returns an error when the contract is paused.
- `emergency_withdraw` intentionally remains callable while paused to enable safe fund recovery.

Acceptance criteria mapping

- `resolve_poll`, `claim_winnings`, `create_match`, `update_match`, `finish_match` are rejected while paused ✅
- `emergency_withdraw` remains callable while paused ✅
- Tests added to assert paused behaviour across mutators ✅
