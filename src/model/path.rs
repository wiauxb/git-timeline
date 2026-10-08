use std::borrow::Cow;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use bstr::{BString, ByteSlice};

/// A path relative to the repository toplevel, stored as the exact bytes git uses.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Default)]
pub struct RepoPath(BString);

impl RepoPath {
    /// Bytes as emitted by git with `-z` (no quoting, '/' separators).
    pub fn from_git_bytes(b: &[u8]) -> Self {
        RepoPath(BString::from(b))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// For `-- <pathspec>`: `:(literal)` so `*`, `?`, `[` in names are not globs.
    pub fn as_literal_pathspec(&self) -> OsString {
        let mut v = b":(literal)".to_vec();
        v.extend_from_slice(self.as_bytes());
        bytes_to_os(v)
    }

    /// Plain path argument (blame, `-L…:path`), no pathspec magic.
    pub fn as_os(&self) -> OsString {
        bytes_to_os(self.as_bytes().to_vec())
    }

    pub fn to_fs_path(&self, toplevel: &Path) -> PathBuf {
        toplevel.join(PathBuf::from(self.as_os()))
    }

    pub fn file_name(&self) -> &[u8] {
        match self.0.rfind_byte(b'/') {
            Some(i) => &self.0[i + 1..],
            None => self.as_bytes(),
        }
    }

    /// Extension of the file name; a leading dot (`.env`) is not an extension.
    pub fn extension(&self) -> Option<&[u8]> {
        let name = self.file_name();
        match name.rfind_byte(b'.') {
            Some(0) | None => None,
            Some(i) => Some(&name[i + 1..]),
        }
    }

    pub fn display(&self) -> Cow<'_, str> {
        self.0.to_str_lossy()
    }
}

#[cfg(unix)]
fn bytes_to_os(v: Vec<u8>) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(v)
}

#[cfg(not(unix))]
fn bytes_to_os(v: Vec<u8>) -> OsString {
    OsString::from(String::from_utf8_lossy(&v).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_extensions() {
        let p = RepoPath::from_git_bytes(b"src/main.rs");
        assert_eq!(p.file_name(), b"main.rs");
        assert_eq!(p.extension(), Some(&b"rs"[..]));
        assert_eq!(RepoPath::from_git_bytes(b".env").extension(), None);
        assert_eq!(RepoPath::from_git_bytes(b"Makefile").extension(), None);
        assert_eq!(
            RepoPath::from_git_bytes(b"a*b").as_literal_pathspec(),
            OsString::from(":(literal)a*b")
        );
    }
}
