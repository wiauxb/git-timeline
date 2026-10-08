# `src/model/revision.rs`

## Purpose
Describe **one row of the timeline**: which revision, the file's path and blob there,
the version just before it, and how the file changed.

## Contents

```rust
/// Full hex object id (40 chars SHA-1 or 64 chars SHA-256, lowercase hex).
#[derive(Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Oid(Box<str>);
impl Oid {
    pub fn parse(hex: &str) -> Option<Oid>;
    pub fn parse_bytes(hex: &[u8]) -> Option<Oid>;
    pub fn as_str(&self) -> &str;
    pub fn short(&self) -> &str;     // first 7 chars (UI); copy/export use the full id
    pub fn is_null(&self) -> bool;   // all zeros: git's "no object" / uncommitted
}
impl Display for Oid;

/// Stable identity of a row (selection and mark survive reloads).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RevisionId { WorkingTree, Index, Commit(Oid) }

pub struct Signature { pub name: String, pub email: String, pub time: DateTime<FixedOffset> }

pub struct CommitMeta {
    pub oid: Oid,
    pub parents: Vec<Oid>,     // len > 1 ⇒ merge
    pub author: Signature,
    pub committer: Signature,
    pub subject: String,       // %s; full body fetched lazily by git/show.rs
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FileMode { Regular, Executable, Symlink, Submodule, Absent }
impl FileMode {
    /// 100755 → Executable, 120000 → Symlink, 160000 → Submodule, 000000 → Absent,
    /// anything else → Regular.
    pub fn from_octal(s: &[u8]) -> FileMode;
}

pub enum ChangeKind {
    Added, Modified, Deleted,
    Renamed { from: RepoPath, similarity: u8 },
    Copied  { from: RepoPath, similarity: u8 },
    TypeChanged,
    Merge,       // commit with > 1 parent
    Untracked,   // WorkingTree row, file not in the index
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum BlobSource {
    Object(Oid),                                // blob in the object db
    WorkingTree(RepoPath),                      // read from disk
    Absent,                                     // file doesn't exist there
    Unresolved { commit: Oid, path: RepoPath }, // resolved via ls-tree by git/blob.rs
}

#[derive(Clone, Debug)]
pub struct TimelineEntry {
    pub rev: RevisionId,
    pub meta: Option<Arc<CommitMeta>>,  // None for WorkingTree / Index
    pub path: RepoPath,                 // file path *in this revision*
    pub change: ChangeKind,
    pub mode: FileMode,
    pub blob: BlobSource,               // the version AT this revision
    pub prev_blob: BlobSource,          // the version just BEFORE it
    pub prev_mode: FileMode,
    pub prev_path: RepoPath,            // path of prev_blob (differs on rename/copy)
    pub conflicted: bool,               // WorkingTree row during a merge conflict
}
impl TimelineEntry {
    pub fn oid(&self) -> Option<&Oid>;  // commits only
    pub fn is_local(&self) -> bool;     // WorkingTree / Index
    pub fn label(&self) -> String;      // "Working tree" | "Staged" | short oid
}

#[derive(Default, Debug)]
pub struct Timeline {
    pub entries: Vec<TimelineEntry>,    // index 0 = newest; pseudo-entries first
    pub complete: bool,                 // false while git log is still streaming
}
impl Timeline {
    pub fn position_of(&self, rev: &RevisionId) -> Option<usize>;
    pub fn local_count(&self) -> usize;               // leading pseudo-entries (0..=2)
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn set_local(&mut self, local: Vec<TimelineEntry>);  // replace leading pseudo-entries
}
```

## Specs
* **`prev_blob` drives "diff vs previous"**: for commits it is the *old blob id* of the
  `git log --raw` record, i.e. the version in the (first) parent. That stays correct when the
  list is filtered by author / `-L` / range.
  * `Added` ⇒ `prev_blob = Absent`; `Deleted` ⇒ `blob = Absent`.
  * Merges (logged with `--diff-merges=first-parent`) carry a raw record against the first
    parent ⇒ normal `Object` blobs, `change = Merge`.
  * Commit without any raw record (fallback, should not happen) ⇒ `blob = Unresolved{commit,
    path}`, `prev_blob = Unresolved{first parent, path}` (or `Absent` for a root commit).
  * `Index` row ⇒ `prev_blob` = HEAD blob. `WorkingTree` row ⇒ `prev_blob` = index blob
    (HEAD blob if the file isn't in the index; `Absent` if untracked).
* Null oids in raw output map to `BlobSource::Absent`.
