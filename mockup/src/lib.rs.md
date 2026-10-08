# `src/lib.rs`

## Purpose
Library root of the crate. All modules live here so that **integration tests in
`tests/` can use them** (a binary-only crate can't be imported by `tests/*.rs`).
`main.rs` is a thin binary on top of this library.

## Contents

```rust
pub mod cli;
pub mod error;
pub mod terminal;
pub mod highlight;
pub mod git;
pub mod model;
pub mod app;
pub mod worker;
pub mod ops;
pub mod ui;

pub use error::{Error, Result};

/// Everything `main` needs: discover, set up terminal, run the app.
pub fn run(cli: cli::Cli) -> anyhow::Result<()>;
```

## Notes
* The library is an implementation detail, not a public API: no semver promise,
  `#![doc(hidden)]`-style items are fine (e.g. `git::Git::with_env` used by test fixtures).
* `[lib]` needs no section in `Cargo.toml` (`src/lib.rs` is picked up automatically,
  named `git_timeline`).
