//! Committed part of the timeline, from
//! `git log --follow --raw --no-abbrev -z --format=…`, streamed in chunks.

use std::ffi::OsString;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::DateTime;

use super::Git;
use crate::error::{Error, Result};
use crate::model::{
    BlobSource, ChangeKind, CommitMeta, FileMode, Oid, RepoPath, RevisionId, Signature,
    TimelineEntry,
};

const FORMAT: &str = "--format=%x1e%H%x1f%P%x1f%an%x1f%ae%x1f%aI%x1f%cn%x1f%ce%x1f%cI%x1f%s";
const BATCH: usize = 256;
const FLUSH_EVERY: Duration = Duration::from_millis(50);

#[derive(Clone, Debug)]
pub struct LogQuery {
    pub path: RepoPath,
    pub follow: bool,
    /// `-L` value without `:path`.
    pub line_range: Option<String>,
    pub filters: LogFilters,
}

#[derive(Clone, Debug, Default)]
pub struct LogFilters {
    pub range: Option<String>,
    pub authors: Vec<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

pub fn build_args(q: &LogQuery) -> Vec<OsString> {
    let mut a: Vec<OsString> = vec!["log".into()];
    if let Some(l) = &q.line_range {
        let mut arg = OsString::from(format!("-L{l}:"));
        arg.push(q.path.as_os());
        a.push(arg);
        a.push("--no-patch".into());
    } else {
        if q.follow {
            a.push("--follow".into());
        }
        // Without it, --follow silently drops merges, even those producing a new version.
        a.push("--diff-merges=first-parent".into());
    }
    a.extend(["--raw", "--no-abbrev", "-z", FORMAT].map(OsString::from));
    for au in &q.filters.authors {
        a.push(format!("--author={au}").into());
    }
    if let Some(s) = &q.filters.since {
        a.push(format!("--since={s}").into());
    }
    if let Some(u) = &q.filters.until {
        a.push(format!("--until={u}").into());
    }
    if let Some(r) = &q.filters.range {
        a.push(r.into());
    }
    if q.line_range.is_none() {
        a.push("--".into());
        a.push(q.path.as_literal_pathspec());
    }
    a
}

/// Runs git log and calls `sink` with batches of entries, newest first.
pub fn stream_timeline(
    git: &Git,
    q: &LogQuery,
    cancel: &AtomicBool,
    sink: &mut dyn FnMut(Vec<TimelineEntry>),
) -> Result<()> {
    let mut stream = git.stream(&build_args(q))?;
    let mut parser = LogParser::new(q);
    let mut out = Vec::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut last_flush = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(()); // dropping the stream kills git
        }
        let n = stream.stdout.read(&mut buf)?;
        if n == 0 {
            break;
        }
        parser.feed(&buf[..n], &mut out)?;
        if out.len() >= BATCH || (!out.is_empty() && last_flush.elapsed() >= FLUSH_EVERY) {
            sink(std::mem::take(&mut out));
            last_flush = Instant::now();
        }
    }
    parser.finish(&mut out)?;
    if !out.is_empty() {
        sink(out);
    }
    stream.finish()
}

struct RawRecord {
    old_mode: FileMode,
    new_mode: FileMode,
    old_oid: Option<Oid>,
    new_oid: Option<Oid>,
    status: u8,
    similarity: u8,
    paths: Vec<RepoPath>,
}

impl RawRecord {
    fn wanted_paths(&self) -> usize {
        if matches!(self.status, b'R' | b'C') { 2 } else { 1 }
    }
    fn new_path(&self) -> &RepoPath {
        self.paths.last().expect("complete record")
    }
}

struct PartialEntry {
    meta: CommitMeta,
    raws: Vec<RawRecord>,
}

/// Incremental parser; fed arbitrary byte chunks.
pub struct LogParser {
    buf: Vec<u8>,
    current: Option<PartialEntry>,
    pending_raw: Option<RawRecord>,
    tracked_path: RepoPath,
    line_mode: bool,
}

