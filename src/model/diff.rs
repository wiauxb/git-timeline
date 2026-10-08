use std::time::{Duration, Instant};

use similar::{Algorithm, DiffOp};

use super::{ContentKey, TextContent};

pub const CONTEXT: usize = 3;
const DEADLINE: Duration = Duration::from_millis(300);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DiffKey {
    pub old: ContentKey,
    pub new: ContentKey,
}

#[derive(Debug, Default)]
pub struct FileDiff {
    /// Full op list; indices are line numbers in old/new.
    pub ops: Vec<DiffOp>,
    pub hunks: Vec<Hunk>,
    pub added: usize,
    pub removed: usize,
    pub timed_out: bool,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub old_start: usize,
    pub old_len: usize,
    pub new_start: usize,
    pub new_len: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffLine {
    Context { old: usize, new: usize },
    Removed { old: usize },
    Added { new: usize },
}

impl DiffLine {
    pub fn new_line(&self) -> Option<usize> {
        match *self {
            DiffLine::Context { new, .. } | DiffLine::Added { new } => Some(new),
            DiffLine::Removed { .. } => None,
        }
    }
}

/// One displayed row of a unified diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffRow {
    HunkHeader(usize),
    Line(DiffLine),
}

impl FileDiff {
    pub fn rows(&self) -> Vec<DiffRow> {
        let mut rows = Vec::new();
        for (i, h) in self.hunks.iter().enumerate() {
            rows.push(DiffRow::HunkHeader(i));
            rows.extend(h.lines.iter().map(|&l| DiffRow::Line(l)));
        }
        rows
    }
}

/// New-side line shown at `row` (removed lines/headers: next new-side line, else previous).
pub fn source_line_of_row(rows: &[DiffRow], row: usize) -> Option<usize> {
    let new_of = |r: &DiffRow| match r {
        DiffRow::Line(l) => l.new_line(),
        DiffRow::HunkHeader(_) => None,
    };
    rows.get(row..)?
        .iter()
        .find_map(new_of)
        .or_else(|| rows[..row].iter().rev().find_map(new_of))
        .or_else(|| rows.iter().find_map(|r| match r {
            DiffRow::Line(DiffLine::Removed { old }) => Some(*old),
            _ => None,
        }))
}

/// Row showing new-side `line`, or the row with the closest new-side line.
pub fn row_of_source_line(rows: &[DiffRow], line: usize) -> usize {
    let mut best = (usize::MAX, 0);
    for (i, r) in rows.iter().enumerate() {
        if let DiffRow::Line(l) = r {
            if let Some(n) = l.new_line() {
                let d = n.abs_diff(line);
                if d < best.0 {
                    best = (d, i);
                }
            }
        }
    }
    best.1
}

/// `None` means absent (treated as empty).
pub fn compute(old: Option<&TextContent>, new: Option<&TextContent>) -> FileDiff {
    let old_lines: Vec<&[u8]> = old.map(|t| t.lines().collect()).unwrap_or_default();
    let new_lines: Vec<&[u8]> = new.map(|t| t.lines().collect()).unwrap_or_default();

    let deadline = Instant::now() + DEADLINE;
    let ops = similar::capture_diff_slices_deadline(
        Algorithm::Myers,
        &old_lines,
        &new_lines,
        Some(deadline),
    );
    let timed_out = Instant::now() >= deadline;

    let mut added = 0;
    let mut removed = 0;
    for op in &ops {
        match *op {
            DiffOp::Delete { old_len, .. } => removed += old_len,
            DiffOp::Insert { new_len, .. } => added += new_len,
            DiffOp::Replace { old_len, new_len, .. } => {
                removed += old_len;
                added += new_len;
            }
            DiffOp::Equal { .. } => {}
        }
    }

    let hunks = group_hunks(&ops);
    FileDiff { ops, hunks, added, removed, timed_out }
}

fn group_hunks(ops: &[DiffOp]) -> Vec<Hunk> {
    let groups = similar::group_diff_ops(ops.to_vec(), CONTEXT);
    groups
        .into_iter()
        .map(|group| {
            let mut lines = Vec::new();
            for op in &group {
                match *op {
                    DiffOp::Equal { old_index, new_index, len } => {
                        for k in 0..len {
                            lines.push(DiffLine::Context { old: old_index + k, new: new_index + k });
                        }
                    }
                    DiffOp::Delete { old_index, old_len, .. } => {
                        for k in 0..old_len {
                            lines.push(DiffLine::Removed { old: old_index + k });
                        }
                    }
                    DiffOp::Insert { new_index, new_len, .. } => {
                        for k in 0..new_len {
                            lines.push(DiffLine::Added { new: new_index + k });
                        }
                    }
                    DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                        for k in 0..old_len {
                            lines.push(DiffLine::Removed { old: old_index + k });
                        }
                        for k in 0..new_len {
                            lines.push(DiffLine::Added { new: new_index + k });
                        }
                    }
                }
            }
            let first = group.first().expect("non-empty group");
            let last = group.last().expect("non-empty group");
            let old_start = first.old_range().start;
            let new_start = first.new_range().start;
            Hunk {
                old_start,
                old_len: last.old_range().end - old_start,
                new_start,
                new_len: last.new_range().end - new_start,
                lines,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> TextContent {
        TextContent::from_bytes(s.as_bytes().to_vec())
    }

    #[test]
    fn counts_and_hunks() {
        let d = compute(Some(&t("a\nb\nc\n")), Some(&t("a\nB\nc\nd\n")));
        assert_eq!((d.added, d.removed), (2, 1));
        assert_eq!(d.hunks.len(), 1);
        let h = &d.hunks[0];
        assert_eq!((h.old_start, h.old_len, h.new_start, h.new_len), (0, 3, 0, 4));
    }

    #[test]
    fn added_and_deleted() {
        let d = compute(None, Some(&t("x\ny\n")));
        assert_eq!((d.added, d.removed), (2, 0));
        let d = compute(Some(&t("x\ny\n")), None);
        assert_eq!((d.added, d.removed), (0, 2));
        let d = compute(None, None);
        assert!(d.hunks.is_empty());
    }

    #[test]
    fn identical() {
        let d = compute(Some(&t("a\nb\n")), Some(&t("a\nb\n")));
        assert!(d.hunks.is_empty());
        assert_eq!((d.added, d.removed), (0, 0));
    }
}
