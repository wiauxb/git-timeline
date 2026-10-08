# `src/model/path.rs`

## Purpose
`RepoPath`: a path **relative to the repository toplevel**, stored as the exact bytes git
uses. Prevents confusion between OS paths (relative to cwd) and git paths, and survives
non-UTF-8 file names.

## Contents

```rust
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct RepoPath(bstr::BString);

impl RepoPath {
    /// Bytes as emitted by git with `-z` (no quoting, '/' separators).
    pub fn from_git_bytes(b: &[u8]) -> Self;
    pub fn as_bytes(&self) -> &[u8];
    /// For `-- <pathspec>`: prefixes ":(literal)" so `*`, `?`, `[` in names are not globs.
    pub fn as_literal_pathspec(&self) -> std::ffi::OsString;
    /// For `<rev>:<path>` object names (used in rev-parse fallback).
    pub fn as_object_suffix(&self) -> std::ffi::OsString;
    /// toplevel.join(self) as an OS path (Unix: from raw bytes; Windows: lossy UTF-8).
    pub fn to_fs_path(&self, toplevel: &std::path::Path) -> std::path::PathBuf;
    pub fn file_name(&self) -> &[u8];
    pub fn extension(&self) -> Option<&[u8]>;
    pub fn display(&self) -> std::borrow::Cow<'_, str>;   // lossy, for UI only
}
```

## Specs
* Never starts with `/` or `./`, never contains `..` components, uses `/` separators.
* Constructed only by `git/repo.rs` (from the CLI argument) and git output parsers.
* `-L<a>,<b>:<path>` does **not** accept pathspec magic, so `git/log.rs` uses the raw
  bytes there, not `as_literal_pathspec`.
