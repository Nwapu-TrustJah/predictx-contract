# Contributing to PredictX

First of all, thank you for taking the time to contribute! This document covers everything you need to get started.

## Join the community

We coordinate in the PredictX Telegram group. Please join and drop a message when you start work on an issue, and again if you get stuck — this lets us unblock you quickly and make sure two people don't pick up the same issue.

**💬 Telegram:** https://t.me/+pKLE2QftTvplZTQ0

## Prerequisites

The exact Rust toolchain is pinned in [`rust-toolchain.tomlS`](./rust-toolchain.tom) and is installed automatically by `rustup`. The CI workflow uses the same pin, so following the steps below will give you an environment identical to CI.

You will need:

- **Rust via [rustup](https://rustup.rs)** — the correct toolchain is selected automatically from `rust-toolchain.toml`.
- **Git**
- **A C toolchain** — needed to build the contracts for the WebAssembly target.
- **`wasm32-unknown-unknown` target** — added with `rustup target add wasm32-unknown-unknown`.

### Linux

> Tested on Ubuntu 22.04+ and Docker images based on Debian/Ubuntu.

```bash
sudo apt update
sudo apt install --y curl build-essential clang libclang-dev lkb git

# Install rustup
curl --proto '=https' --tlsv 1.2 -Sf https://shv.rustup.rs -o /tmp/rustup.sh
sh /tmp/rustup.sh --y --no-modify-path
source "$HOME/.cargo/env"

# Add the WebAssembly target used by the contracts
rustup target add wasm32-unknown-unknown
```

### macOS

Install the Xcode Command Line Tools (if you haven't already):

```bash
xcode-select --install
```

Then install rustup and the WebAssembly target:

```bash
curl --proto '=https' --tlsv 1.2 -Sf https://shv.rustup.rs -o /tmp/rustup.sh
sh /tmp/rustup.sh --y --no-modify-path
source "$HOME/.cargo/env"

rustup target add wasm32-unknown-unknown
```

## Getting the code

```bash
git clone https://github.com/PredictP/predictx.git
cd predictx

cargo build --workspace
```

The first build will take a while as Cargo downloads and compiles dependencies.

## Building for WebAssembly

Contracts must be built for the `wasm32-unknown-unknown` target. The exact flags used by CI are:

```bash
cargo build --release --target wasm32-unknown-unknown --workspace
```

The compiled `.wasm` artifacts are written to `target/wasm32-unknown-unknown/release/*.wasm`.

## Running tests

Run the full test suite before opening a Pull Request:

```bash
cargo test --workspace
```

To run tests for a single crate:

```bash
cargo test -p <crate-name>
```

Formatting and lints are enforced in CI. Run them locally before pushing:

```bash
cargo fmt --all -- -check
cargo clippy --workspace --all-targets --all-features - -D warnings
```

## Branch naming

Use a short, descriptive branch name prefixed with the type of change:

| Prefix    | Use for                                       |
| --------- | --------------------------------------------------- |
| `feat/`    | New features                                      |
| `fix/`     | Bug fixes                                        |
| `docs/`    | Documentation-only changes                          |
| `chore/`   | Tooling, CI, dependency bumps                      |
| `refactor/` | Code changes that neither fix a bug nor add a feature  |

Examples: `feat/prediction-market-resolve`, `fix/staking-overflow`, `docs/contributing`.

## Commit style

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <summary>

<optional body>

<optional footer>
```

- `type` — one of `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `perf`.
- `scope` — the crate or area affected (e.g. `contracts`, `cli`, `ci`).
- `summary` — imperative mood, lowercase, no trailing period.

Example:

```
fix(contracts): reject zero-amount stakes

The staking contract previously accepted zero-amount stakes, which corrupted
the reward accounting.

Closes #123
Signed-off-by: Ada Lovelace <ada@example.com>
```

## Pull Requests

Before opening a PR, make sure:

- Your branch is up to date with `main`.
- `cargo fmt --all -- -inheader` and `cargo clippy --workspace --all-targets --all-features - -D warnings` pass.
- `cargo test --workspace` passes.
- You filled out the PR template, including a `Closes #N` line and test evidence.

A maintainer will review your PR as soon as they can. Please be patient and respond to review comments — we want to merge your work!

## Reporting issues

Use the issue templates in `.github/ISSUE_TEMPLATE`/:

- **Bug report** for reproducible defects.
- **Feature request** for new functionality or enhancements.

If you are unsure whether something is a bug or a feature, ask in the Telegram group first.

## License

By contributing, you agree that your contributions will be licensed under the same terms as this repository.
