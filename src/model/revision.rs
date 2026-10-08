use std::sync::Arc;

use chrono::{DateTime, FixedOffset};

use super::RepoPath;

/// Full hex object id (40 chars SHA-1 or 64 chars SHA-256).
#[derive(Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Oid(Box<str>);

impl Oid {
    pub fn parse(hex: &str) -> Option<Oid> {
        let ok = (hex.len() == 40 || hex.len() == 64)
            && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        ok.then(|| Oid(hex.into()))
    }

    pub fn parse_bytes(hex: &[u8]) -> Option<Oid> {
        std::str::from_utf8(hex).ok().and_then(Oid::parse)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn short(&self) -> &str {
        &self.0[..7]
    }

    pub fn is_null(&self) -> bool {
        self.0.bytes().all(|b| b == b'0')
    }
}

impl std::fmt::Display for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identity of a timeline row; stable across reloads.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RevisionId {
    WorkingTree,
    Index,
    Commit(Oid),
}

#[derive(Clone, Debug)]
pub struct Signature {
    pub name: String,
    pub email: String,
    pub time: DateTime<FixedOffset>,
}

#[derive(Clone, Debug)]
pub struct CommitMeta {
    pub oid: Oid,
    pub parents: Vec<Oid>,
    pub author: Signature,
    pub committer: Signature,
    pub subject: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FileMode {
    Regular,
    Executable,
    Symlink,
    Submodule,
    Absent,
}

impl FileMode {
    pub fn from_octal(s: &[u8]) -> FileMode {
        match s {
            b"100755" => FileMode::Executable,
            b"120000" => FileMode::Symlink,
            b"160000" => FileMode::Submodule,
            b"000000" => FileMode::Absent,
            _ => FileMode::Regular,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed { from: RepoPath, similarity: u8 },
    Copied { from: RepoPath, similarity: u8 },
    TypeChanged,
    /// Merge without raw output (git prints none for merges by default).
    Merge,
    /// WorkingTree row, file not in the index.
    Untracked,
}

/// Where the bytes of a version can be read from.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum BlobSource {
    Object(Oid),
    WorkingTree(RepoPath),
    Absent,
    /// Resolve with `git rev-parse <commit>:<path>`.
    Unresolved { commit: Oid, path: RepoPath },
}

#[derive(Clone, Debug)]
pub struct TimelineEntry {
    pub rev: RevisionId,
    pub meta: Option<Arc<CommitMeta>>,
    /// File path in this revision.
    pub path: RepoPath,
    pub change: ChangeKind,
    pub mode: FileMode,
    /// The version AT this revision.
    pub blob: BlobSource,
    /// The version just BEFORE this revision (parent / HEAD / index).
    pub prev_blob: BlobSource,
    pub prev_mode: FileMode,
    pub prev_path: RepoPath,
    pub conflicted: bool,
}

impl TimelineEntry {
    pub fn oid(&self) -> Option<&Oid> {
        match &self.rev {
            RevisionId::Commit(o) => Some(o),
            _ => None,
        }
    }

    pub fn is_local(&self) -> bool {
        !matches!(self.rev, RevisionId::Commit(_))
    }

    pub fn label(&self) -> String {
        match &self.rev {
            RevisionId::WorkingTree => "Working tree".into(),
            RevisionId::Index => "Staged".into(),
            RevisionId::Commit(o) => o.short().into(),
        }
    }
}

#[derive(Default, Debug)]
pub struct Timeline {
    /// Index 0 = newest (pseudo-entries first).
    pub entries: Vec<TimelineEntry>,
    pub complete: bool,
}

impl Timeline {
    pub fn position_of(&self, rev: &RevisionId) -> Option<usize> {
        self.entries.iter().position(|e| &e.rev == rev)
    }

    pub fn local_count(&self) -> usize {
        self.entries.iter().take_while(|e| e.is_local()).count()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Replace the pseudo-entries at the top.
    pub fn set_local(&mut self, local: Vec<TimelineEntry>) {
        let n = self.local_count();
        self.entries.splice(0..n, local);
    }
}
