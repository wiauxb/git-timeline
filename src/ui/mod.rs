//! Pure rendering: `&AppState` in, frame out. Returns the layout for hit-testing.

pub mod blame_view;
pub mod content_view;
pub mod dialog;
pub mod diff_view;
pub mod file_pane;
pub mod help;
pub mod status_bar;
pub mod theme;
pub mod timeline_pane;

use chrono::{DateTime, FixedOffset, Local};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;

use crate::app::state::{AppState, Focus};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct LayoutInfo {
    /// Zero-size when hidden.
    pub timeline: Rect,
    pub file: Rect,
    /// Rows where file text is drawn (below the header line).
    pub file_body: Rect,
    pub file_body_rows: u16,
    pub file_body_cols: u16,
}

pub fn draw(f: &mut Frame, s: &AppState) -> LayoutInfo {
    let [main, status] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(f.area());

    let narrow = main.width < 60;
    let (timeline, file) = if s.zoomed || (narrow && s.focus == Focus::File) {
        (Rect::default(), main)
    } else if narrow {
        (main, Rect::default())
    } else {
        let w = (main.width as u32 * 35 / 100) as u16;
        let w = w.clamp(30, 60).min(main.width.saturating_sub(20));
        let [t, fl] = Layout::horizontal([Constraint::Length(w), Constraint::Min(1)]).areas(main);
        (t, fl)
    };

    let mut layout = LayoutInfo { timeline, file, ..Default::default() };
    if timeline.area() > 0 {
        timeline_pane::render(f, timeline, s);
    }
    if file.area() > 0 {
        layout.file_body = file_pane::render(f, file, s);
        layout.file_body_rows = layout.file_body.height;
        layout.file_body_cols = layout.file_body.width;
    }
    status_bar::render(f, status, s);
    if let Some(o) = &s.overlay {
        dialog::render(f, f.area(), s, o);
    }
    layout
}

/// Compact relative age: `5m`, `3h`, `2d`, `3w`, `4mo`, `2y`.
pub fn age(now: DateTime<Local>, t: &DateTime<FixedOffset>) -> String {
    let secs = (now.timestamp() - t.timestamp()).max(0);
    let (m, h, d) = (60, 3600, 86400);
    match secs {
        s if s < h => format!("{}m", s / m),
        s if s < d => format!("{}h", s / h),
        s if s < 14 * d => format!("{}d", s / d),
        s if s < 8 * 7 * d => format!("{}w", s / (7 * d)),
        s if s < 365 * d => format!("{}mo", s / (30 * d)),
        s => format!("{}y", s / (365 * d)),
    }
}

/// Longer relative date for titles: "3 days ago".
pub fn ago(now: DateTime<Local>, t: &DateTime<FixedOffset>) -> String {
    let secs = (now.timestamp() - t.timestamp()).max(0);
    let unit = |n: i64, u: &str| format!("{n} {u}{} ago", if n == 1 { "" } else { "s" });
    match secs {
        s if s < 60 => "just now".into(),
        s if s < 3600 => unit(s / 60, "minute"),
        s if s < 86400 => unit(s / 3600, "hour"),
        s if s < 30 * 86400 => unit(s / 86400, "day"),
        s if s < 365 * 86400 => unit(s / (30 * 86400), "month"),
        s => unit(s / (365 * 86400), "year"),
    }
}

/// Truncate to `w` display columns, with `…` when cut.
pub fn fit(s: &str, w: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut out = String::new();
    let mut used = 0;
    let total: usize = s.chars().map(|c| c.width().unwrap_or(0)).sum();
    if total <= w {
        return s.to_string();
    }
    for c in s.chars() {
        let cw = c.width().unwrap_or(0);
        if used + cw + 1 > w {
            break;
        }
        out.push(c);
        used += cw;
    }
    if w > 0 {
        out.push('…');
    }
    out
}

pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width.saturating_sub(2)).max(1);
    let h = h.min(area.height.saturating_sub(2)).max(1);
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}
