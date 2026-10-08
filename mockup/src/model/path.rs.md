# `src/model/path.rs`

## Purpose
`RepoPath`: a path **relative to the repository toplevel**, stored as the exact bytes git
uses. Keeps OS paths (relative to cwd) and git paths apart, and survives non-UTF-8 names.

## Contents

```rust
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Default)]
pub struct RepoPath(bstr::BString);

impl RepoPath {
    /// Bytes as emitted by git with `-z` (no quoting, '/' separators).
    pub fn from_git_bytes(b: &[u8]) -> Self;
    pub fn as_bytes(&self) -> &[u8];
    /// For `-- <pathspec>`: ":(literal)" + path, so `*`, `?`, `[` are not globs.
    pub fn as_literal_pathspec(&self) -> OsString;
    /// Plain path argument (blame, ls-tree, `-L…:path`), no pathspec magic.
    pub fn as_os(&self) -> OsString;
    /// toplevel.join(path).
    pub fn to_fs_path(&self, toplevel: &Path) -> PathBuf;
    /// Last component.
    pub fn file_name(&self) -> &[u8];
    /// Extension of the file name; a leading dot is not one (`.env` → None, `Makefile` → None).
    pub fn extension(&self) -> Option<&[u8]>;
    /// Lossy UTF-8, for UI and messages only.
    pub fn display(&self) -> Cow<'_, str>;
}
```

## Specs
* Never starts with `/` or `./`, no `..`, `/` separators.
* Built only by `git/repo.rs` (from the CLI argument) and git output parsers.
* Bytes → `OsString`: lossless on Unix (`OsStringExt::from_vec`), lossy UTF-8 elsewhere.
* `-L<a>,<b>:<path>`, `git blame` and `git ls-tree` take plain paths (`as_os`), not pathspec
  magic.

## Tests
File name / extension edge cases (`.env`, `Makefile`), literal pathspec prefix.
