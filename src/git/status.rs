//! Staged / working-tree changes → pseudo-entries above HEAD.

use super::Git;
use crate::error::Result;
use crate::model::{BlobSource, ChangeKind, FileMode, Oid, RepoPath, RevisionId, TimelineEntry};

#[derive(Debug, Default)]
pub struct LocalState {
    pub head_blob: Option<(Oid, FileMode)>,
    pub index_blob: Option<(Oid, FileMode)>,
    pub conflicted: bool,
    pub index_differs: bool,
    pub worktree_differs: bool,
    pub untracked: bool,
    pub worktree_mode: Option<FileMode>,
}

pub fn read_local_state(git: &Git, path: &RepoPath) -> Result<LocalState> {
    let mut st = LocalState::default();
    let spec = path.as_literal_pathspec();

    let out = git.output(&[
        "status".into(),
        "--porcelain=v2".into(),
        "-z".into(),
        "--no-renames".into(),
        "--untracked-files=all".into(),
        "--".into(),
        spec.clone(),
    ])?;
    for rec in out.split(|&b| b == 0).filter(|r| !r.is_empty()) {
        match rec[0] {
            b'1' | b'2' if rec.len() > 4 => {
                st.index_differs |= rec[2] != b'.';
                st.worktree_differs |= rec[3] != b'.';
            }
            b'u' => st.conflicted = true,
            b'?' => {
                st.untracked = true;
                st.worktree_differs = true;
            }
            _ => {}
        }
    }

    let ls = git.output(&["ls-files".into(), "-s".into(), "-z".into(), "--".into(), spec.clone()])?;
    for rec in ls.split(|&b| b == 0).filter(|r| !r.is_empty()) {
        // <mode> <oid> <stage>\t<path>
        let meta = rec.split(|&b| b == b'\t').next().unwrap_or_default();
        let f: Vec<&[u8]> = meta.split(|&b| b == b' ').collect();
        if f.len() == 3 {
            if f[2] == b"0" {
                if let Some(oid) = Oid::parse_bytes(f[1]) {
                    st.index_blob = Some((oid, FileMode::from_octal(f[0])));
                }
            } else {
                st.conflicted = true;
            }
        }
    }

    if let Some(out) = git.probe(&["ls-tree".into(), "-z".into(), "HEAD".into(), "--".into(), path.as_os()])? {
        st.head_blob = parse_ls_tree(&out);
    }

    let fs_path = path.to_fs_path(git.dir());
    st.worktree_mode = std::fs::symlink_metadata(&fs_path).ok().map(|m| mode_of(&m));
    Ok(st)
}

/// First record of `git ls-tree -z`: `<mode> <type> <oid>\t<path>`.
pub fn parse_ls_tree(out: &[u8]) -> Option<(Oid, FileMode)> {
    let rec = out.split(|&b| b == 0).next()?;
    let meta = rec.split(|&b| b == b'\t').next()?;
    let f: Vec<&[u8]> = meta.split(|&b| b == b' ').collect();
    if f.len() != 3 {
        return None;
    }
    Some((Oid::parse_bytes(f[2])?, FileMode::from_octal(f[0])))
}

pub fn mode_of(m: &std::fs::Metadata) -> FileMode {
    if m.file_type().is_symlink() {
        return FileMode::Symlink;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if m.permissions().mode() & 0o111 != 0 {
            return FileMode::Executable;
        }
    }
    FileMode::Regular
}

fn src(b: &Option<(Oid, FileMode)>) -> (BlobSource, FileMode) {
    match b {
        Some((o, m)) => (BlobSource::Object(o.clone()), *m),
        None => (BlobSource::Absent, FileMode::Absent),
    }
}

/// Pseudo-entries, newest first: WorkingTree, then Index.
pub fn local_entries(st: &LocalState, path: &RepoPath) -> Vec<TimelineEntry> {
    let mut v = Vec::new();
    let (head, head_mode) = src(&st.head_blob);
    let (index, index_mode) = src(&st.index_blob);
    let (wt, wt_mode) = match st.worktree_mode {
        Some(m) => (BlobSource::WorkingTree(path.clone()), m),
        None => (BlobSource::Absent, FileMode::Absent),
    };
    let entry = |rev, change, blob, mode, prev_blob, prev_mode, conflicted| TimelineEntry {
        rev,
        meta: None,
        path: path.clone(),
        change,
        mode,
        blob,
        prev_blob,
        prev_mode,
        prev_path: path.clone(),
        conflicted,
    };

    if st.conflicted {
        v.push(entry(RevisionId::WorkingTree, ChangeKind::Modified, wt, wt_mode, head, head_mode, true));
        return v;
    }
    if st.worktree_differs {
        let (prev, prev_mode) = if st.untracked {
            (BlobSource::Absent, FileMode::Absent)
        } else if st.index_blob.is_some() {
            (index.clone(), index_mode)
        } else {
            (head.clone(), head_mode)
        };
        let change = if st.untracked {
            ChangeKind::Untracked
        } else if wt == BlobSource::Absent {
            ChangeKind::Deleted
        } else if prev == BlobSource::Absent {
            ChangeKind::Added
        } else {
            ChangeKind::Modified
        };
        v.push(entry(RevisionId::WorkingTree, change, wt, wt_mode, prev, prev_mode, false));
    }
    if st.index_differs {
        let change = match (&st.head_blob, &st.index_blob) {
            (None, _) => ChangeKind::Added,
            (_, None) => ChangeKind::Deleted,
            _ => ChangeKind::Modified,
        };
        v.push(entry(RevisionId::Index, change, index, index_mode, head, head_mode, false));
    }
    v
}
