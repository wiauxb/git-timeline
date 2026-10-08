use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};

use super::{Oid, RepoPath};

#[derive(Debug, Default)]
pub struct BlameData {
    /// One per content line, same order.
    pub lines: Vec<BlameLine>,
    pub commits: HashMap<Oid, Arc<BlameCommit>>,
}

#[derive(Debug, Clone)]
pub struct BlameLine {
    /// Null oid = not committed yet.
    pub commit: Oid,
    pub orig_path: RepoPath,
    pub orig_line: usize,
    /// First line of a run of lines from the same commit.
    pub group_start: bool,
}

#[derive(Debug)]
pub struct BlameCommit {
    pub oid: Oid,
    pub author: String,
    pub author_time: DateTime<FixedOffset>,
    pub summary: String,
    pub boundary: bool,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum BlameKey {
    Commit { oid: Oid, path: RepoPath },
    /// Index / WorkingTree rows: keyed by the exact bytes being blamed.
    Local { rev: super::RevisionId, content: super::ContentKey },
}
