//! Unified diff between two versions (diff vs previous, diff vs marked).

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::content_view::{cursor_style, digits, styled_line};
use super::theme::theme;
use crate::app::state::AppState;
use crate::highlight::HighlightedText;
use crate::model::{DiffLine, DiffRow, FileContent, FileDiff, FileMode, TextContent};

pub struct DiffInputs<'a> {
    pub diff: &'a FileDiff,
    pub rows: &'a [DiffRow],
    pub old: Option<&'a TextContent>,
    pub new: Option<&'a TextContent>,
    pub old_hl: Option<&'a HighlightedText>,
    pub new_hl: Option<&'a HighlightedText>,
    pub old_mode: FileMode,
    pub new_mode: FileMode,
}

pub fn render(f: &mut Frame, area: Rect, s: &AppState, d: &DiffInputs) {
    let t = theme();
    if d.rows.is_empty() {
        let msg = if d.old_mode != d.new_mode && d.old_mode != FileMode::Absent && d.new_mode != FileMode::Absent {
            format!("mode changed {} → {}", mode_str(d.old_mode), mode_str(d.new_mode))
        } else if d.old.is_none() && d.new.is_none() {
            "file absent on both sides".into()
        } else {
            "no changes".into()
        };
        f.render_widget(Paragraph::new(msg).style(t.dim).centered(), Rect { y: area.y + area.height / 3, height: 1, ..area });
        return;
    }

    let max_line = d.old.map(|x| x.line_count()).unwrap_or(0).max(d.new.map(|x| x.line_count()).unwrap_or(0));
    let nw = if s.file.show_line_numbers { digits(max_line) } else { 0 };
    let gutter = if nw > 0 { nw * 2 + 4 } else { 2 };
    let text_w = (area.width as usize).saturating_sub(gutter);
    let num = |n: Option<usize>| match n {
        Some(n) if nw > 0 => format!("{:>nw$}", n + 1),
        _ => " ".repeat(nw),
    };

    let lines: Vec<Line> = d
        .rows
        .iter()
        .enumerate()
        .skip(s.file.top)
        .take(area.height as usize)
        .map(|(row, r)| {
            let line = match *r {
                DiffRow::HunkHeader(h) => {
                    let h = &d.diff.hunks[h];
                    Line::from(Span::styled(
                        format!("@@ -{},{} +{},{} @@", h.old_start + 1, h.old_len, h.new_start + 1, h.new_len),
                        t.diff_hunk_header,
                    ))
                }
                DiffRow::Line(l) => {
                    let (o, n, marker, mstyle, base, side, idx) = match l {
                        DiffLine::Context { old, new } => (Some(old), Some(new), " ", t.dim, Style::default(), d.new.zip(Some(d.new_hl)), new),
                        DiffLine::Removed { old } => (Some(old), None, "-", t.diff_removed, t.diff_removed_bg, d.old.zip(Some(d.old_hl)), old),
                        DiffLine::Added { new } => (None, Some(new), "+", t.diff_added, t.diff_added_bg, d.new.zip(Some(d.new_hl)), new),
                    };
                    let mut spans = Vec::new();
                    if nw > 0 {
                        spans.push(Span::styled(format!("{} {} │", num(o), num(n)), t.line_number));
                    }
                    spans.push(Span::styled(format!("{marker} "), mstyle.patch(base)));
                    if let Some((text, hl)) = side {
                        // without colors, unhighlighted +/- lines take the marker's style
                        let base = if hl.is_none() && marker != " " { base.patch(mstyle) } else { base };
                        spans.extend(styled_line(text, hl, idx, base, s.file.hscroll, text_w, s.opts.tab_width));
                        if is_last_without_newline(text, idx) {
                            spans.push(Span::styled(" ⏎̸ no newline at end of file", t.dim));
                        }
                    }
                    Line::from(spans)
                }
            };
            cursor_style(line, s, row)
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}

fn is_last_without_newline(t: &TextContent, line: usize) -> bool {
    !t.ends_with_newline && line + 1 == t.line_count()
}

fn mode_str(m: FileMode) -> &'static str {
    match m {
        FileMode::Regular => "100644",
        FileMode::Executable => "100755",
        FileMode::Symlink => "120000",
        FileMode::Submodule => "160000",
        FileMode::Absent => "000000",
    }
}

/// One-line description of a non-text pair.
pub fn summary(old: &FileContent, new: &FileContent) -> String {
    fn desc(c: &FileContent) -> String {
        match c {
            FileContent::Absent => "absent".into(),
            FileContent::Text(t) => format!("text, {} lines", t.line_count()),
            FileContent::Binary { bytes } => format!("binary, {}", human(bytes.len() as u64)),
            FileContent::TooLarge { size, .. } => format!("too large, {}", human(*size)),
            FileContent::Symlink { target } => format!("symlink → {}", String::from_utf8_lossy(target)),
            FileContent::Submodule { commit } => format!("submodule at {}", commit.short()),
        }
    }
    format!("{}  →  {}", desc(old), desc(new))
}

pub fn human(n: u64) -> String {
    match n {
        n if n < 1024 => format!("{n} B"),
        n if n < 1024 * 1024 => format!("{:.1} KiB", n as f64 / 1024.0),
        n => format!("{:.1} MiB", n as f64 / (1024.0 * 1024.0)),
    }
}
