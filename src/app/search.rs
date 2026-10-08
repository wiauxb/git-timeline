//! `/` search of the focused pane: timeline (hash prefix + subject) or file view (displayed
//! lines). Jumps and highlights, never filters. One query per pane.

use std::borrow::Cow;
use std::sync::Arc;

use super::state::{AppState, Displayed, Focus, Level, Overlay, TextInput};
use super::update::{info, scroll_to_cursor, select};
use crate::model::{ContentKey, DiffKey, DiffLine, DiffRow, Query, RevisionId, TimelineEntry};
use crate::ui::{content_view::col_of_byte, file_pane::gutter_cols};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    Timeline,
    File,
}

#[derive(Default, Debug)]
pub struct SearchState {
    pub timeline: Option<Arc<Query>>,
    pub file: Option<Arc<Query>>,
    pub timeline_hits: TimelineHits,
    pub file_hits: FileHits,
}

/// Matching entry indices, ascending; commits only get appended while streaming.
#[derive(Default, Debug)]
pub struct TimelineHits {
    pub scanned: usize,
    pub rows: Vec<usize>,
}

/// Matching rows of the current file view, ascending, computed on `key`.
#[derive(Default, Debug)]
pub struct FileHits {
    pub key: Option<FileHitsKey>,
    pub rows: Vec<usize>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FileHitsKey {
    Text(ContentKey),
    Diff(DiffKey),
}

#[derive(Debug)]
pub struct SearchPrompt {
    pub scope: Scope,
    pub input: TextInput,
    pub previous: Option<Arc<Query>>,
    pub origin: Origin,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Origin {
    Timeline { selected: Option<RevisionId>, idx: usize },
    File { cursor: usize, top: usize, hscroll: usize },
}

impl SearchState {
    pub fn query(&self, scope: Scope) -> Option<&Arc<Query>> {
        match scope {
            Scope::Timeline => self.timeline.as_ref(),
            Scope::File => self.file.as_ref(),
        }
    }

