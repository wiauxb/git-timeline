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
use crate::app::search::current_match;
use crate::highlight::HighlightedText;
use crate::model::TextContent;

pub fn render(f: &mut Frame, area: Rect, s: &AppState, text: &TextContent, hl: Option<&HighlightedText>) {
    let t = theme();
    let gutter_w = gutter_cols(s, text);
    let text_w = (area.width as usize).saturating_sub(gutter_w);
    let num_w = gutter_w.saturating_sub(3);
    let lines = body_lines(s, area.height as usize, text.line_count(), |i| {
        let marks = search_marks(s, i);
        let rows = if s.file.wrap {
            styled_wrapped(text, hl, i, Style::default(), &marks, text_w, s.opts.tab_width)
        } else {
            vec![styled_line(text, hl, i, Style::default(), &marks, s.file.hscroll, text_w, s.opts.tab_width)]
        };
        rows.into_iter()
            .enumerate()
            .map(|(seg, spans)| {
                let gutter = match (s.file.show_line_numbers, seg) {
                    (true, 0) => Some(Span::styled(format!("{:>num_w$} │ ", i + 1), t.line_number)),
                    (true, _) => Some(Span::styled(format!("{:>num_w$} │ ", "↪"), t.line_number)),
                    (false, _) if gutter_w == 0 => None,
                    (false, 0) => Some(Span::raw("  ")),
                    (false, _) => Some(Span::styled("↪ ", t.dim)),
                };
                Line::from(gutter.into_iter().chain(spans).collect::<Vec<_>>())
            })
            .collect()
    });
    f.render_widget(Paragraph::new(lines), area);
}

/// Screen lines of the body: rows `top..n` (each 1+ screen lines from `row_lines`), the
/// first `top_skip` lines of the top row skipped, cut at `height`; cursor style applied.
pub fn body_lines<'a>(
    s: &AppState,
    height: usize,
    n: usize,
    mut row_lines: impl FnMut(usize) -> Vec<Line<'a>>,
) -> Vec<Line<'a>> {
    let mut out = Vec::with_capacity(height);
    for i in s.file.top..n {
        if out.len() >= height {
            break;
        }
        let skip = if i == s.file.top { s.file.top_skip } else { 0 };
        let lines = row_lines(i);
        let skip = skip.min(lines.len().saturating_sub(1));
        for l in lines.into_iter().skip(skip) {
            if out.len() >= height {
                break;
            }
            out.push(cursor_style(l, s, i));
        }
    }
    out
}

/// Columns left of the text: line numbers + ` │ `; without numbers 0, or 2 when wrapped
/// (room for the `↪` marker).
pub fn gutter_cols(s: &AppState, text: &TextContent) -> usize {
    match (s.file.show_line_numbers, s.file.wrap) {
        (true, _) => digits(text.line_count()) + 3,
        (false, true) => 2,
        (false, false) => 0,
    }
}

/// A search match on one row: byte range in the row's text, and whether `n`/`N` is on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    pub bytes: Range<usize>,
    pub current: bool,
}

/// The file search's matches on display row `row`, from the cached hit list.
pub fn search_marks(s: &AppState, row: usize) -> Vec<Mark> {
    if s.search.file.is_none() {
        return Vec::new();
    }
    let all = &s.search.file_hits.matches;
    let current = current_match(s).map(|m| m.bytes.start);
    let lo = all.partition_point(|m| m.row < row);
    all[lo..]
        .iter()
        .take_while(|m| m.row == row)
        .map(|m| Mark { bytes: m.bytes.clone(), current: current == Some(m.bytes.start) && s.file.cursor == row })
        .collect()
}

/// Columns taken by `ch` at column `col`.
fn char_cols(ch: char, col: usize, tab: usize) -> usize {
    if ch == '\t' {
        tab - col % tab
    } else if ch.is_control() {
        2
    } else {
        ch.width().unwrap_or(0)
    }
}

/// Display column of byte `byte` in `line`, with the same rules as `styled_line`.
pub fn col_of_byte(line: &str, byte: usize, tab_width: u8) -> usize {
    let tab = tab_width.max(1) as usize;
    let mut col = 0;
    for (bi, ch) in line.char_indices() {
        if bi >= byte {
            break;
        }
        col += char_cols(ch, col, tab);
    }
    col
}

