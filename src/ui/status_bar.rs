//! Bottom line: focus badge, view tabs, hints or transient message.

use ratatui::layout::{Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::fit;
use super::theme::theme;
use crate::app::search::{current_hit, scope_of, SearchPrompt};
use crate::app::state::{AppState, Focus, Level, Overlay, ViewMode};

fn badge(s: &AppState) -> Span<'static> {
    Span::styled(
        match s.focus {
            Focus::Timeline => " TIMELINE ",
            Focus::File => " FILE ",
        },
        theme().status_badge,
    )
}

pub fn render(f: &mut Frame, area: Rect, s: &AppState) {
    if let Some(Overlay::Search(p)) = &s.overlay {
        return prompt(f, area, s, p);
    }
    let t = theme();
    let mut spans = vec![badge(s)];
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
    let pos = format!("{}{}", search_info(s), position(s));
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
        Focus::Timeline => "/ search · m mark · y copy · e export · R restore · d details · ? help".into(),
        Focus::File => match s.view {
            ViewMode::Blame => "⏎ jump to commit · [ ] older/newer · # numbers · ? help".into(),
            ViewMode::DiffPrev | ViewMode::DiffMarked => "{ } hunk · / search · [ ] older/newer · z zoom · ? help".into(),
            ViewMode::Content => "/ search · [ ] older/newer · # numbers · w wrap · z zoom · ? help".into(),
        },
    }
}

fn position(s: &AppState) -> String {
    match s.displayed().row_count() {
        Some(n) if n > 0 => format!("{}/{} ", s.file.cursor + 1, n),
        _ => String::new(),
    }
}

/// `/<query> k/M · ` for the focused pane's search.
fn search_info(s: &AppState) -> String {
    let scope = scope_of(s.focus);
    let Some(q) = s.search.query(scope) else { return String::new() };
    let count = match s.search.hit_count(scope) {
        0 => "no match".to_string(),
        m => match current_hit(s, scope) {
            Some(k) => format!("{}/{m}", k + 1),
            None => format!("-/{m}"),
        },
    };
    format!("/{} {count} · ", fit(q.pattern(), 20))
}

/// The `/` prompt replaces the whole bar while it is open.
fn prompt(f: &mut Frame, area: Rect, s: &AppState, p: &SearchPrompt) {
    let t = theme();
    let (right, rstyle) = match &p.error {
        Some(e) => (format!("invalid regex: {e}"), t.status_error),
        None if p.input.value.is_empty() => (String::new(), t.dim),
        None => match s.search.hit_count(p.scope) {
            0 => ("no match".to_string(), t.dim),
            1 => ("1 match".to_string(), t.dim),
            n => (format!("{n} matches"), t.dim),
        },
    };
    let badge = badge(s);
    let lead = badge.width() + 2; // badge, space, `/`
    let width = area.width as usize;
    let right = fit(&right, (width / 2).saturating_sub(1));
    let input_w = width.saturating_sub(lead + right.chars().count() + 2);
    let mut spans = vec![badge, Span::raw(" /"), Span::raw(fit(&p.input.value, input_w))];
    let used: usize = spans.iter().map(|x| x.width()).sum();
    let pad = width.saturating_sub(used + right.chars().count() + 1);
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(right, rstyle));
    f.render_widget(Paragraph::new(Line::from(spans)), area);

    // terminal cursor at the insertion point
    let col: usize = p.input.value.chars().take(p.input.cursor).map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)).sum();
    let x = area.x + (lead + col.min(input_w)) as u16;
    if x < area.x + area.width {
        f.set_cursor_position(Position { x, y: area.y });
    }
}
