# `src/git/mod.rs`

## Purpose
The **only** place that spawns `git`. Provides a small runner with consistent flags,
environment, and error conversion. Sub-modules build commands and parse output.

## Contents

```rust
pub mod repo; pub mod log; pub mod status; pub mod blob; pub mod blame; pub mod show;

#[derive(Clone)]
pub struct Git {
    toplevel: std::path::PathBuf,   // every command runs with `-C <toplevel>`
}

impl Git {
    pub fn new(toplevel: PathBuf) -> Self;
    #[doc(hidden)] pub fn with_env(   // test fixtures only (tests/ can't see cfg(test))
        toplevel: PathBuf, env: Vec<(OsString, OsString)>) -> Self;

    /// Base command: git -C <toplevel> --no-pager -c color.ui=never
    ///   -c core.quotepath=off -c log.showSignature=false <args…>
    /// env: GIT_TERMINAL_PROMPT=0, GIT_OPTIONAL_LOCKS=0, LC_ALL=C (stable messages),
    ///      GIT_PAGER/PAGER removed. stdin = null unless specified.
    pub fn command<I, S>(&self, args: I) -> std::process::Command
        where I: IntoIterator<Item = S>, S: AsRef<std::ffi::OsStr>;

    /// Run to completion, return stdout; non-zero exit ⇒ Error::Git{stderr}.
    pub fn output(&self, args: &[&OsStr]) -> Result<Vec<u8>>;

    /// Same, with bytes written to stdin (used by `blame --contents -`).
    pub fn output_with_stdin(&self, args: &[&OsStr], stdin: &[u8]) -> Result<Vec<u8>>;

    /// Spawn and return a buffered reader on stdout for incremental parsing (git log).
    /// The returned guard kills + reaps the child on drop (cancellation).
    pub fn stream(&self, args: &[&OsStr]) -> Result<GitStream>;
}

pub struct GitStream { child: std::process::Child, pub stdout: BufReader<ChildStdout> }
impl GitStream { pub fn finish(self) -> Result<()>; }   // wait + map exit status
impl Drop for GitStream { /* kill() + wait() if not finished */ }
```

## Specs
* stderr is always captured (never inherited — it would corrupt the TUI).
  For `stream`, stderr is drained by a helper thread into a bounded buffer to avoid
  pipe deadlocks.
* `GIT_OPTIONAL_LOCKS=0` keeps `git status` from taking `index.lock` (no interference with
  the user's concurrent git commands).
* `LC_ALL=C` only affects git's messages; data is requested with explicit formats and `-z`.
* `core.quotepath=off` is belt-and-braces: all path-producing commands use `-z`.
* Spawn failure with `ErrorKind::NotFound` ⇒ `Error::GitNotFound`.
* Every invocation is logged (args + duration) to the debug log when enabled.
