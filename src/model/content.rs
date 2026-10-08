use std::borrow::Cow;
use std::sync::Arc;

use bstr::BString;

use super::{FileMode, Oid, RepoPath};

pub const MAX_BLOB_SIZE: u64 = 50 * 1024 * 1024;

/// Cache key for content. Working-tree content includes mtime+len so an edit on disk
/// invalidates the cache naturally.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ContentKey {
    Object(Oid),
    WorkingTree { path: RepoPath, mtime_ns: i128, len: u64 },
    Absent,
}

#[derive(Debug)]
pub enum FileContent {
    Text(TextContent),
    Binary { bytes: Arc<[u8]> },
    TooLarge { size: u64, limit: u64 },
    Symlink { target: BString },
    Submodule { commit: Oid },
    Absent,
}

impl FileContent {
    pub fn as_text(&self) -> Option<&TextContent> {
        match self {
            FileContent::Text(t) => Some(t),
            _ => None,
        }
    }

    /// Raw bytes when available (export / restore).
    pub fn bytes(&self) -> Option<Arc<[u8]>> {
        match self {
            FileContent::Text(t) => Some(t.bytes.clone()),
            FileContent::Binary { bytes } => Some(bytes.clone()),
            FileContent::Symlink { target } => Some(Arc::from(target.as_slice())),
            _ => None,
        }
    }

    pub fn approx_size(&self) -> usize {
        match self {
            FileContent::Text(t) => t.bytes.len() + t.line_starts.len() * 4,
            FileContent::Binary { bytes } => bytes.len(),
            FileContent::Symlink { target } => target.len(),
            _ => 0,
        }
    }
}

#[derive(Debug)]
pub struct TextContent {
    bytes: Arc<[u8]>,
    line_starts: Vec<u32>,
    pub ends_with_newline: bool,
    pub has_crlf: bool,
    pub invalid_utf8: bool,
}

impl TextContent {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut line_starts = Vec::new();
        if !bytes.is_empty() {
            line_starts.push(0);
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'\n' && i + 1 < bytes.len() {
                    line_starts.push((i + 1) as u32);
                }
            }
        }
        let ends_with_newline = bytes.last() == Some(&b'\n');
        let has_crlf = bytes.windows(2).any(|w| w == b"\r\n");
        let invalid_utf8 = std::str::from_utf8(&bytes).is_err();
        TextContent { bytes: bytes.into(), line_starts, ends_with_newline, has_crlf, invalid_utf8 }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Line bytes without the trailing `\n` / `\r\n`.
    pub fn line_bytes(&self, i: usize) -> &[u8] {
        let start = self.line_starts[i] as usize;
        let end = self
            .line_starts
            .get(i + 1)
            .map(|&e| e as usize)
            .unwrap_or(self.bytes.len());
        let mut l = &self.bytes[start..end];
        if l.last() == Some(&b'\n') {
            l = &l[..l.len() - 1];
        }
        if l.last() == Some(&b'\r') {
            l = &l[..l.len() - 1];
        }
        l
    }

    pub fn line_str(&self, i: usize) -> Cow<'_, str> {
        String::from_utf8_lossy(self.line_bytes(i))
    }

    pub fn lines(&self) -> impl Iterator<Item = &[u8]> {
        (0..self.line_count()).map(move |i| self.line_bytes(i))
    }

    pub fn bytes(&self) -> &Arc<[u8]> {
        &self.bytes
    }
}

/// git's binary heuristic: a NUL byte in the first 8000 bytes.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(8000)].contains(&0)
}

pub fn classify(bytes: Vec<u8>, mode: FileMode) -> FileContent {
    match mode {
        FileMode::Symlink => FileContent::Symlink { target: BString::from(bytes) },
        _ if is_binary(&bytes) => FileContent::Binary { bytes: bytes.into() },
        _ => FileContent::Text(TextContent::from_bytes(bytes)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines() {
        let t = TextContent::from_bytes(b"a\r\nbb\nc".to_vec());
        assert_eq!(t.line_count(), 3);
        assert_eq!(t.line_bytes(0), b"a");
        assert_eq!(t.line_bytes(1), b"bb");
        assert_eq!(t.line_bytes(2), b"c");
        assert!(t.has_crlf);
        assert!(!t.ends_with_newline);

        let t = TextContent::from_bytes(b"x\n\n".to_vec());
        assert_eq!(t.line_count(), 2);
        assert_eq!(t.line_bytes(1), b"");
        assert!(t.ends_with_newline);

        assert_eq!(TextContent::from_bytes(vec![]).line_count(), 0);
    }

    #[test]
    fn binary() {
        let mut v = vec![b'a'; 9000];
        assert!(!is_binary(&v));
        v[8500] = 0;
        assert!(!is_binary(&v));
        v[10] = 0;
        assert!(is_binary(&v));
    }
}
