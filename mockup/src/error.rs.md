# `src/error.rs`

## Purpose
One error enum for the crate (`anyhow` only at the `lib::run` / `main` boundary). Each
variant has a user-facing `Display` message.

## Contents

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    GitNotFound,                                         // "git executable not found on PATH"
    GitTooOld { found: String, required: &'static str },
    NotARepository,
    OutsideRepository(PathBuf),
    IsADirectory(PathBuf),
    NoHistory(RepoPath),     // "'p' has no history in this range and does not exist in the working tree"
    BadRevision(String),
    /// Display: "git <subcommand> failed: <stderr without leading 'fatal: '>"
    Git { args: String, status: Option<i32>, stderr: String },
    Parse { context: &'static str },
    NotATty,
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
```

## Specs
* `Git::args` holds the full joined argument list (for debugging), but only its first
  word (the subcommand) is shown to the user, e.g.
  `git log failed: file src/main.rs has only 4 lines`.
* Errors **before** the TUI are printed by `main` (exit 1).
* Errors **inside** the TUI are not fatal: jobs carry them as `String` in `JobResult`;
  the UI shows an inline error, status message or error popup. Only an initial timeline
  failure with nothing to show is fatal (`AppState::fatal`).
