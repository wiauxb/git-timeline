use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, TimeZone};

use super::Git;
use crate::error::{Error, Result};
use crate::model::{BlameCommit, BlameData, BlameLine, Oid, RepoPath, RevisionId, TimelineEntry};

/// `staged` = bytes of the staged blob, required for the Index row.
pub fn blame(git: &Git, entry: &TimelineEntry, staged: Option<&[u8]>) -> Result<BlameData> {
    let mut args: Vec<OsString> = vec!["blame".into(), "--porcelain".into()];
    let out = match &entry.rev {
        RevisionId::Commit(oid) => {
            args.extend([oid.as_str().into(), "--".into(), entry.path.as_os()]);
            git.output(&args)?
        }
        RevisionId::WorkingTree => {
            args.extend(["--".into(), entry.path.as_os()]);
            git.output(&args)?
        }
        RevisionId::Index => {
            args.extend(["--contents".into(), "-".into(), "--".into(), entry.path.as_os()]);
            git.output_with_stdin(&args, staged.unwrap_or_default())?
        }
    };
    parse_porcelain(&out)
}

#[derive(Default)]
struct Pending {
    author: String,
    time: i64,
    tz: String,
    summary: String,
    boundary: bool,
}

pub fn parse_porcelain(out: &[u8]) -> Result<BlameData> {
    let err = || Error::Parse { context: "blame porcelain" };
    let mut lines: Vec<(usize, BlameLine)> = Vec::new();
    let mut commits: HashMap<Oid, Arc<BlameCommit>> = HashMap::new();
    let mut filenames: HashMap<Oid, RepoPath> = HashMap::new();

    let mut current: Option<(Oid, usize, usize)> = None;
    let mut pending = Pending::default();

    for raw in out.split(|&b| b == b'\n') {
        if raw.is_empty() {
            continue;
        }
        if raw[0] == b'\t' {
            // content line: closes the current record
            let (oid, orig, fin) = current.take().ok_or_else(err)?;
            if !commits.contains_key(&oid) {
                commits.insert(
                    oid.clone(),
                    Arc::new(BlameCommit {
                        oid: oid.clone(),
                        author: std::mem::take(&mut pending.author),
                        author_time: to_time(pending.time, &pending.tz),
                        summary: std::mem::take(&mut pending.summary),
                        boundary: pending.boundary,
                    }),
                );
            }
            pending = Pending::default();
            let orig_path = filenames.get(&oid).cloned().unwrap_or_default();
            lines.push((
                fin,
                BlameLine { commit: oid, orig_path, orig_line: orig, group_start: false },
            ));
            continue;
        }
        if current.is_none() {
            let s = std::str::from_utf8(raw).map_err(|_| err())?;
            let mut f = s.split(' ');
            let oid = Oid::parse(f.next().ok_or_else(err)?).ok_or_else(err)?;
            let orig: usize = f.next().and_then(|x| x.parse().ok()).ok_or_else(err)?;
            let fin: usize = f.next().and_then(|x| x.parse().ok()).ok_or_else(err)?;
            current = Some((oid, orig.saturating_sub(1), fin.saturating_sub(1)));
            continue;
        }
        let (key, val) = match raw.iter().position(|&b| b == b' ') {
            Some(i) => (&raw[..i], &raw[i + 1..]),
            None => (raw, &b""[..]),
        };
        let val_s = || String::from_utf8_lossy(val).into_owned();
        match key {
            b"author" => pending.author = val_s(),
            b"author-time" => pending.time = val_s().parse().unwrap_or(0),
            b"author-tz" => pending.tz = val_s(),
            b"summary" => pending.summary = val_s(),
            b"boundary" => pending.boundary = true,
            b"filename" => {
                let oid = current.as_ref().unwrap().0.clone();
                filenames.insert(oid, RepoPath::from_git_bytes(val));
            }
            _ => {}
        }
    }

    lines.sort_by_key(|(fin, _)| *fin);
    let mut out_lines: Vec<BlameLine> = lines.into_iter().map(|(_, l)| l).collect();
    for i in 0..out_lines.len() {
        out_lines[i].group_start = i == 0 || out_lines[i - 1].commit != out_lines[i].commit;
    }
    Ok(BlameData { lines: out_lines, commits })
}

fn to_time(unix: i64, tz: &str) -> DateTime<FixedOffset> {
    // tz like "+0200" / "-0530"
    let secs = if tz.len() == 5 {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        let h: i32 = tz[1..3].parse().unwrap_or(0);
        let m: i32 = tz[3..5].parse().unwrap_or(0);
        sign * (h * 3600 + m * 60)
    } else {
        0
    };
    let off = FixedOffset::east_opt(secs).unwrap_or(FixedOffset::east_opt(0).unwrap());
    off.timestamp_opt(unix, 0).single().unwrap_or_else(|| off.timestamp_opt(0, 0).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let a = "1111111111111111111111111111111111111111";
        let b = "2222222222222222222222222222222222222222";
        let out = format!(
            "{a} 1 1 2\nauthor Alice\nauthor-time 1700000000\nauthor-tz +0100\nsummary first\nfilename f.txt\n\tx\n\
             {a} 2 2\nfilename f.txt\n\ty\n\
             {b} 3 3 1\nauthor Bob\nauthor-time 1700000100\nauthor-tz -0500\nsummary second\nboundary\nfilename f.txt\n\tz\n"
        );
        let d = parse_porcelain(out.as_bytes()).unwrap();
        assert_eq!(d.lines.len(), 3);
        assert!(d.lines[0].group_start && !d.lines[1].group_start && d.lines[2].group_start);
        let bc = &d.commits[&Oid::parse(b).unwrap()];
        assert_eq!(bc.author, "Bob");
        assert!(bc.boundary);
        assert_eq!(d.commits[&Oid::parse(a).unwrap()].summary, "first");
    }
}
