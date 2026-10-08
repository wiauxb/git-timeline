# `src/git/repo.rs`

## Purpose
Startup discovery: find the repository, normalize the user's path into a `RepoPath`,
validate options against git, and produce the immutable `RepoContext` shared (`Arc`) by
the whole app. Every failure here happens **before** the TUI starts.

## Contents

```rust
pub const MIN_GIT: (u32, u32, u32) = (2, 36, 0);

#[derive(Clone, Debug)]
pub struct RepoContext {
    pub git: Git,                    // runs with -C <toplevel>
    pub toplevel: PathBuf,           // canonical
    pub invocation_dir: PathBuf,     // cwd at launch (export base directory)
    pub target: RepoPath,            // the file, at the tip of the explored history
    pub query: LogQuery,             // see git/log.rs
    pub show_local: bool,            // pseudo-entries enabled (= no revision range)
    pub git_version: (u32, u32, u32),
}

impl RepoContext {
    pub fn discover(cli: &Cli, cwd: PathBuf) -> Result<Self>;
    /// `base` supplies env settings (tests); its directory is ignored.
    pub fn discover_with(cli: &Cli, cwd: PathBuf, base: Git) -> Result<Self>;
}

fn git_version(git: &Git) -> Result<(u32, u32, u32)>;
fn normalize(p: &Path) -> PathBuf;                     // lexical `.` / `..`
fn canonicalize_existing_prefix(p: &Path) -> PathBuf;
fn to_repo_path(rel: &Path) -> RepoPath;               // components joined with '/'
```

## Discovery algorithm
1. `git --version` (in cwd) → parse `git version X.Y.Z…`; spawn NotFound ⇒ `GitNotFound`;
   `< MIN_GIT` ⇒ `GitTooOld` (2.36 is needed for `cat-file --batch-command`).
2. `git rev-parse --show-toplevel` in cwd; failure/empty ⇒ `NotARepository` (bare repos too).
   The result is canonicalized.
3. Path:
   * `abs = normalize(cwd / cli.path)`; existing directory ⇒ `IsADirectory`.
   * `canonicalize_existing_prefix`: canonicalize the deepest existing ancestor and
     re-append the rest (the file may be deleted; symlinked directories resolve). The last
     component is never resolved if it is itself a symlink.
   * Strip the toplevel prefix ⇒ `OutsideRepository` on mismatch; empty remainder (the
     toplevel itself) ⇒ `IsADirectory`.
4. Revision range (if given): contains `..` ⇒ `git rev-list --max-count=1 <range> --` must
   succeed; else `git rev-parse --verify -q <rev>^{commit}`. Failure ⇒ `BadRevision`.
5. `query = LogQuery { path: target, follow: !no_follow, line_range, filters }`,
   `show_local = range.is_none()`.
6. History check, only if HEAD is born (`rev-parse --verify -q HEAD`) or a range is given:
   * `-L`: the real `git log -L…` args with `--max-count=1` (git validates the range; e.g.
     "file has only 4 lines" ⇒ `Error::Git`).
   * else the cheap `git log --max-count=1 --format=%H [range] -- :(literal)<path>` (no
     `--follow`, which is slow to start on big repos).
7. Local existence (only if `show_local`): path exists on disk (`symlink_metadata`) or
   `git ls-files -z -- :(literal)<path>` is non-empty.
8. No history and no local existence ⇒ `NoHistory`. Unborn HEAD with a staged/untracked
   file is allowed (timeline = pseudo-entries only).

## Notes
* git runs external subcommands in the caller's cwd without setting `GIT_PREFIX`
  (verified), so the path argument is resolved against `cwd`.
* After discovery all commands use `-C <toplevel>` with repo-relative paths.

## Tests (tests/timeline.rs)
Subdir invocation (`../top.txt`), glob characters in names, every error variant.
