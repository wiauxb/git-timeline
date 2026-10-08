//! `/` search of the focused pane: timeline (hash prefix + subject) or file view (displayed
//! lines). Jumps and highlights, never filters. One query per pane.

use std::ops::Range;
use std::sync::Arc;

use super::state::{AppState, Displayed, Focus, Level, Overlay, TextInput};
use super::update::{info, select};
use super::viewport;
use crate::model::{ContentKey, DiffKey, Query, RevisionId, TimelineEntry};
use crate::ui::content_view::{col_of_byte, wrap_breaks};

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

/// Every match of the current file view in reading order, computed on `key`.
/// `current` = the match `n` / `N` last landed on.
#[derive(Default, Debug)]
pub struct FileHits {
    pub key: Option<FileHitsKey>,
    pub matches: Vec<FileMatch>,
    pub current: Option<usize>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FileMatch {
    pub row: usize,
    /// Byte range in the row's text (`Displayed::row_text`).
    pub bytes: Range<usize>,
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

    /// Matching rows in the timeline, matches in the file.
    pub fn hit_count(&self, scope: Scope) -> usize {
        match scope {
            Scope::Timeline => self.timeline_hits.rows.len(),
            Scope::File => self.file_hits.matches.len(),
        }
    }
}

/// 0-based index of the current hit, for the `k/M` counter.
pub fn current_hit(s: &AppState, scope: Scope) -> Option<usize> {
    match scope {
        Scope::Timeline => s.search.timeline_hits.rows.binary_search(&s.selected_idx).ok(),
        Scope::File => current_match(s).and(s.search.file_hits.current),
    }
}

/// The current match, if the cursor is still on its row.
pub fn current_match(s: &AppState) -> Option<&FileMatch> {
    let h = &s.search.file_hits;
    h.current.and_then(|i| h.matches.get(i)).filter(|m| m.row == s.file.cursor)
}

pub fn scope_of(f: Focus) -> Scope {
    match f {
        Focus::Timeline => Scope::Timeline,
        Focus::File => Scope::File,
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
            let target = match scope {
                Scope::Timeline => {
                    let rows = &s.search.timeline_hits.rows;
                    rows.iter().copied().find(|&h| h >= from).or(rows.first().copied())
                }
                Scope::File => {
                    let all = &s.search.file_hits.matches;
                    (!all.is_empty()).then(|| all.iter().position(|m| m.row >= from).unwrap_or(0))
                }
            };
            match target {
                Some(t) => jump(s, scope, t),
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
    if s.search.hit_count(scope) == 0 {
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
            viewport::set_top(s, top);
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
    let n = s.search.hit_count(scope);
    if n == 0 {
        info(s, Level::Warn, format!("pattern not found: {}", q.pattern()));
        return;
    }
    let found = match scope {
        // timeline: by row
        Scope::Timeline => {
            let (rows, cur) = (&s.search.timeline_hits.rows, s.selected_idx);
            if forward {
                rows.iter().copied().find(|&h| h > cur)
            } else {
                rows.iter().rev().copied().find(|&h| h < cur)
            }
        }
        // file: by match; without a current match the cursor stands before its row's start
        Scope::File => {
            let all = &s.search.file_hits.matches;
            let cur = s.file.cursor;
            match (current_match(s).and(s.search.file_hits.current), forward) {
                (Some(i), true) => (i + 1 < n).then_some(i + 1),
                (Some(i), false) => i.checked_sub(1),
                (None, true) => all.iter().position(|m| m.row >= cur),
                (None, false) => all.iter().rposition(|m| m.row < cur),
            }
        }
    };
    let wrapped = found.is_none();
    let target = found.unwrap_or_else(|| match (scope, forward) {
        (Scope::Timeline, true) => s.search.timeline_hits.rows[0],
        (Scope::Timeline, false) => s.search.timeline_hits.rows[n - 1],
        (Scope::File, true) => 0,
        (Scope::File, false) => n - 1,
    });
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

/// Timeline: `target` is a row; file: an index into the match list.
fn jump(s: &mut AppState, scope: Scope, target: usize) {
    match scope {
        Scope::Timeline => select(s, target),
        Scope::File => jump_file(s, target),
    }
}

fn jump_file(s: &mut AppState, m: usize) {
    let Some(FileMatch { row, bytes }) = s.search.file_hits.matches.get(m).cloned() else { return };
    s.search.file_hits.current = Some(m);
    s.file.pending_anchor = None;
    s.file.cursor = row;
    let d = s.displayed();
    let Some(text) = d.row_text(row) else { return viewport::scroll_to_cursor(s) };
    let tab = s.opts.tab_width;
    let cols = viewport::text_cols(s, &d);
    if s.file.wrap {
        // the wrapped screen row holding the match start
        let segment = wrap_breaks(&text, cols, tab).partition_point(|&b| b <= bytes.start);
        viewport::reveal(s, &d, row, segment);
    } else {
        viewport::scroll_to_cursor(s);
        let (start, end) = (col_of_byte(&text, bytes.start, tab), col_of_byte(&text, bytes.end, tab));
        if start < s.file.hscroll || end > s.file.hscroll + cols {
            s.file.hscroll = start.saturating_sub(8);
        }
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
        let mut matches = Vec::new();
        for row in 0..d.row_count().unwrap_or(0) {
            if let Some(t) = d.row_text(row) {
                matches.extend(q.find_iter(&t).map(|bytes| FileMatch { row, bytes }));
            }
        }
        s.search.file_hits = FileHits { key, matches, current: None };
    }
}

pub fn entry_matches(q: &Query, e: &TimelineEntry) -> bool {
    match (e.oid(), &e.meta) {
        (Some(oid), Some(meta)) => q.hash_prefix(oid.as_str()).is_some() || q.is_match(&meta.subject),
        _ => q.is_match(&e.label()),
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
