//! Left pane: the list of revisions.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use super::theme::theme;
use super::{age, fit};
use crate::app::state::{AppState, Focus};
use crate::model::{ChangeKind, RevisionId, TimelineEntry};

pub fn render(f: &mut Frame, area: Rect, s: &AppState) {
    let t = theme();
    let focused = s.focus == Focus::Timeline;
    let mut title = vec![
        Span::styled(" Timeline ", t.title),
        Span::raw(format!("─ {} ({})", s.ctx.target.display(), s.timeline.len())),
    ];
    if let Some(l) = &s.ctx.query.line_range {
        title.push(Span::styled(format!(" lines {l}"), t.dim));
    }
    if s.timeline_loading {
        title.push(Span::raw(format!(" {} ", s.spinner())));
    } else {
        title.push(Span::raw(" "));
    }
    let block = Block::bordered()
        .title(Line::from(title))
        .border_style(if focused { t.border_focused } else { t.border });
    let inner = block.inner(area);
    f.render_widget(block, area);

    if s.timeline.is_empty() {
        let msg = if s.timeline_loading { "loading history…" } else { "no revisions" };
        f.render_widget(
            Paragraph::new(msg).style(t.dim).centered(),
            Rect { y: inner.y + inner.height / 2, height: 1, ..inner },
        );
        return;
    }

    let now = (s.clock)();
    let width = inner.width as usize;
    let lines: Vec<Line> = s
        .timeline
        .entries
        .iter()
        .enumerate()
        .skip(s.list_offset)
        .take(inner.height as usize)
        .map(|(i, e)| {
            let selected = s.selected.is_some() && i == s.selected_idx;
            let marked = s.mark.as_ref() == Some(&e.rev);
            let mut line = row(e, marked, selected, width, now);
            if selected {
                line = line.style(if focused { t.selected_row } else { t.selected_row_unfocused });
            }
            line
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn change_span(c: &ChangeKind, conflicted: bool) -> Span<'static> {
    let t = theme();
    let (text, style) = match c {
        _ if conflicted => ("!", t.change_deleted),
        ChangeKind::Added | ChangeKind::Untracked => ("A", t.change_added),
        ChangeKind::Deleted => ("D", t.change_deleted),
        ChangeKind::Renamed { .. } => ("R", t.change_renamed),
        ChangeKind::Copied { .. } => ("C", t.change_renamed),
        ChangeKind::TypeChanged => ("T", t.change_other),
        ChangeKind::Merge => ("⑂", t.change_other),
        ChangeKind::Modified => (" ", t.dim),
    };
    Span::styled(text, style)
}

fn row(e: &TimelineEntry, marked: bool, selected: bool, width: usize, now: chrono::DateTime<chrono::Local>) -> Line<'static> {
    let t = theme();
    let marker = if marked {
        Span::styled("◆", t.mark)
    } else if e.is_local() {
        Span::styled("●", t.pseudo_entry)
    } else {
        Span::raw(" ")
    };
    let mut spans = vec![marker];
    let show_author = width >= 45;

    match &e.rev {
        RevisionId::Commit(oid) => {
            let meta = e.meta.as_ref().expect("commits have metadata");
            spans.push(Span::styled(oid.short().to_string(), t.hash));
            spans.push(Span::styled(format!(" {:>4} ", age(now, &meta.author.time)), t.date));
            if show_author {
                spans.push(Span::styled(format!("{:<12} ", fit(&meta.author.name, 12)), t.author));
            }
            spans.push(change_span(&e.change, e.conflicted));
            spans.push(Span::raw(" "));
            let used: usize = spans.iter().map(|s| s.width()).sum();
            let mut subject = meta.subject.clone();
            if selected {
                if let ChangeKind::Renamed { from, .. } | ChangeKind::Copied { from, .. } = &e.change {
                    subject = format!("{subject}  ← {}", from.display());
                }
            }
            spans.push(Span::styled(fit(&subject, width.saturating_sub(used)), t.subject));
        }
        _ => {
            let label = format!("{:<12}", e.label());
            spans.push(Span::styled(label, t.pseudo_entry));
            if show_author {
                spans.push(Span::raw(" ".repeat(13)));
            } else {
                spans.push(Span::raw(" "));
            }
            spans.push(change_span(&e.change, e.conflicted));
            let what = match (&e.change, e.conflicted) {
                (_, true) => "conflicted",
                (ChangeKind::Untracked, _) => "untracked",
                (ChangeKind::Added, _) => "new file",
                (ChangeKind::Deleted, _) => "deleted",
                _ => "modified",
            };
            spans.push(Span::styled(format!(" {what}"), t.dim));
        }
    }
    Line::from(spans)
}