/// Byte offsets where the continuation segments of `line` start when wrapped to `width`
/// columns, at word boundaries (a word longer than `width` is hard-broken). Tab stops
/// are relative to each segment's start; trailing whitespace hangs at the segment's end.
pub fn wrap_breaks(line: &str, width: usize, tab_width: u8) -> Vec<usize> {
    let mut breaks = Vec::new();
    if width == 0 {
        return breaks;
    }
    let tab = tab_width.max(1) as usize;
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let (mut i, mut seg, mut col) = (0, 0, 0);
    // char index where the next segment may start (after whitespace following a word)
    let mut brk: Option<usize> = None;
    let mut word = false;
    while i < chars.len() {
        let ch = chars[i].1;
        let w = char_cols(ch, col, tab);
        if col + w > width && i > seg {
            let at = if ch.is_whitespace() {
                // let the whitespace run hang at the end of this segment
                let mut j = i;
                while j < chars.len() && chars[j].1.is_whitespace() {
                    j += 1;
                }
                j
            } else {
                brk.filter(|&b| b > seg).unwrap_or(i)
            };
            if at >= chars.len() {
                break;
            }
            breaks.push(chars[at].0);
            (i, seg, col, brk, word) = (at, at, 0, None, false);
            continue;
        }
        col += w;
        if ch.is_whitespace() {
            if word {
                brk = Some(i + 1);
            }
        } else {
            word = true;
        }
        i += 1;
    }
    breaks
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
/// top of `base`, then search `marks` on top of everything.
#[allow(clippy::too_many_arguments)]
pub fn styled_line(
    text: &TextContent,
    hl: Option<&HighlightedText>,
    line: usize,
    base: Style,
    marks: &[Mark],
    hscroll: usize,
    width: usize,
    tab_width: u8,
) -> Vec<Span<'static>> {
    let s = text.line_str(line);
    let spans = hl.and_then(|h| h.lines.get(line)).map(Vec::as_slice).unwrap_or(&[]);
    styled_range(&s, 0..s.len(), spans, base, marks, hscroll, width, tab_width)
}

/// Like `styled_line` without hscroll, one span list per screen row (`wrap_breaks`).
#[allow(clippy::too_many_arguments)]
pub fn styled_wrapped(
    text: &TextContent,
    hl: Option<&HighlightedText>,
    line: usize,
    base: Style,
    marks: &[Mark],
    width: usize,
    tab_width: u8,
) -> Vec<Vec<Span<'static>>> {
    let s = text.line_str(line);
    let spans = hl.and_then(|h| h.lines.get(line)).map(Vec::as_slice).unwrap_or(&[]);
    let breaks = wrap_breaks(&s, width, tab_width);
    let starts = std::iter::once(0).chain(breaks.iter().copied());
    let ends = breaks.iter().copied().chain(std::iter::once(s.len()));
    starts.zip(ends).map(|(a, b)| styled_range(&s, a..b, spans, base, marks, 0, width, tab_width)).collect()
}

