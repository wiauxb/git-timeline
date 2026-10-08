//! Right pane frame: title, header line, dispatch to the view renderer or a placeholder.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use super::diff_view::{self, human, DiffInputs};
use super::theme::theme;
use super::{ago, blame_view, content_view, fit};
use crate::app::state::{AppState, Displayed, Focus, ViewMode};
use crate::model::{FileContent, RevisionId};

/// Returns the body rect (where text rows are drawn).
pub fn render(f: &mut Frame, area: Rect, s: &AppState) -> Rect {
    let t = theme();
    let focused = s.focus == Focus::File;
    let block = Block::bordered()
        .title(title(s, area.width as usize))
        .border_style(if focused { t.border_focused } else { t.border });
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 2 {
        return Rect::default();
    }
    let header_area = Rect { height: 1, ..inner };
    let body = Rect { y: inner.y + 1, height: inner.height - 1, ..inner };

    let disp = s.displayed();
    f.render_widget(Paragraph::new(header(s, &disp)).style(t.dim), header_area);

    match &disp {
        Displayed::Text { content, hl, .. } => {
            let text = content.as_text().expect("Text variant holds text");
            if s.view == ViewMode::Blame {
                let blame = s.blame();
                let blame = blame.as_ref().map(|r| r.as_ref().map(|b| &**b).map_err(|e| e.as_str()));
                blame_view::render(f, body, s, text, hl.as_deref(), blame);
            } else {
                content_view::render(f, body, s, text, hl.as_deref());
            }
        }
        Displayed::Diff { diff, rows, old, new, old_hl, new_hl, old_mode, new_mode, .. } => {
            diff_view::render(
                f,
                body,
                s,
                &DiffInputs {
                    diff,
                    rows,
                    old: old.as_text(),
                    new: new.as_text(),
                    old_hl: old_hl.as_deref(),
                    new_hl: new_hl.as_deref(),
                    old_mode: *old_mode,
                    new_mode: *new_mode,
                },
            );
        }
        Displayed::DiffSummary { old, new, .. } => placeholder(f, body, &diff_view::summary(old, new)),
        Displayed::NotText { content, .. } => placeholder(f, body, &describe(content, s)),
        Displayed::Waiting => placeholder(f, body, &format!("{} loading…", s.spinner())),
        Displayed::Failed(e) => {
            f.render_widget(Paragraph::new(format!("error: {e}")).style(t.status_error), Rect { height: 1, ..body })
        }
        Displayed::NoMark => placeholder(f, body, "no compare base — press m on a revision to mark it"),
        Displayed::SameRevision => placeholder(f, body, "the marked revision is the selected one"),
        Displayed::NoSelection => placeholder(f, body, if s.timeline_loading { "loading history…" } else { "nothing selected" }),
    }

    if let Some(n) = disp.row_count() {
        if n > body.height as usize {
            let mut st = ScrollbarState::new(n.saturating_sub(body.height as usize)).position(s.file.top);
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight).begin_symbol(None).end_symbol(None),
                Rect { y: body.y, height: body.height, ..area },
                &mut st,
            );
        }
    }
    body
}

fn placeholder(f: &mut Frame, area: Rect, msg: &str) {
    if area.height == 0 {
        return;
    }
    f.render_widget(
        Paragraph::new(msg.to_string()).style(theme().dim).centered(),
        Rect { y: area.y + area.height / 3, height: 1, ..area },
    );
}

fn describe(c: &FileContent, s: &AppState) -> String {
    match c {
        FileContent::Absent => match s.selected_entry().map(|e| &e.rev) {
            Some(RevisionId::Commit(_)) => "file deleted in this revision".into(),
            _ => "file does not exist here".into(),
        },
        FileContent::Binary { bytes } => format!("binary file, {}", human(bytes.len() as u64)),
        FileContent::TooLarge { size, limit } => format!("file too large to display ({} > {})", human(*size), human(*limit)),
        FileContent::Symlink { target } => format!("symlink → {}", String::from_utf8_lossy(target)),
        FileContent::Submodule { commit } => format!("submodule at commit {}", commit.short()),
        FileContent::Text(_) => String::new(),
    }
}

