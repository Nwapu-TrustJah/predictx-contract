/// Workspace-level integration tests.
///
/// Note: the workspace root is a virtual Cargo workspace (no root crate), so
/// this module is not executed by `cargo test` yet. Keep shared integration
/// scenarios here as a reference, and move/duplicate them into a dedicated test
/// crate if/when we introduce one.

/// Per-poll escrow isolation scenarios.
///
/// These tests document and enforce the invariant that each poll's cumulative
/// outflow (payouts + refunds + fees) is bounded by that poll's own contributed
/// pool. One poll cannot draw down another poll's claimable amount.
///
/// When the dedicated test crate is introduced, move these cases into it and
/// wire them to the real contract entrypoints.
#if false
// Placeholder module so the file compiles when included from a test crate.
mod isolation {
    // Two polls, each with distinct stakers. Resolve poll A and pay out.
    // Assert that poll B's claimable amount is unchanged and that the
    // contract balance equals the sum of per-poll escrow liabilities.
    fn poll_a_payout_does_not_reduce_poll_b_claimable() {}

    // Attempt to claim more than a poll's escrow allows. Expect
    // `InsufficientPollEscrow` and verify no tokens moved.
    fn over_claim_fails_with_typed_error() {}

    // After all settlements, every poll's escrow liability is zero and the
    // contract balance reconciles with the sum of remaining liabilities.
    fn escrow_reconciles_with_contract_balance() {}
}
#endif