/// Render the bytes `range` of `s` (a char-aligned range; columns restart at its start).
#[allow(clippy::too_many_arguments)]
fn styled_range(
    s: &str,
    range: Range<usize>,
    spans: &[(Range<usize>, Style)],
    base: Style,
    marks: &[Mark],
    hscroll: usize,
    width: usize,
    tab_width: u8,
) -> Vec<Span<'static>> {
    let t = theme();
    let tab = tab_width.max(1) as usize;
    let mut b = LineBuilder { hscroll, width, col: 0, used: 0, buf: String::new(), buf_style: base, out: Vec::new() };
    let (mut si, mut mi) = (0, 0);

    for (bi, ch) in s[range.clone()].char_indices() {
        let bi = bi + range.start;
        while si < spans.len() && spans[si].0.end <= bi {
            si += 1;
        }
        while mi < marks.len() && marks[mi].bytes.end <= bi {
            mi += 1;
        }
        let mark = |st: Style| match marks.get(mi) {
            Some(m) if m.bytes.contains(&bi) => st.patch(if m.current { t.search_current } else { t.search_match }),
            _ => st,
        };
        let style = mark(match spans.get(si) {
            Some((r, st)) if r.contains(&bi) => base.patch(*st),
            _ => base,
        });
        let ok = if ch == '\t' {
            let n = char_cols(ch, b.col, tab);
            b.push(&" ".repeat(n), n, style)
        } else if ch.is_control() {
            b.push(&format!("^{}", ((ch as u32 as u8) ^ 0x40) as char), 2, mark(base.patch(t.dim)))
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
        styled_line(&t, None, 0, Style::default(), &[], hscroll, width, 4).iter().map(|s| s.content.to_string()).collect()
    }

    #[test]
    fn tabs_scroll_and_wide_chars() {
        assert_eq!(render("a\tb", 0, 80), "a   b");
        assert_eq!(render("abcdef", 2, 3), "cde");
        assert_eq!(render("日本", 1, 10), " 本");
        assert_eq!(render("x\u{1}y", 0, 10), "x^Ay");
        assert_eq!(render("日本語", 0, 3), "日");
    }

    #[test]
    fn marks_styled_and_scrolled() {
        let t = TextContent::from_bytes(b"abcdef".to_vec());
        let hit = theme().search_match;
        // mark "bcd", scrolled by 2: only "cd" is visible and styled
        let marks = [Mark { bytes: 1..4, current: false }];
        let spans = styled_line(&t, None, 0, Style::default(), &marks, 2, 10, 4);
        let styled: String = spans.iter().filter(|s| s.style == hit).map(|s| s.content.to_string()).collect();
        assert_eq!(styled, "cd");
        let plain: String = spans.iter().filter(|s| s.style != hit).map(|s| s.content.to_string()).collect();
        assert_eq!(plain, "ef");

        assert_eq!(col_of_byte("a\tb", 2, 4), 4);
        assert_eq!(col_of_byte("日本x", "日本".len(), 4), 4);
        assert_eq!(col_of_byte("x\u{1}y", 2, 4), 3);
        assert_eq!(render("a\tb", 0, 80).find('b'), Some(4));
    }

    fn segments(s: &str, width: usize) -> Vec<String> {
        let b = wrap_breaks(s, width, 4);
        let starts = std::iter::once(0).chain(b.iter().copied());
        let ends = b.iter().copied().chain(std::iter::once(s.len()));
        starts.zip(ends).map(|(a, e)| s[a..e].to_string()).collect()
    }

    #[test]
    fn wrap_word_boundaries() {
        assert_eq!(segments("hello world foo", 11), ["hello world ", "foo"]);
        assert_eq!(segments("hello world foo", 10), ["hello ", "world foo"]);
        assert_eq!(segments("hello world foo", 15), ["hello world foo"]);
        // whitespace overflowing hangs at the end of the segment
        assert_eq!(segments("abcde   fgh", 5), ["abcde   ", "fgh"]);
        // a word longer than the width is hard-broken
        assert_eq!(segments("abcdefghij xy", 4), ["abcd", "efgh", "ij ", "xy"]);
        // leading indentation is not a break opportunity
        assert_eq!(segments("    abcdefgh", 6), ["    ab", "cdefgh"]);
        // a wide char at the edge moves to the next row; width 1 still progresses
        assert_eq!(segments("ab日", 3), ["ab", "日"]);
        assert_eq!(segments("日本", 1), ["日", "本"]);
        // tab stops restart with each segment
        assert_eq!(segments("ab\tc d\tef", 6), ["ab\tc ", "d\tef"]);
        assert!(wrap_breaks("anything", 0, 4).is_empty());
    }

    #[test]
    fn styled_wrapped_spans() {
        let t = TextContent::from_bytes(b"one two three".to_vec());
        let hit = theme().search_match;
        let cur = theme().search_current;
        // "two thr" crosses the break after "two "
        let marks = [Mark { bytes: 4..11, current: false }];
        let rows = styled_wrapped(&t, None, 0, Style::default(), &marks, 8, 4);
        let text = |r: &Vec<Span>| r.iter().map(|s| s.content.to_string()).collect::<String>();
        let styled = |r: &Vec<Span>| r.iter().filter(|s| s.style == hit).map(|s| s.content.to_string()).collect::<String>();
        assert_eq!(rows.iter().map(text).collect::<Vec<_>>(), ["one two ", "three"]);
        assert_eq!(styled(&rows[0]), "two ");
        assert_eq!(styled(&rows[1]), "thr");
        let marks = [Mark { bytes: 0..3, current: true }];
        let rows = styled_wrapped(&t, None, 0, Style::default(), &marks, 8, 4);
        assert_eq!(rows[0][0].style, cur);
    }
}
