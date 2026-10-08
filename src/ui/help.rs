//! `?` overlay, generated from `app::keymap::BINDINGS`.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

use super::centered;
use super::theme::theme;
use crate::app::keymap::{Context, BINDINGS};
use crate::app::state::AppState;

pub fn render(f: &mut Frame, area: Rect, s: &AppState, scroll: usize) {
    let t = theme();
    let mut lines: Vec<Line> = Vec::new();
    for ctx in [Context::Global, Context::Timeline, Context::File, Context::DiffViews, Context::BlameView] {
        if !lines.is_empty() {
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(Span::styled(ctx.title(), t.title)));
        for b in BINDINGS.iter().filter(|b| b.context == ctx) {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<26}", b.keys), t.hash),
                Span::raw(b.action),
            ]));
        }
    }
    lines.push(Line::raw(""));
    let (a, b, c) = s.ctx.git_version;
    lines.push(Line::from(Span::styled(
        format!("git-timeline {} · git {a}.{b}.{c}", env!("CARGO_PKG_VERSION")),
        t.dim,
    )));

    let h = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let rect = centered(area, 82, h);
    let max_scroll = lines.len().saturating_sub(rect.height.saturating_sub(2) as usize);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll.min(max_scroll) as u16, 0))
            .block(Block::bordered().title(" Keys ").border_style(t.border_focused)),
        rect,
    );
}
