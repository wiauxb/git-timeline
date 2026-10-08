# `src/git/mod.rs`

## Purpose
The **only** place that spawns `git`. A small runner with consistent flags, environment
and error conversion; sub-modules build commands and parse output.

## Contents

```rust
pub mod blame; pub mod blob; pub mod log; pub mod repo; pub mod show; pub mod status;

#[derive(Clone, Debug)]
pub struct Git {
    dir: PathBuf,                                  // every command runs with `-C <dir>`
    env: Arc<Vec<(OsString, OsString)>>,           // extra env (test fixtures)
}

impl Git {
    pub fn new(dir: PathBuf) -> Self;
    #[doc(hidden)] pub fn with_env(dir: PathBuf, env: Vec<(OsString, OsString)>) -> Self;
    /// Same settings (env), other directory.
    pub fn at(&self, dir: PathBuf) -> Self;
    pub fn dir(&self) -> &Path;

    /// git -C <dir> --no-pager -c color.ui=never -c core.quotepath=off
    ///     -c log.showSignature=false <args…>
    /// env: GIT_TERMINAL_PROMPT=0, GIT_OPTIONAL_LOCKS=0, LC_ALL=C; GIT_PAGER/PAGER removed;
    ///      then the extra env. stdin null, stdout+stderr piped.
    pub fn command<I, S>(&self, args: I) -> Command;

    /// Run to completion; non-zero exit ⇒ Error::Git { args, status, stderr }.
    pub fn output<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<Vec<u8>>;
    /// Like `output`, but non-zero exit ⇒ Ok(None) (for `--verify -q` style probes).
    pub fn probe<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<Option<Vec<u8>>>;
    /// Bytes written to stdin from a helper thread (blame --contents -).
    pub fn output_with_stdin<S: AsRef<OsStr>>(&self, args: &[S], input: &[u8]) -> Result<Vec<u8>>;
    /// Spawn; stdout wrapped in a 64 KiB BufReader for incremental parsing.
    pub fn stream<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<GitStream>;
}

pub struct GitStream { child: Option<Child>, pub stdout: BufReader<ChildStdout>,
                       stderr: Arc<Mutex<Vec<u8>>>, stderr_thread: Option<JoinHandle<()>>,
                       args: String }
impl GitStream { pub fn finish(self) -> Result<()>; }   // wait, join stderr, map exit status
impl Drop for GitStream { /* kill + wait if not finished (cancellation) */ }
```

## Specs
* stderr is always captured, never inherited (it would corrupt the TUI). For `stream`, a
  helper thread drains stderr into a buffer capped at 64 KiB (no pipe deadlock).
* `GIT_OPTIONAL_LOCKS=0`: `git status` doesn't take `index.lock`.
* `LC_ALL=C` only affects messages; data comes from explicit formats and `-z`.
* Spawn failure `NotFound` ⇒ `Error::GitNotFound`; other spawn errors ⇒ `Error::Io`.
* `Error::Git.args` is the space-joined argument list; its `Display` shows only the
  subcommand and git's stderr without the `fatal: ` prefix.
