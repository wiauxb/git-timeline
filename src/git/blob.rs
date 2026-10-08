//! Read the bytes of a version through a persistent `git cat-file --batch-command`.
//!
//! Known limitation: working-tree bytes are read raw (no `.gitattributes` filters).

use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::time::UNIX_EPOCH;

use super::{status, Git};
use crate::error::{Error, Result};
use crate::model::{
    classify, BlobSource, ContentKey, FileContent, FileMode, Oid, RepoPath, MAX_BLOB_SIZE,
};

pub struct BlobReader {
    git: Git,
    batch: Option<CatFileBatch>,
    limit: u64,
}

impl BlobReader {
    pub fn new(git: Git) -> Self {
        BlobReader { git, batch: None, limit: MAX_BLOB_SIZE }
    }

    #[doc(hidden)]
    pub fn with_limit(git: Git, limit: u64) -> Self {
        BlobReader { git, batch: None, limit }
    }

    pub fn read(&mut self, src: &BlobSource, mode: FileMode) -> Result<(ContentKey, FileContent)> {
        match src {
            BlobSource::Absent => Ok((ContentKey::Absent, FileContent::Absent)),
            BlobSource::Object(oid) => {
                let key = ContentKey::Object(oid.clone());
                if mode == FileMode::Submodule {
                    return Ok((key, FileContent::Submodule { commit: oid.clone() }));
                }
                let content = self.with_batch(|b, limit| {
                    let size = b.info(oid)?.ok_or(Error::Parse { context: "missing object" })?;
                    if size > limit {
                        return Ok(FileContent::TooLarge { size, limit });
                    }
                    Ok(classify(b.contents(oid)?, mode))
                })?;
                Ok((key, content))
            }
            BlobSource::WorkingTree(path) => self.read_worktree(path),
            BlobSource::Unresolved { commit, path } => {
                let (src, mode) = self.resolve(commit, path)?;
                self.read(&src, mode)
            }
        }
    }

    /// `BlobSource::Unresolved` → Object / Absent, with its mode.
    pub fn resolve(&mut self, commit: &Oid, path: &RepoPath) -> Result<(BlobSource, FileMode)> {
        let out = self.git.output(&[
            "ls-tree".into(),
            "-z".into(),
            commit.as_str().into(),
            "--".into(),
            path.as_os(),
        ])?;
        Ok(match status::parse_ls_tree(&out) {
            Some((oid, mode)) => (BlobSource::Object(oid), mode),
            None => (BlobSource::Absent, FileMode::Absent),
        })
    }

    fn read_worktree(&mut self, path: &RepoPath) -> Result<(ContentKey, FileContent)> {
        let fs_path = path.to_fs_path(self.git.dir());
        let meta = match std::fs::symlink_metadata(&fs_path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok((ContentKey::Absent, FileContent::Absent))
            }
            Err(e) => return Err(e.into()),
        };
        let mtime_ns = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i128)
            .unwrap_or(0);
        let key = ContentKey::WorkingTree { path: path.clone(), mtime_ns, len: meta.len() };
        let mode = status::mode_of(&meta);
        if mode == FileMode::Symlink {
            let target = std::fs::read_link(&fs_path)?;
            return Ok((key, classify(target.into_os_string().into_encoded_bytes(), mode)));
        }
        if meta.len() > self.limit {
            return Ok((key, FileContent::TooLarge { size: meta.len(), limit: self.limit }));
        }
        Ok((key, classify(std::fs::read(&fs_path)?, mode)))
    }

    fn with_batch<T>(&mut self, f: impl Fn(&mut CatFileBatch, u64) -> Result<T>) -> Result<T> {
        for attempt in 0..2 {
            if self.batch.is_none() {
                self.batch = Some(CatFileBatch::spawn(&self.git)?);
            }
            match f(self.batch.as_mut().unwrap(), self.limit) {
                Err(Error::Io(e)) if attempt == 0 => {
                    let _ = e;
                    self.batch = None; // process died: restart once
                }
                r => return r,
            }
        }
        unreachable!()
    }
}

struct CatFileBatch {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl CatFileBatch {
    fn spawn(git: &Git) -> Result<Self> {
        let mut child = git
            .command(["cat-file", "--batch-command", "--buffer"])
            .stdin(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound { Error::GitNotFound } else { e.into() }
            })?;
        let stdin = BufWriter::new(child.stdin.take().expect("piped"));
        let stdout = BufReader::new(child.stdout.take().expect("piped"));
        Ok(CatFileBatch { child, stdin, stdout })
    }

    fn header(&mut self) -> Result<Option<u64>> {
        let mut line = String::new();
        if self.stdout.read_line(&mut line)? == 0 {
            return Err(Error::Io(std::io::ErrorKind::UnexpectedEof.into()));
        }
        let f: Vec<&str> = line.trim_end().split(' ').collect();
        match f.as_slice() {
            [_, "missing"] => Ok(None),
            [_, _, size] => Ok(Some(size.parse().map_err(|_| Error::Parse { context: "cat-file size" })?)),
            _ => Err(Error::Parse { context: "cat-file header" }),
        }
    }

    fn info(&mut self, oid: &Oid) -> Result<Option<u64>> {
        writeln!(self.stdin, "info {oid}\nflush")?;
        self.stdin.flush()?;
        self.header()
    }

    fn contents(&mut self, oid: &Oid) -> Result<Vec<u8>> {
        writeln!(self.stdin, "contents {oid}\nflush")?;
        self.stdin.flush()?;
        let size = self.header()?.ok_or(Error::Parse { context: "missing object" })?;
        let mut buf = vec![0u8; size as usize + 1]; // + trailing LF
        self.stdout.read_exact(&mut buf)?;
        buf.pop();
        Ok(buf)
    }
}

impl Drop for CatFileBatch {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