impl LogParser {
    pub fn new(q: &LogQuery) -> Self {
        LogParser {
            buf: Vec::new(),
            current: None,
            pending_raw: None,
            tracked_path: q.path.clone(),
            line_mode: q.line_range.is_some(),
        }
    }

    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<TimelineEntry>) -> Result<()> {
        self.buf.extend_from_slice(bytes);
        let mut start = 0;
        while let Some(rel) = self.buf[start..].iter().position(|&b| b == 0) {
            let end = start + rel;
            let token = self.buf[start..end].to_vec();
            self.token(&token, out)?;
            start = end + 1;
        }
        self.buf.drain(..start);
        Ok(())
    }

    pub fn finish(mut self, out: &mut Vec<TimelineEntry>) -> Result<()> {
        let rest = std::mem::take(&mut self.buf);
        if !rest.iter().all(|b| b.is_ascii_whitespace()) {
            self.token(&rest, out)?;
        }
        self.flush_entry(out);
        Ok(())
    }

    fn token(&mut self, token: &[u8], out: &mut Vec<TimelineEntry>) -> Result<()> {
        if let Some(raw) = &mut self.pending_raw {
            raw.paths.push(RepoPath::from_git_bytes(token));
            if raw.paths.len() == raw.wanted_paths() {
                let raw = self.pending_raw.take().unwrap();
                match &mut self.current {
                    Some(cur) => cur.raws.push(raw),
                    None => return Err(Error::Parse { context: "raw record before commit" }),
                }
            }
            return Ok(());
        }
        let t = trim_leading_newlines(token);
        if t.is_empty() {
            return Ok(());
        }
        match t[0] {
            0x1e => {
                self.flush_entry(out);
                self.current = Some(PartialEntry { meta: parse_header(&t[1..])?, raws: Vec::new() });
            }
            b':' => self.pending_raw = Some(parse_raw(&t[1..])?),
            _ => return Err(Error::Parse { context: "unexpected token in git log output" }),
        }
        Ok(())
    }

    fn flush_entry(&mut self, out: &mut Vec<TimelineEntry>) {
        let Some(cur) = self.current.take() else { return };
        let meta = Arc::new(cur.meta);
        let oid = meta.oid.clone();

        let raw = if cur.raws.len() > 1 {
            let idx = cur.raws.iter().position(|r| r.new_path() == &self.tracked_path).unwrap_or(0);
            cur.raws.into_iter().nth(idx)
        } else {
            cur.raws.into_iter().next()
        };

        let entry = match raw {
            None => {
                let path = self.tracked_path.clone();
                let first_parent = meta.parents.first().cloned();
                TimelineEntry {
                    rev: RevisionId::Commit(oid.clone()),
                    change: if meta.parents.len() > 1 { ChangeKind::Merge } else { ChangeKind::Modified },
                    mode: FileMode::Regular,
                    blob: BlobSource::Unresolved { commit: oid, path: path.clone() },
                    prev_blob: match first_parent {
                        Some(p) => BlobSource::Unresolved { commit: p, path: path.clone() },
                        None => BlobSource::Absent,
                    },
                    prev_mode: FileMode::Regular,
                    prev_path: path.clone(),
                    path,
                    meta: Some(meta),
                    conflicted: false,
                }
            }
            Some(r) => {
                let path = r.new_path().clone();
                let prev_path = r.paths[0].clone();
                let change = match r.status {
                    _ if meta.parents.len() > 1 => ChangeKind::Merge,
                    b'A' => ChangeKind::Added,
                    b'D' => ChangeKind::Deleted,
                    b'T' => ChangeKind::TypeChanged,
                    b'R' => ChangeKind::Renamed { from: prev_path.clone(), similarity: r.similarity },
                    b'C' => ChangeKind::Copied { from: prev_path.clone(), similarity: r.similarity },
                    _ => ChangeKind::Modified,
                };
                if !self.line_mode {
                    self.tracked_path = prev_path.clone();
                }
                TimelineEntry {
                    rev: RevisionId::Commit(oid),
                    meta: Some(meta),
                    path,
                    change,
                    mode: r.new_mode,
                    blob: r.new_oid.map(BlobSource::Object).unwrap_or(BlobSource::Absent),
                    prev_blob: r.old_oid.map(BlobSource::Object).unwrap_or(BlobSource::Absent),
                    prev_mode: r.old_mode,
                    prev_path,
                    conflicted: false,
                }
            }
        };
        out.push(entry);
    }
}

fn trim_leading_newlines(t: &[u8]) -> &[u8] {
    let n = t.iter().take_while(|&&b| b == b'\n').count();
    &t[n..]
}

