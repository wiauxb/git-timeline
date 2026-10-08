# `src/model/revision.rs`

## Purpose
Describe **one row of the timeline**: which revision, the file's path and blob at that
revision, and how the file changed there.

## Contents

```rust
/// Full hex object id (40 chars SHA-1 or 64 chars SHA-256), validated on construction.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Oid(Box<str>);
impl Oid {
    pub fn parse(hex: &str) -> Option<Oid>;
    pub fn short(&self) -> &str;      // first 7 chars (UI). Copy uses the full id.
    pub fn is_null(&self) -> bool;    // all zeros (git's "no object")
}

/// Identity of a timeline row.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RevisionId {
    WorkingTree,          // file on disk, if different from the index
    Index,                // staged version, if different from HEAD
    Commit(Oid),
}

pub struct Signature { pub name: String, pub email: String,
                       pub time: chrono::DateTime<chrono::FixedOffset> }

pub struct CommitMeta {
    pub oid: Oid,
    pub parents: Vec<Oid>,            // len > 1 ⇒ merge
    pub author: Signature,
    pub committer: Signature,
    pub subject: String,              // %s; full body fetched lazily (git/show.rs)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileMode { Regular, Executable, Symlink, Submodule /*gitlink*/, Absent }
// from octal modes 100644 / 100755 / 120000 / 160000 / 000000

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,                                   // row shows an "absent" file
    Renamed { from: RepoPath, similarity: u8 },
    Copied  { from: RepoPath, similarity: u8 },
    TypeChanged,                               // e.g. regular → symlink
    Merge,                                     // merge without raw lines (see below)
    Untracked,                                 // WorkingTree row, file not in index
}

/// Where the bytes of a version can be read from.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum BlobSource {
    Object(Oid),                                // a blob in the object db
    WorkingTree(RepoPath),                      // read from disk
    Absent,                                     // file does not exist there
    Unresolved { commit: Oid, path: RepoPath }, // resolve with rev-parse <commit>:<path>
}

pub struct TimelineEntry {
    pub rev: RevisionId,
    pub meta: Option<Arc<CommitMeta>>,  // None for WorkingTree / Index
    pub path: RepoPath,                 // file path *in this revision*
    pub change: ChangeKind,
    pub mode: FileMode,
    pub blob: BlobSource,               // the version AT this revision
    pub prev_blob: BlobSource,          // the version just BEFORE this revision
    pub prev_path: RepoPath,            // path of prev_blob (differs on rename)
    pub conflicted: bool,               // WorkingTree row while index has stages 1-3
}

pub struct Timeline {
    pub entries: Vec<TimelineEntry>,    // index 0 = newest (pseudo-entries first)
    pub complete: bool,                 // false while still streaming
}
impl Timeline {
    pub fn position_of(&self, rev: &RevisionId) -> Option<usize>;
    pub fn local_count(&self) -> usize; // number of pseudo-entries at the top (0..=2)
}
```

## Specs
* **`prev_blob` is the key to "diff vs previous"**: for commits it is the *old blob id*
  printed by `git log --raw` (`:<m1> <m2> <old> <new> <status>`), i.e. the version in the
  commit's parent — correct even when the list is filtered by author/-L/range.
  * `Added` ⇒ `prev_blob = Absent`.
  * `Merge` (no raw line; git omits raw output for merges by default) ⇒
    `blob = Unresolved{commit, path}`, `prev_blob = Unresolved{first_parent, path}`.
  * `Index` row ⇒ `prev_blob` = HEAD blob. `WorkingTree` row ⇒ `prev_blob` = index blob
    (or HEAD blob when the index equals HEAD).
* `RevisionId` is the **stable identity** used for selection and the compare mark, so a
  reload that inserts/removes pseudo-entries does not shift the selection.
* Null oids in raw output (`0000…`) map to `BlobSource::Absent`.
