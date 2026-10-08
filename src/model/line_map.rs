//! "Keep the same logical line" when switching revision.

use similar::DiffOp;

use super::{ContentKey, FileDiff};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappedLine {
    /// The line exists unchanged in the target.
    Exact(usize),
    /// The line was changed/removed; closest surviving line in the target.
    Nearest(usize),
}

impl MappedLine {
    pub fn line(&self) -> usize {
        match *self {
            MappedLine::Exact(l) | MappedLine::Nearest(l) => l,
        }
    }
}

/// What the view remembers to re-anchor after a revision change.
#[derive(Clone, Debug)]
pub struct LineAnchor {
    pub content: ContentKey,
    pub line: usize,
    /// cursor row - top row, to keep the line at the same screen position.
    pub viewport_offset: usize,
}

/// `diff` is old→new; `forward=true` maps old→new, `false` maps new→old.
pub fn map_line(diff: &FileDiff, line: usize, forward: bool) -> MappedLine {
    let target_len: usize = diff
        .ops
        .last()
        .map(|op| if forward { op.new_range().end } else { op.old_range().end })
        .unwrap_or(0);
    if target_len == 0 {
        return MappedLine::Nearest(0);
    }
    let clamp = |l: usize| l.min(target_len - 1);

    for op in &diff.ops {
        let (src, dst) = if forward {
            (op.old_range(), op.new_range())
        } else {
            (op.new_range(), op.old_range())
        };
        if !src.contains(&line) {
            continue;
        }
        let off = line - src.start;
        return match op {
            DiffOp::Equal { .. } => MappedLine::Exact(dst.start + off),
            _ if dst.is_empty() => MappedLine::Nearest(clamp(dst.start)),
            _ => MappedLine::Nearest(dst.start + off.min(dst.len() - 1)),
        };
    }
    // Beyond the end of the source: last line of the target.
    MappedLine::Nearest(target_len - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{diff::compute, TextContent};

    fn d(a: &str, b: &str) -> FileDiff {
        compute(
            Some(&TextContent::from_bytes(a.as_bytes().to_vec())),
            Some(&TextContent::from_bytes(b.as_bytes().to_vec())),
        )
    }

    #[test]
    fn unchanged() {
        let diff = d("a\nb\nc\n", "a\nb\nc\n");
        assert_eq!(map_line(&diff, 1, true), MappedLine::Exact(1));
    }

    #[test]
    fn insert_before() {
        let diff = d("a\nb\nc\n", "x\ny\na\nb\nc\n");
        assert_eq!(map_line(&diff, 2, true), MappedLine::Exact(4));
        assert_eq!(map_line(&diff, 4, false), MappedLine::Exact(2));
        assert_eq!(map_line(&diff, 0, false), MappedLine::Nearest(0));
    }

    #[test]
    fn deleted_block() {
        let diff = d("a\nb\nc\nd\n", "a\nd\n");
        assert_eq!(map_line(&diff, 2, true), MappedLine::Nearest(1));
        assert_eq!(map_line(&diff, 3, true), MappedLine::Exact(1));
    }

    #[test]
    fn replaced_block() {
        let diff = d("a\nb\nc\nd\n", "a\nX\nd\n");
        assert_eq!(map_line(&diff, 1, true), MappedLine::Nearest(1));
        assert_eq!(map_line(&diff, 2, true), MappedLine::Nearest(1));
        assert_eq!(map_line(&diff, 3, true), MappedLine::Exact(2));
    }

    #[test]
    fn delete_at_eof_and_empty() {
        let diff = d("a\nb\nc\n", "a\n");
        assert_eq!(map_line(&diff, 2, true), MappedLine::Nearest(0));
        let diff = d("a\nb\n", "");
        assert_eq!(map_line(&diff, 1, true), MappedLine::Nearest(0));
        let diff = d("", "a\nb\n");
        assert_eq!(map_line(&diff, 0, true), MappedLine::Nearest(1));
    }
}