fn parse_header(h: &[u8]) -> Result<CommitMeta> {
    let err = || Error::Parse { context: "commit header" };
    let s = String::from_utf8_lossy(h);
    let f: Vec<&str> = s.splitn(9, '\x1f').collect();
    if f.len() != 9 {
        return Err(err());
    }
    let date = |d: &str| DateTime::parse_from_rfc3339(d).map_err(|_| err());
    Ok(CommitMeta {
        oid: Oid::parse(f[0]).ok_or_else(err)?,
        parents: f[1].split_whitespace().filter_map(Oid::parse).collect(),
        author: Signature { name: f[2].into(), email: f[3].into(), time: date(f[4])? },
        committer: Signature { name: f[5].into(), email: f[6].into(), time: date(f[7])? },
        subject: f[8].trim_end_matches('\n').into(),
    })
}

/// `<mode_old> <mode_new> <oid_old> <oid_new> <status>` (after the ':').
fn parse_raw(r: &[u8]) -> Result<RawRecord> {
    let err = || Error::Parse { context: "raw diff record" };
    let mut it = r.split(|&b| b == b' ');
    let old_mode = FileMode::from_octal(it.next().ok_or_else(err)?);
    let new_mode = FileMode::from_octal(it.next().ok_or_else(err)?);
    let old_oid = Oid::parse_bytes(it.next().ok_or_else(err)?).ok_or_else(err)?;
    let new_oid = Oid::parse_bytes(it.next().ok_or_else(err)?).ok_or_else(err)?;
    let status_field = it.next().ok_or_else(err)?;
    let status = *status_field.first().ok_or_else(err)?;
    let similarity = std::str::from_utf8(&status_field[1..]).ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    Ok(RawRecord {
        old_mode,
        new_mode,
        old_oid: (!old_oid.is_null()).then_some(old_oid),
        new_oid: (!new_oid.is_null()).then_some(new_oid),
        status,
        similarity,
        paths: Vec::with_capacity(2),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "1111111111111111111111111111111111111111";
    const H2: &str = "2222222222222222222222222222222222222222";
    const B1: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B2: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const Z: &str = "0000000000000000000000000000000000000000";

    fn header(h: &str, p: &str, s: &str) -> String {
        format!(
            "\x1e{h}\x1f{p}\x1fA\x1fa@x\x1f2024-01-01T00:00:00+01:00\x1fC\x1fc@x\x1f2024-01-01T00:00:00+01:00\x1f{s}\0"
        )
    }

    fn sample() -> Vec<u8> {
        let mut s = String::new();
        s += &header(H2, H1, "move");
        s += &format!("\n:100644 100644 {B1} {B2} R090\0old.txt\0new.txt\0");
        s += &header(H1, "", "add");
        s += &format!("\n:000000 100644 {Z} {B1} A\0old.txt\0");
        s.into_bytes()
    }

    fn query() -> LogQuery {
        LogQuery {
            path: RepoPath::from_git_bytes(b"new.txt"),
            follow: true,
            line_range: None,
            filters: LogFilters::default(),
        }
    }

    #[test]
    fn parses_rename_history() {
        let mut p = LogParser::new(&query());
        let mut out = Vec::new();
        p.feed(&sample(), &mut out).unwrap();
        p.finish(&mut out).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].path.as_bytes(), b"new.txt");
        assert_eq!(out[0].prev_path.as_bytes(), b"old.txt");
        assert!(matches!(out[0].change, ChangeKind::Renamed { similarity: 90, .. }));
        assert_eq!(out[1].path.as_bytes(), b"old.txt");
        assert_eq!(out[1].change, ChangeKind::Added);
        assert_eq!(out[1].prev_blob, BlobSource::Absent);
        assert_eq!(out[0].meta.as_ref().unwrap().subject, "move");
    }

    #[test]
    fn byte_by_byte_is_identical() {
        let mut p = LogParser::new(&query());
        let mut out = Vec::new();
        for b in sample() {
            p.feed(&[b], &mut out).unwrap();
        }
        p.finish(&mut out).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].path.as_bytes(), b"old.txt");
    }

    #[test]
    fn merge_without_raw() {
        let mut p = LogParser::new(&query());
        let mut out = Vec::new();
        p.feed(header(H2, &format!("{H1} {B1}"), "merge").as_bytes(), &mut out).unwrap();
        p.finish(&mut out).unwrap();
        assert_eq!(out[0].change, ChangeKind::Merge);
        assert!(matches!(out[0].blob, BlobSource::Unresolved { .. }));
    }
}
