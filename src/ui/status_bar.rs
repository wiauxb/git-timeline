//! Bottom line: focus badge, view tabs, hints or transient message.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::fit;
use super::theme::theme;
use crate::app::state::{AppState, Focus, Level, ViewMode};

pub fn render(f: &mut Frame, area: Rect, s: &AppState) {
    let t = theme();
    let mut spans = vec![Span::styled(
        match s.focus {
            Focus::Timeline => " TIMELINE ",
            Focus::File => " FILE ",
        },
        t.status_badge,
    )];
    spans.push(Span::raw(" "));
    for (key, name, v) in [
        ("1", "Content", ViewMode::Content),
        ("2", "Diff", ViewMode::DiffPrev),
        ("3", "Cmp", ViewMode::DiffMarked),
        ("4", "Blame", ViewMode::Blame),
    ] {
        let style = if s.view == v {
            t.status_tab_active
        } else if v == ViewMode::DiffMarked && s.mark.is_none() {
            t.dim
        } else {
            t.status_tab
        };
        spans.push(Span::styled(format!(" {key} {name} "), style));
    }
    spans.push(Span::raw(" │ "));
    let used: usize = spans.iter().map(|x| x.width()).sum();

    let (right, style) = match &s.status {
        Some(m) => (
            m.text.clone(),
            match m.level {
                Level::Info => t.status_info,
                Level::Warn => t.status_warn,
                Level::Error => t.status_error,
            },
        ),
        None => (hints(s), t.dim),
    };
    let pos = position(s);
    let avail = (area.width as usize).saturating_sub(used + pos.len() + 1);
    spans.push(Span::styled(fit(&right, avail), style));
    let used: usize = spans.iter().map(|x| x.width()).sum();
    let pad = (area.width as usize).saturating_sub(used + pos.len());
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(pos, t.dim));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn hints(s: &AppState) -> String {
    match s.focus {
        Focus::Timeline => "m mark · y copy · e export · R restore · d details · ? help".into(),
        Focus::File => match s.view {
            ViewMode::Blame => "⏎ jump to commit · [ ] older/newer · # numbers · ? help".into(),
            ViewMode::DiffPrev | ViewMode::DiffMarked => "n/N hunk · [ ] older/newer · z zoom · ? help".into(),
            ViewMode::Content => "[ ] older/newer · # numbers · z zoom · ? help".into(),
        },
    }
}

fn position(s: &AppState) -> String {
    match s.displayed().row_count() {
        Some(n) if n > 0 => format!("{}/{} ", s.file.cursor + 1, n),
        _ => String::new(),
    }
}