fn title(s: &AppState, width: usize) -> Line<'static> {
    let t = theme();
    let view = match s.view {
        ViewMode::Content => "Content",
        ViewMode::DiffPrev => "Diff",
        ViewMode::DiffMarked => "Compare",
        ViewMode::Blame => "Blame",
    };
    let Some(e) = s.selected_entry() else {
        return Line::from(Span::styled(format!(" [{view}] "), t.title));
    };
    let mut spans = Vec::new();
    match (&s.view, s.marked_entry()) {
        (ViewMode::DiffMarked, Some((mi, m))) if m.rev != e.rev => {
            let (old, new) = if mi > s.selected_idx { (m, e) } else { (e, m) };
            spans.push(Span::styled(format!(" {} → {} ", old.label(), new.label()), t.hash));
        }
        _ => {
            spans.push(Span::styled(format!(" {} ", e.label()), if e.is_local() { t.pseudo_entry } else { t.hash }));
            if let Some(m) = &e.meta {
                let now = (s.clock)();
                spans.push(Span::styled(format!("· {} ", m.author.name), t.author));
                spans.push(Span::styled(format!("· {} ", ago(now, &m.author.time)), t.date));
                // borders (2) + "· " and trailing space (3) + "─ [view] " suffix
                let used: usize = spans.iter().map(|x| x.width()).sum::<usize>() + 2 + 3 + view.len() + 5;
                spans.push(Span::raw(format!("· {} ", fit(&m.subject, width.saturating_sub(used)))));
            }
        }
    }
    spans.push(Span::styled(format!("─ [{view}] "), t.title));
    Line::from(spans)
}

fn header(s: &AppState, d: &Displayed) -> Line<'static> {
    let mut parts: Vec<String> = Vec::new();
    match d {
        Displayed::Text { content, path, hl, hl_note, .. } => {
            let text = content.as_text().expect("text");
            parts.push(path.display().into_owned());
            parts.push(format!("{} lines", text.line_count()));
            if s.file.wrap {
                parts.push("wrap".into());
            }
            match (hl, hl_note) {
                (Some(h), _) => parts.push(h.syntax_name.clone()),
                (None, Some(n)) => parts.push((*n).to_string()),
                _ => {}
            }
            if text.has_crlf {
                parts.push("CRLF".into());
            }
            if text.invalid_utf8 {
                parts.push("non-UTF-8".into());
            }
            if s.view == ViewMode::Blame {
                parts.push("⏎ jump to commit".into());
            }
        }
        Displayed::Diff { diff, old_label, new_label, .. } => {
            parts.push(format!("{old_label}  →  {new_label}"));
            parts.push(format!("+{} −{}", diff.added, diff.removed));
            parts.push(format!("{} hunks", diff.hunks.len()));
            if s.file.wrap {
                parts.push("wrap".into());
            }
        }
        Displayed::DiffSummary { old_label, new_label, .. } => parts.push(format!("{old_label}  →  {new_label}")),
        Displayed::NotText { path, .. } => parts.push(path.display().into_owned()),
        _ => {
            if let Some(e) = s.selected_entry() {
                parts.push(e.path.display().into_owned());
            }
        }
    }
    if let Some(e) = s.selected_entry() {
        if let crate::model::ChangeKind::Renamed { from, similarity } = &e.change {
            if !matches!(d, Displayed::Diff { .. }) {
                parts.push(format!("renamed from {} ({similarity}%)", from.display()));
            }
        }
    }
    Line::from(parts.join(" · "))
}

/// Columns left of the text in the current view. Shared with the search's horizontal reveal.
pub fn gutter_cols(s: &AppState, d: &Displayed) -> usize {
    match d {
        Displayed::Text { content, .. } => match content.as_text() {
            Some(t) if s.view == ViewMode::Blame => blame_view::gutter_cols(s, t),
            Some(t) => content_view::gutter_cols(s, t),
            None => 0,
        },
        Displayed::Diff { old, new, .. } => diff_view::gutter_cols(s, old.as_text(), new.as_text()),
        _ => 0,
    }
}
