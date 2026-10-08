# `src/error.rs`

## Purpose
One error enum for the whole crate (except `main.rs` which uses `anyhow` for context).
Each variant has a **user-facing** `Display` message; some carry a hint.

## Contents

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("git executable not found on PATH")]
    GitNotFound,

    #[error("git {found} is too old; git-timeline needs git >= {required}")]
    GitTooOld { found: String, required: &'static str },

    #[error("not inside a git repository")]
    NotARepository,

    #[error("'{0}' is outside the repository")]
    OutsideRepository(std::path::PathBuf),

    #[error("'{0}' is a directory; git-timeline works on a single file")]
    IsADirectory(std::path::PathBuf),

    #[error("'{0}' has no history in this range and does not exist in the working tree")]
    NoHistory(crate::model::RepoPath),

    #[error("invalid revision or range '{0}'")]
    BadRevision(String),

    /// A git command exited non-zero. `stderr` is trimmed and shown as-is.
    #[error("`git {args}` failed: {stderr}")]
    Git { args: String, status: Option<i32>, stderr: String },

    /// Output of git did not match the expected format (bug or unexpected git version).
    #[error("could not parse git output ({context})")]
    Parse { context: &'static str },

    #[error("stdin/stdout is not a terminal")]
    NotATty,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

## Specs
* Errors **before** the TUI → printed by `main` and exit 1.
* Errors **inside** the TUI (a job failed) are *not* fatal: `update` stores them and
  `ui/dialog.rs` shows an error popup or the file pane shows an inline error box
  (e.g. "blame failed: …"). Only an error in the initial timeline load with zero entries
  is fatal (the app quits and `main` prints it).
* `Error` must be `Send + 'static` (travels through worker channels). Errors are wrapped
  in `Arc<Error>` inside `JobResult` so they can be cached and cloned.
