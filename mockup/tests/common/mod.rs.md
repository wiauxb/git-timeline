# `tests/common/mod.rs`

## Purpose
Build throwaway git repositories with a precise history for integration tests. Shared
by `tests/timeline.rs` and `tests/views.rs` (`mod common;`).

## Contents

```rust
pub struct Fixture { pub dir: tempfile::TempDir, day: Cell<u32> }

/// Isolated env: GIT_CONFIG_GLOBAL=/dev/null, GIT_CONFIG_NOSYSTEM=1,
/// author/committer = Alice <alice@example.com>.
pub fn env() -> Vec<(OsString, OsString)>;
/// `Git::with_env(PathBuf::new(), env())` — passed to the code under test.
pub fn base_git() -> Git;

impl Fixture {
    /// git init -b main; commit.gpgsign=false; core.autocrlf=false.
    pub fn new() -> Self;
    pub fn path(&self) -> &Path;
    /// Runs git with env() and GIT_AUTHOR_DATE/GIT_COMMITTER_DATE = 2024-01-<day>;
    /// panics on failure; returns trimmed stdout.
    pub fn git(&self, args: &[&str]) -> String;
    pub fn write(&self, rel: &str, content: &str) -> &Self;
    pub fn write_bytes(&self, rel: &str, content: &[u8]) -> &Self;
    pub fn stage(&self, rel: &str) -> &Self;               // git add
    pub fn mv(&self, from: &str, to: &str) -> &Self;       // git mv (creates parent dir)
    pub fn rm(&self, rel: &str) -> &Self;                  // git rm
    pub fn commit(&self, msg: &str) -> String;             // --allow-empty; day += 1; full oid
    pub fn commit_as(&self, author: &str, msg: &str) -> String;
    pub fn edit(&self, rel: &str, content: &str, msg: &str) -> String;  // write+stage+commit
    /// Cli::parse_from(["git-timeline", args…]) + RepoContext::discover_with(cwd = repo/cwd_rel).
    pub fn context(&self, cwd_rel: &str, args: &[&str]) -> git_timeline::Result<RepoContext>;
}

/// a.txt edited 5 times ("1\n", "1\n2\n", …).
pub fn linear_history() -> (Fixture, Vec<String>);
/// a.txt (add, edit) → git mv to dir/b.txt → edit b. 4 commits.
pub fn renamed_history() -> (Fixture, Vec<String>);
pub fn git_available() -> bool;
```

## Notes
* Dates advance one day per commit (stable ordering, deterministic relative dates).
* Fixtures never read the developer's git config.
