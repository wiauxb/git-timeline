# `tests/common/mod.rs`

## Purpose
Build throwaway git repositories with a precise history for integration tests.
Shared by every file in `tests/`.

## Contents

```rust
pub struct Fixture { pub dir: tempfile::TempDir }

impl Fixture {
    /// git init + deterministic config:
    ///   user.name/email, commit.gpgsign=false, init.defaultBranch=main,
    ///   core.autocrlf=false. Env for commits: GIT_AUTHOR_DATE / GIT_COMMITTER_DATE
    ///   incremented by 1 day per commit (stable ordering & relative dates).
    pub fn new() -> Self;
    pub fn path(&self) -> &Path;

    pub fn write(&self, rel: &str, content: &str) -> &Self;
    pub fn write_bytes(&self, rel: &str, content: &[u8]) -> &Self;
    pub fn rm(&self, rel: &str) -> &Self;
    pub fn mv(&self, from: &str, to: &str) -> &Self;        // git mv
    pub fn stage(&self, rel: &str) -> &Self;                // git add
    pub fn commit(&self, msg: &str) -> String;              // returns full oid
    pub fn commit_as(&self, author: &str, msg: &str) -> String;
    pub fn branch(&self, name: &str) -> &Self;
    pub fn checkout(&self, name: &str) -> &Self;
    pub fn merge(&self, name: &str, msg: &str) -> String;   // --no-ff
    pub fn git(&self, args: &[&str]) -> String;             // raw escape hatch
    pub fn context(&self, file: &str, cli: &[&str]) -> RepoContext;  // via Cli::parse_from
}

/// Pre-built scenarios
pub fn linear_history() -> (Fixture, Vec<String>);        // 5 edits of a.txt
pub fn renamed_history() -> (Fixture, Vec<String>);       // a.txt → dir/b.txt (+ edits)
pub fn merge_history() -> (Fixture, Vec<String>);         // edits on two branches + merge
pub fn binary_and_symlink() -> (Fixture, Vec<String>);
```

## Notes
* Tests are skipped (not failed) with a message when `git` is not installed or older
  than 2.36.
* Fixtures never depend on the developer's global git config:
  `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1` are set for fixture commands
  **and** passed to the code under test via the `Git` runner's env override (test-only
  constructor `Git::with_env`, `#[doc(hidden)] pub`).
