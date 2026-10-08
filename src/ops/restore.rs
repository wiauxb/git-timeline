//! Replace the working-tree file with a historical version. Does not stage.

use std::path::Path;

use crate::app::action::OpResult;
use crate::model::FileMode;

pub fn restore(bytes: &[u8], mode: FileMode, target: &Path) -> OpResult {
    match restore_inner(bytes, mode, target) {
        Ok(()) => OpResult::Restored { target: target.to_path_buf() },
        Err(e) => OpResult::Failed { op: "restore", error: format!("{}: {e}", target.display()) },
    }
}

fn restore_inner(bytes: &[u8], mode: FileMode, target: &Path) -> std::io::Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match mode {
        FileMode::Symlink => restore_symlink(bytes, target),
        FileMode::Submodule | FileMode::Absent => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "nothing to restore for this kind of entry",
        )),
        FileMode::Regular | FileMode::Executable => {
            // a symlink in the way is replaced, not followed
            if target.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
                std::fs::remove_file(target)?;
            }
            super::atomic_write(target, bytes)?;
            set_exec(target, mode == FileMode::Executable)
        }
    }
}

#[cfg(unix)]
fn restore_symlink(bytes: &[u8], target: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let link = std::ffi::OsStr::from_bytes(bytes);
    if target.symlink_metadata().is_ok() {
        std::fs::remove_file(target)?;
    }
    std::os::unix::fs::symlink(link, target)
}

#[cfg(not(unix))]
fn restore_symlink(_bytes: &[u8], _target: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "symlinks are not supported here"))
}

#[cfg(unix)]
fn set_exec(target: &Path, exec: bool) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(target)?.permissions();
    let mode = perm.mode();
    // give exec where read is granted (like git checkout), or strip all exec bits
    let new = if exec { mode | ((mode & 0o444) >> 2) } else { mode & !0o111 };
    perm.set_mode(new);
    std::fs::set_permissions(target, perm)
}

#[cfg(not(unix))]
fn set_exec(_target: &Path, _exec: bool) -> std::io::Result<()> {
    Ok(())
}
