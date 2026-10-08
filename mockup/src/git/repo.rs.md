# `src/git/repo.rs`

## Purpose
Startup discovery: find the repository, normalize the user's path into a `RepoPath`,
validate options against git, and produce an immutable `RepoContext` shared (via `Arc`)
by the whole app.

## Contents

```rust
pub struct RepoContext {
    pub git: Git,
    pub toplevel: PathBuf,           // canonical
    pub invocation_dir: PathBuf,     // cwd at launch (default export directory)
    pub target: RepoPath,            // the file, at the tip of the explored history
    pub query: LogQuery,             // see git/log.rs
    pub show_local: bool,            // pseudo-entries enabled (no revision range given)
    pub git_version: (u32, u32, u32),
    pub object_format: ObjectFormat, // Sha1 | Sha256 (rev-parse --show-object-format)
}

impl RepoContext {
    pub fn discover(cli: &Cli, cwd: PathBuf) -> Result<Self>;
}

pub const MIN_GIT: (u32, u32, u32) = (2, 36, 0);
```

## Discovery algorithm
1. `git --version` → parse `git version X.Y.Z[...]`; spawn NotFound ⇒ `GitNotFound`;
   `< MIN_GIT` ⇒ `GitTooOld`.
2. `git rev-parse --show-toplevel --show-object-format` **run in `cwd`** (not `-C`)
   ⇒ toplevel. Failure ⇒ `NotARepository`. Bare repositories are rejected the same way.
3. Path normalization (file may not exist anymore — it may be deleted):
   * `abs = cwd.join(cli.path)`, then lexical normalization of `.` / `..`.
   * Canonicalize the **deepest existing ancestor** of `abs` and re-append the remainder
     (handles symlinked directories such as `/home → /var/home`).
   * Strip the canonical toplevel prefix ⇒ `OutsideRepository` if it does not match.
   * If `abs` exists and is a directory ⇒ `IsADirectory`.
4. Revision range check (if given):
   * contains `..` ⇒ `git rev-list --max-count=1 <range> --` must succeed
   * otherwise ⇒ `git rev-parse --verify -q <rev>^{commit}`
   * failure ⇒ `BadRevision`.
5. `-L` check: run the real `git log -L…` with `--max-count=1` (cheap) and map failure to
   `Error::Git` (e.g. "file has only 12 lines").
6. Existence check: the path must either exist in the working tree / index **or** have at
   least one commit: `git log --max-count=1 --format=%H [range] -- :(literal)<path>`
   (note: without `--follow`, cheap). Empty ⇒ `NoHistory`.
7. Unborn `HEAD` (fresh repo, no commit): allowed; the timeline will contain only
   pseudo-entries.

## Notes
* All subsequent commands use `git -C <toplevel>` with repo-relative paths, so the
  rest of the code never deals with cwd.
* `RepoContext` is immutable after discovery; reload (`r`) rebuilds the timeline, not the
  context.