    pub fn hits(&self, scope: Scope) -> &[usize] {
        match scope {
            Scope::Timeline => &self.timeline_hits.rows,
            Scope::File => &self.file_hits.rows,
        }
    }
}

pub fn scope_of(f: Focus) -> Scope {
    match f {
        Focus::Timeline => Scope::Timeline,
        Focus::File => Scope::File,
    }
}

/// The row a scope's navigation starts from.
pub fn current_row(s: &AppState, scope: Scope) -> usize {
    match scope {
        Scope::Timeline => s.selected_idx,
        Scope::File => s.file.cursor,
    }
}

fn set_query(s: &mut AppState, scope: Scope, q: Option<Arc<Query>>) {
    match scope {
        Scope::Timeline => {
            s.search.timeline = q;
            s.search.timeline_hits = TimelineHits::default();
        }
        Scope::File => {
            s.search.file = q;
            s.search.file_hits = FileHits::default();
        }
    }
    s.needs_requests = true;
}

fn prompt(s: &mut AppState) -> Option<&mut SearchPrompt> {
    match &mut s.overlay {
        Some(Overlay::Search(p)) => Some(p),
        _ => None,
    }
}

pub fn start(s: &mut AppState) {
    let scope = scope_of(s.focus);
    let origin = match scope {
        Scope::Timeline => Origin::Timeline { selected: s.selected.clone(), idx: s.selected_idx },
        Scope::File => Origin::File { cursor: s.file.cursor, top: s.file.top, hscroll: s.file.hscroll },
    };
    let previous = s.search.query(scope).cloned();
    s.overlay = Some(Overlay::Search(SearchPrompt { scope, input: TextInput::default(), previous, origin, error: None }));
}

/// After every change of the prompt's text: live highlights and jump.
pub fn edited(s: &mut AppState) {
    let Some(p) = prompt(s) else { return };
    let (scope, pattern, origin) = (p.scope, p.input.value.clone(), p.origin.clone());
    if pattern.is_empty() {
        p.error = None;
        set_query(s, scope, None);
        restore_origin(s, &origin);
        return;
    }
    match Query::compile(&pattern) {
        Err(e) => {
            p.error = Some(e);
            set_query(s, scope, None);
        }
        Ok(q) => {
            p.error = None;
            set_query(s, scope, Some(Arc::new(q)));
            refresh(s);
            let from = origin_row(s, &origin);
            let hits = s.search.hits(scope);
            match hits.iter().copied().find(|&h| h >= from).or(hits.first().copied()) {
                Some(row) => jump(s, scope, row),
                None => restore_origin(s, &origin),
            }
        }
    }
}

pub fn confirm(s: &mut AppState) {
    let Some(p) = prompt(s) else { return };
    if p.error.is_some() {
        return; // an invalid regex can't be committed
    }
    let (scope, empty, had) = (p.scope, p.input.value.is_empty(), p.previous.is_some());
    s.overlay = None;
    if empty {
        set_query(s, scope, None);
        if had {
            info(s, Level::Info, "search cleared");
        }
        return;
    }
    refresh(s);
    if s.search.hits(scope).is_empty() {
        let q = s.search.query(scope).map(|q| q.pattern().to_string()).unwrap_or_default();
        info(s, Level::Warn, format!("pattern not found: {q}"));
    }
}

pub fn cancel(s: &mut AppState) {
    let Some(Overlay::Search(p)) = s.overlay.take() else { return };
    set_query(s, p.scope, p.previous);
    restore_origin(s, &p.origin);
}

fn origin_row(s: &AppState, origin: &Origin) -> usize {
    match origin {
        Origin::Timeline { selected, idx } => selected.as_ref().and_then(|r| s.timeline.position_of(r)).unwrap_or(*idx),
        Origin::File { cursor, .. } => *cursor,
    }
}

fn restore_origin(s: &mut AppState, origin: &Origin) {
    match *origin {
        Origin::Timeline { .. } => {
            if !s.timeline.is_empty() {
                let i = origin_row(s, origin).min(s.timeline.len() - 1);
                select(s, i);
            }
        }
        Origin::File { cursor, top, hscroll } => {
            s.file.pending_anchor = None;
            s.file.cursor = cursor;
            s.file.top = top;
            s.file.hscroll = hscroll;
        }
    }
}

/// `n` / `N` in the focused pane.
pub fn next(s: &mut AppState, forward: bool) {
    let scope = scope_of(s.focus);
    let Some(q) = s.search.query(scope).cloned() else {
        info(s, Level::Info, "no search — press / to search");
        return;
    };
    refresh(s);
    let hits = s.search.hits(scope);
    if hits.is_empty() {
        info(s, Level::Warn, format!("pattern not found: {}", q.pattern()));
        return;
    }
    let cur = current_row(s, scope);
    let (target, wrapped) = if forward {
        hits.iter().copied().find(|&h| h > cur).map(|h| (h, false)).unwrap_or((hits[0], true))
    } else {
        hits.iter().rev().copied().find(|&h| h < cur).map(|h| (h, false)).unwrap_or((hits[hits.len() - 1], true))
    };
    jump(s, scope, target);
    if wrapped {
        info(s, Level::Info, if forward { "search wrapped to top" } else { "search wrapped to bottom" });
    }
}

/// Esc outside the prompt. False when there was no search to clear.
pub fn clear(s: &mut AppState, scope: Scope) -> bool {
    if s.search.query(scope).is_none() {
        return false;
    }
    set_query(s, scope, None);
    info(s, Level::Info, "search cleared");
    true
}

fn jump(s: &mut AppState, scope: Scope, row: usize) {
    match scope {
        Scope::Timeline => select(s, row),
        Scope::File => jump_file(s, row),
    }
}

fn jump_file(s: &mut AppState, row: usize) {
    s.file.pending_anchor = None;
    s.file.cursor = row;
    scroll_to_cursor(s);
    // horizontal reveal of the first match on the row
    let Some(q) = s.search.file.clone() else { return };
    let d = s.displayed();
    let Some(text) = row_text(&d, row) else { return };
    let Some(m) = q.find_iter(&text).next() else { return };
    let tab = s.opts.tab_width;
    let (start, end) = (col_of_byte(&text, m.start, tab), col_of_byte(&text, m.end, tab));
    let cols = s.file_cols().saturating_sub(gutter_cols(s, &d)).max(1);
    if start < s.file.hscroll || end > s.file.hscroll + cols {
        s.file.hscroll = start.saturating_sub(8);
    }
}

/// Rescan after a reload or new local state (pseudo-entries shift the indices).
pub fn reset_timeline_hits(s: &mut AppState) {
    s.search.timeline_hits = TimelineHits::default();
}

/// Keep the hit lists in step with the timeline and the displayed view.
pub fn refresh(s: &mut AppState) {
    match s.search.timeline.clone() {
        None => s.search.timeline_hits = TimelineHits::default(),
        Some(q) => {
            let h = &mut s.search.timeline_hits;
            if h.scanned > s.timeline.len() {
                *h = TimelineHits::default();
            }
            for (i, e) in s.timeline.entries.iter().enumerate().skip(h.scanned) {
                if entry_matches(&q, e) {
                    h.rows.push(i);
                }
            }
            h.scanned = s.timeline.len();
        }
    }

    let Some(q) = s.search.file.clone() else {
        s.search.file_hits = FileHits::default();
        return;
    };
    let d = s.displayed();
    let key = file_hits_key(&d);
    if key.is_none() {
        s.search.file_hits = FileHits::default();
    } else if key != s.search.file_hits.key {
        let rows = (0..d.row_count().unwrap_or(0))
            .filter(|&r| row_text(&d, r).is_some_and(|t| q.is_match(&t)))
            .collect();
        s.search.file_hits = FileHits { key, rows };
    }
}

pub fn entry_matches(q: &Query, e: &TimelineEntry) -> bool {
    match (e.oid(), &e.meta) {
        (Some(oid), Some(meta)) => q.hash_prefix(oid.as_str()).is_some() || q.is_match(&meta.subject),
        _ => q.is_match(&e.label()),
    }
}

/// Searchable text of a file-view row (hunk headers and non-text: none).
pub fn row_text(d: &Displayed, row: usize) -> Option<Cow<'_, str>> {
    match d {
        Displayed::Text { content, .. } => {
            let t = content.as_text()?;
            (row < t.line_count()).then(|| t.line_str(row))
        }
        Displayed::Diff { rows, old, new, .. } => match rows.get(row)? {
            DiffRow::HunkHeader(_) => None,
            DiffRow::Line(DiffLine::Removed { old: l }) => Some(old.as_text()?.line_str(*l)),
            DiffRow::Line(DiffLine::Context { new: l, .. } | DiffLine::Added { new: l }) => {
                Some(new.as_text()?.line_str(*l))
            }
        },
        _ => None,
    }
}

fn file_hits_key(d: &Displayed) -> Option<FileHitsKey> {
    match d {
        Displayed::Text { key, .. } => Some(FileHitsKey::Text(key.clone())),
        Displayed::Diff { old_key, new_key, .. } => {
            Some(FileHitsKey::Diff(DiffKey { old: old_key.clone(), new: new_key.clone() }))
        }
        _ => None,
    }
}
