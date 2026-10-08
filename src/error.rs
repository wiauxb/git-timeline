use std::path::PathBuf;

use crate::model::RepoPath;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("git executable not found on PATH")]
    GitNotFound,

    #[error("git {found} is too old; git-timeline needs git >= {required}")]
    GitTooOld { found: String, required: &'static str },

    #[error("not inside a git repository")]
    NotARepository,

    #[error("'{}' is outside the repository", .0.display())]
    OutsideRepository(PathBuf),

    #[error("'{}' is a directory; git-timeline works on a single file", .0.display())]
    IsADirectory(PathBuf),

    #[error("'{}' has no history in this range and does not exist in the working tree", .0.display())]
    NoHistory(RepoPath),

    #[error("invalid revision or range '{0}'")]
    BadRevision(String),

    /// A git command exited non-zero. Shown as `git <subcommand> failed: <stderr>`.
    #[error("git {} failed: {}", args.split(' ').next().unwrap_or_default(), stderr.trim_start_matches("fatal: "))]
    Git { args: String, status: Option<i32>, stderr: String },

    /// Output of git did not match the expected format.
    #[error("could not parse git output ({context})")]
    Parse { context: &'static str },

    #[error("stdin/stdout is not a terminal")]
    NotATty,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
