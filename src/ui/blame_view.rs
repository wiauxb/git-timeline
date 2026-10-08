//! File content with a blame gutter.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::content_view::{cursor_style, digits, styled_line};
use super::fit;
use super::theme::theme;
use crate::app::state::AppState;
use crate::highlight::HighlightedText;
use crate::model::{BlameData, RevisionId, TextContent};

const GUTTER: usize = 28;

pub fn render(
    f: &mut Frame,
    area: Rect,
    s: &AppState,
    text: &TextContent,
    hl: Option<&HighlightedText>,
    blame: Option<Result<&BlameData, &str>>,
) {
    let t = theme();
    let mut area = area;
    let blame = match blame {
        Some(Err(e)) => {
            f.render_widget(Paragraph::new(format!("blame failed: {e}")).style(t.status_error), Rect { height: 1, ..area });
            area.y += 1;
            area.height = area.height.saturating_sub(1);
            None
        }
        Some(Ok(b)) if b.lines.len() == text.line_count() => Some(b),
        _ => None,
    };

    // age buckets between oldest and newest commit of this blame
    let (oldest, newest) = blame
        .map(|b| {
            let times = b.commits.values().map(|c| c.author_time.timestamp());
            (times.clone().min().unwrap_or(0), times.max().unwrap_or(0))
        })
        .unwrap_or((0, 0));
    let bucket = |ts: i64| -> usize {
        if newest <= oldest {
            return 0;
        }
        (((newest - ts) as f64 / (newest - oldest) as f64) * 5.0).round() as usize
    };

    let nw = if s.file.show_line_numbers { digits(text.line_count()) + 3 } else { 0 };
    let text_w = (area.width as usize).saturating_sub(GUTTER + 2 + nw);
    let lines: Vec<Line> = (s.file.top..text.line_count())
        .take(area.height as usize)
        .map(|i| {
            let mut spans = Vec::new();
            match blame.and_then(|b| b.lines.get(i).map(|l| (b, l))) {
                Some((b, l)) if l.group_start || i == s.file.top => {
                    if l.commit.is_null() {
                        spans.push(Span::styled(format!("{:<GUTTER$}", "(not committed yet)"), t.dim));
                    } else {
                        let c = &b.commits[&l.commit];
                        let mut hstyle = t.hash;
                        if !t.mono {
                            hstyle = Style::default().fg(t.blame_age[bucket(c.author_time.timestamp()).min(5)]);
                        }
                        if s.timeline.position_of(&RevisionId::Commit(l.commit.clone())).is_some() {
                            hstyle = hstyle.patch(t.link);
                        }
                        // 7 + 1 + 9 + 1 + 10 = GUTTER
                        spans.push(Span::styled(l.commit.short().to_string(), hstyle));
                        spans.push(Span::styled(format!(" {:<9} ", fit(&c.author, 9)), t.blame_gutter));
                        spans.push(Span::styled(c.author_time.format("%Y-%m-%d").to_string(), t.date));
                    }
                }
                Some(_) => spans.push(Span::styled(format!("{:<GUTTER$}", "   ┆"), t.dim)),
                None if i == s.file.top => {
                    spans.push(Span::styled(format!("{:<GUTTER$}", format!("{} blame…", s.spinner())), t.dim))
                }
                None => spans.push(Span::raw(" ".repeat(GUTTER))),
            }
            spans.push(Span::styled(" │", t.dim));
            if nw > 0 {
                spans.push(Span::styled(format!("{:>w$} │ ", i + 1, w = nw - 3), t.line_number));
            } else {
                spans.push(Span::raw(" "));
            }
            spans.extend(styled_line(text, hl, i, Style::default(), s.file.hscroll, text_w, s.opts.tab_width));
            cursor_style(Line::from(spans), s, i)
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}
