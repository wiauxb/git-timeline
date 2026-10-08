//! Modal overlays: confirmations, text input, commit details, errors.

use std::path::Path;

use ratatui::layout::{Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::theme::theme;
use super::{centered, help};
use crate::app::state::{AppState, Overlay, TextInput};
use crate::model::{Oid, RevisionId};

pub fn render(f: &mut Frame, area: Rect, s: &AppState, overlay: &Overlay) {
    match overlay {
        Overlay::Help { scroll } => help::render(f, area, s, *scroll),
        Overlay::Details { oid, scroll } => details(f, area, s, oid, *scroll),
        Overlay::ConfirmRestore { rev, overwrites_local_changes } => {
            confirm_restore(f, area, s, rev, *overwrites_local_changes)
        }
        Overlay::ExportPrompt { input, error, .. } => export_prompt(f, area, input, error.as_deref()),
        Overlay::ConfirmOverwrite { dest, .. } => confirm_overwrite(f, area, dest),
        Overlay::Error { message } => error(f, area, message),
    }
}

fn frame<'a>(f: &mut Frame, area: Rect, title: &'a str, lines: Vec<Line<'a>>, width: u16, danger: bool) -> Rect {
    let t = theme();
    let h = lines.len() as u16 + 2;
    let rect = centered(area, width, h);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::bordered()
                .title(format!(" {title} "))
                .border_style(if danger { t.danger } else { t.border_focused }),
        ),
        rect,
    );
    rect
}

fn confirm_restore(f: &mut Frame, area: Rect, s: &AppState, rev: &RevisionId, overwrites: bool) {
    let t = theme();
    let e = s.timeline.position_of(rev).map(|i| &s.timeline.entries[i]);
    let what = match e.and_then(|e| e.meta.as_ref()) {
        Some(m) => format!("{} \"{}\"", m.oid.short(), m.subject),
        None => e.map(|e| e.label()).unwrap_or_default(),
    };
    let mut lines = vec![
        Line::from(vec![Span::raw("Write "), Span::styled(what, t.hash)]),
        Line::from(format!("to {} in the working tree?", s.ctx.target.display())),
        Line::raw(""),
    ];
    if overwrites {
        lines.push(Line::from(Span::styled(
            format!("⚠ {} has uncommitted changes. They will be lost.", s.ctx.target.display()),
            t.danger,
        )));
        lines.push(Line::raw(""));
    }
    lines.push(Line::from(vec![
        Span::styled("The index is not modified.", t.dim),
        Span::raw("     "),
        Span::styled("[y] Yes", t.title),
        Span::raw("  "),
        Span::styled("[n] No", t.title),
    ]));
    frame(f, area, "Restore version", lines, 64, overwrites);
}

fn export_prompt(f: &mut Frame, area: Rect, input: &TextInput, err: Option<&str>) {
    let t = theme();
    let mut lines = vec![
        Line::from("Save this version as (relative to where you launched git timeline):"),
        Line::from(Span::styled(format!("> {} ", input.value), t.title)),
    ];
    if let Some(e) = err {
        lines.push(Line::from(Span::styled(e.to_string(), t.danger)));
    }
    lines.push(Line::from(Span::styled("Enter save · Esc cancel", t.dim)));
    let rect = frame(f, area, "Export", lines, 70, false);
    // terminal cursor at the insertion point
    let col: usize = input.value.chars().take(input.cursor).map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)).sum();
    let x = rect.x + 1 + 2 + col as u16;
    if x < rect.right().saturating_sub(1) {
        f.set_cursor_position(Position { x, y: rect.y + 2 });
    }
}

fn confirm_overwrite(f: &mut Frame, area: Rect, dest: &Path) {
    let t = theme();
    let lines = vec![
        Line::from(format!("{} already exists.", dest.display())),
        Line::from(vec![Span::raw("Overwrite?  "), Span::styled("[y] Yes  [n] No", t.title)]),
    ];
    frame(f, area, "Export", lines, 70, true);
}

fn error(f: &mut Frame, area: Rect, msg: &str) {
    let t = theme();
    let lines = vec![Line::from(msg.to_string()), Line::raw(""), Line::from(Span::styled("press any key", t.dim))];
    frame(f, area, "Error", lines, 70, true);
}

fn details(f: &mut Frame, area: Rect, s: &AppState, oid: &Oid, scroll: usize) {
    let t = theme();
    let mut lines: Vec<Line> = Vec::new();
    let meta = s
        .timeline
        .position_of(&RevisionId::Commit(oid.clone()))
        .and_then(|i| s.timeline.entries[i].meta.clone());
    let cached = s.cache.details.peek(oid).cloned();

    let mut first = vec![Span::styled(format!("commit {oid}"), t.hash)];
    if let Some(Ok(d)) = &cached {
        if !d.refs.is_empty() {
            first.push(Span::styled(format!("  ({})", d.refs.join(", ")), t.pseudo_entry));
        }
    }
    lines.push(Line::from(first));
    if let Some(m) = &meta {
        let sig = |label: &str, sg: &crate::model::Signature| {
            Line::from(vec![
                Span::styled(format!("{label:<11}"), t.dim),
                Span::styled(format!("{} <{}>", sg.name, sg.email), t.author),
                Span::styled(format!("  {}", sg.time.format("%Y-%m-%d %H:%M %z")), t.date),
            ])
        };
        lines.push(sig("Author:", &m.author));
        lines.push(sig("Committer:", &m.committer));
        let parents: Vec<_> = m.parents.iter().map(|p| p.short().to_string()).collect();
        lines.push(Line::from(vec![Span::styled("Parents:   ", t.dim), Span::raw(parents.join(" "))]));
    }
    lines.push(Line::raw(""));
    match &cached {
        None => lines.push(Line::from(format!("{} loading…", s.spinner()))),
        Some(Err(e)) => lines.push(Line::from(Span::styled(e.clone(), t.danger))),
        Some(Ok(d)) => {
            for l in d.body.lines() {
                lines.push(Line::raw(format!("    {l}")));
            }
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                format!("Files changed ({}{}):", d.files.len(), if d.files_truncated { "+" } else { "" }),
                t.title,
            )));
            for (st, p) in &d.files {
                let style = match st {
                    'A' => t.change_added,
                    'D' => t.change_deleted,
                    'R' | 'C' => t.change_renamed,
                    _ => t.change_other,
                };
                let is_target = s.timeline.entries.iter().any(|e| e.oid() == Some(oid) && &e.path == p);
                let pstyle = if is_target { t.title } else { ratatui::style::Style::default() };
                lines.push(Line::from(vec![
                    Span::styled(format!(" {st} "), style),
                    Span::styled(p.display().into_owned(), pstyle),
                ]));
            }
        }
    }

    let h = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let rect = centered(area, 100, h);
    let max_scroll = lines.len().saturating_sub(rect.height.saturating_sub(2) as usize);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll.min(max_scroll) as u16, 0))
            .block(Block::bordered().title(" Commit details · Esc close ").border_style(t.border_focused)),
        rect,
    );
}
