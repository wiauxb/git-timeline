//! Full file content at the selected revision, plus the line renderer shared by all views.

use std::ops::Range;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

use super::theme::theme;
use crate::app::state::{AppState, Focus};
use crate::highlight::HighlightedText;
use crate::model::TextContent;

pub fn render(f: &mut Frame, area: Rect, s: &AppState, text: &TextContent, hl: Option<&HighlightedText>) {
    let t = theme();
    let gutter_w = if s.file.show_line_numbers { digits(text.line_count()) + 3 } else { 0 };
    let text_w = (area.width as usize).saturating_sub(gutter_w);
    let lines: Vec<Line> = (s.file.top..text.line_count())
        .take(area.height as usize)
        .map(|i| {
            let mut spans = Vec::new();
            if gutter_w > 0 {
                spans.push(Span::styled(format!("{:>w$} │ ", i + 1, w = gutter_w - 3), t.line_number));
            }
            spans.extend(styled_line(text, hl, i, Style::default(), s.file.hscroll, text_w, s.opts.tab_width));
            cursor_style(Line::from(spans), s, i)
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}

pub fn cursor_style<'a>(line: Line<'a>, s: &AppState, row: usize) -> Line<'a> {
    let t = theme();
    if row != s.file.cursor || s.file.pending_anchor.is_some() {
        return line;
    }
    line.style(if s.focus == Focus::File { t.cursor_line } else { t.cursor_line_unfocused })
}

pub fn digits(n: usize) -> usize {
    n.max(1).to_string().len()
}

/// Render one line: tabs expanded, control chars as `^X`, horizontal scroll by display
/// columns (a wide char cut by the scroll becomes a space), highlight spans applied on
/// top of `base`.
pub fn styled_line(
    text: &TextContent,
    hl: Option<&HighlightedText>,
    line: usize,
    base: Style,
    hscroll: usize,
    width: usize,
    tab_width: u8,
) -> Vec<Span<'static>> {
    let s = text.line_str(line);
    let spans: &[(Range<usize>, Style)] = hl.and_then(|h| h.lines.get(line)).map(Vec::as_slice).unwrap_or(&[]);
    let dim = theme().dim;
    let tab = tab_width.max(1) as usize;
    let mut b = LineBuilder { hscroll, width, col: 0, used: 0, buf: String::new(), buf_style: base, out: Vec::new() };
    let mut si = 0;

    for (bi, ch) in s.char_indices() {
        while si < spans.len() && spans[si].0.end <= bi {
            si += 1;
        }
        let style = match spans.get(si) {
            Some((r, st)) if r.contains(&bi) => base.patch(*st),
            _ => base,
        };
        let ok = if ch == '\t' {
            let n = tab - b.col % tab;
            b.push(&" ".repeat(n), n, style)
        } else if ch.is_control() {
            b.push(&format!("^{}", ((ch as u32 as u8) ^ 0x40) as char), 2, base.patch(dim))
        } else {
            b.push(ch.encode_utf8(&mut [0; 4]), ch.width().unwrap_or(0), style)
        };
        if !ok {
            break;
        }
    }
    b.flush();
    // tinted lines (diff) are filled to the edge
    if base.bg.is_some() && b.used < width {
        b.out.push(Span::styled(" ".repeat(width - b.used), base));
    }
    b.out
}

struct LineBuilder {
    hscroll: usize,
    width: usize,
    /// Column in the full line.
    col: usize,
    /// Columns emitted.
    used: usize,
    buf: String,
    buf_style: Style,
    out: Vec<Span<'static>>,
}

impl LineBuilder {
    /// False when the line is full.
    fn push(&mut self, piece: &str, w: usize, style: Style) -> bool {
        let start = self.col;
        self.col += w;
        if self.col <= self.hscroll {
            return true;
        }
        let (piece, w) = if start < self.hscroll {
            let vis = self.col - self.hscroll; // partly scrolled out: pad the visible part
            (" ".repeat(vis), vis)
        } else {
            (piece.to_string(), w)
        };
        if self.used + w > self.width {
            return false;
        }
        if style != self.buf_style {
            self.flush();
            self.buf_style = style;
        }
        self.buf.push_str(&piece);
        self.used += w;
        true
    }

    fn flush(&mut self) {
        if !self.buf.is_empty() {
            self.out.push(Span::styled(std::mem::take(&mut self.buf), self.buf_style));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(s: &str, hscroll: usize, width: usize) -> String {
        let t = TextContent::from_bytes(s.as_bytes().to_vec());
        styled_line(&t, None, 0, Style::default(), hscroll, width, 4).iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn tabs_scroll_and_wide_chars() {
        assert_eq!(render("a\tb", 0, 80), "a   b");
        assert_eq!(render("abcdef", 2, 3), "cde");
        assert_eq!(render("日本", 1, 10), " 本");
        assert_eq!(render("x\u{1}y", 0, 10), "x^Ay");
        assert_eq!(render("日本語", 0, 3), "日");
    }
}
