//! Save a version of the file to a new file, without touching the repository.

use std::path::Path;

use crate::app::action::OpResult;
use crate::model::{RevisionId, TimelineEntry};

/// `<stem>@<short-oid>.<ext>` using the file name at that revision.
pub fn default_name(e: &TimelineEntry) -> String {
    let tag = match &e.rev {
        RevisionId::Commit(o) => o.short().to_string(),
        RevisionId::Index => "staged".into(),
        RevisionId::WorkingTree => "worktree".into(),
    };
    let name = String::from_utf8_lossy(e.path.file_name()).into_owned();
    match e.path.extension() {
        Some(ext) => {
            let ext = String::from_utf8_lossy(ext);
            let stem = &name[..name.len() - ext.len() - 1];
            format!("{stem}@{tag}.{ext}")
        }
        None => format!("{name}@{tag}"),
    }
}

pub fn write(bytes: &[u8], dest: &Path, overwrite: bool) -> OpResult {
    if !overwrite && dest.symlink_metadata().is_ok() {
        return OpResult::ExportExists { dest: dest.to_path_buf() };
    }
    if dest.is_dir() {
        return OpResult::Failed { op: "export", error: format!("{} is a directory", dest.display()) };
    }
    match super::atomic_write(dest, bytes) {
        Ok(()) => OpResult::Exported { dest: dest.to_path_buf() },
        Err(e) => OpResult::Failed { op: "export", error: format!("{}: {e}", dest.display()) },
    }
}
