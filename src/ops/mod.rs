//! User-triggered side effects on the outside world. Never touch the index, HEAD or refs.

pub mod clipboard;
pub mod export;
pub mod restore;

use std::io::Write;
use std::path::Path;

/// Write to `<dest>.git-timeline.tmp` next to the destination, fsync, rename over it.
pub(crate) fn atomic_write(dest: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = dest
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?;
    let mut tmp_name = name.to_os_string();
    tmp_name.push(".git-timeline.tmp");
    let tmp = dest.with_file_name(tmp_name);
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        Ok::<_, std::io::Error>(f)
    })();
    match result {
        Ok(f) => {
            // keep the destination's permissions when replacing a file
            if let Ok(meta) = std::fs::metadata(dest) {
                let _ = f.set_permissions(meta.permissions());
            }
            std::fs::rename(&tmp, dest)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}
