# `src/app/search.rs`

## Purpose
Search inside the UI (`/`). The search applies to the **focused pane**: in the timeline
it matches commit hashes and subjects; in the file pane it matches the displayed lines.
Each pane has its own independent search. The search **jumps and highlights**: nothing
is hidden or filtered. This file holds the state, the prompt flow and the hit lists. The
reducer (`update.rs`) delegates to it.

## Contents

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope { Timeline, File }

#[derive(Default)]
pub struct SearchState {
    pub timeline: Option<Arc<Query>>,
    pub file: Option<Arc<Query>>,
    pub timeline_hits: TimelineHits,
    pub file_hits: FileHits,
}

/// Matching entry indices, ascending. Computed incrementally, because commits only get
/// appended while the history streams.
#[derive(Default)]
pub struct TimelineHits { pub scanned: usize, pub rows: Vec<usize> }

/// Every match of the current file view, in reading order: (row, byte range in
/// `row_text`). `key` = what they were computed on; a different key ⇒ recompute.
/// `current` = index of the match `n`/`N` last landed on.
#[derive(Default)]
pub struct FileHits {
    pub key: Option<FileHitsKey>,
    pub matches: Vec<FileMatch>,
    pub current: Option<usize>,
}
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FileMatch { pub row: usize, pub bytes: Range<usize> }

#[derive(Clone, PartialEq, Eq)]
pub enum FileHitsKey { Text(ContentKey), Diff(DiffKey) }

/// Overlay state while the prompt is open (`Overlay::Search`).
pub struct SearchPrompt {
    pub scope: Scope,
    pub input: TextInput,
    pub previous: Option<Arc<Query>>,   // restored on Esc
    pub origin: Origin,                 // restored on Esc; live jumps start from here
    pub error: Option<String>,          // invalid regex
}
#[derive(Clone)]
pub enum Origin {
    Timeline { selected: Option<RevisionId>, idx: usize },
    File { cursor: usize, top: usize, hscroll: usize },
}

impl SearchState {
    pub fn query(&self, scope: Scope) -> Option<&Arc<Query>>;
    /// Hit count: timeline rows / file matches.
    pub fn hit_count(&self, scope: Scope) -> usize;
}
/// 0-based index of the current hit (timeline: the selected row if it matches; file:
/// `current` if its row is the cursor row), for the `k/M` counter.
pub fn current_hit(s: &AppState, scope: Scope) -> Option<usize>;
/// The current match, if it is on the cursor row (else None: the cursor moved away).
pub fn current_match(s: &AppState) -> Option<&FileMatch>;
pub fn scope_of(f: Focus) -> Scope;
pub fn current_row(s: &AppState, scope: Scope) -> usize;   // selected_idx / file.cursor

pub fn start(s: &mut AppState);                    // `/`
pub fn edited(s: &mut AppState);                   // after every prompt input change
pub fn confirm(s: &mut AppState);                  // Enter
pub fn cancel(s: &mut AppState);                   // Esc in the prompt
pub fn next(s: &mut AppState, forward: bool);      // n / N
pub fn clear(s: &mut AppState, scope: Scope);      // Esc outside the prompt
pub fn refresh(s: &mut AppState);                  // from update::after_batch
pub fn reset_timeline_hits(s: &mut AppState);      // reload / local state replaced
pub fn entry_matches(q: &Query, e: &TimelineEntry) -> bool;
fn file_hits_key(d: &Displayed) -> Option<FileHitsKey>;   // Diff ⇒ DiffKey{old_key, new_key}
fn set_query(s: &mut AppState, scope: Scope, q: Option<Arc<Query>>);   // + reset that scope's hits
fn origin_row(s: &AppState, o: &Origin) -> usize;
fn restore_origin(s: &mut AppState, o: &Origin);
fn jump_file(s: &mut AppState, m: usize);          // current = m; cursor + reveal of that match
```

## What matches
* **Timeline entry**: commit ⇒ `q.hash_prefix(full oid)` or `q.is_match(subject)`;
  pseudo-entry ⇒ `q.is_match(label)` (`Working tree` / `Staged`). Author is not searched.
* **File row** (all views, text from `Displayed::row_text`): Content / Blame ⇒ each text line (`line_str`, lossy UTF-8;
  the blame gutter is not searched). Diff views ⇒ each `Line` row, text taken from the
  side it is rendered from (old for removed rows, new for context/added). Hunk headers
  are skipped. Non-text displays (`NotText`, `DiffSummary`, `Waiting`, …) have no rows
  and no hits.

## Prompt flow (incremental)
* `start`: `scope` = focused pane. Opens `Overlay::Search` with an empty input, the
  current query of that scope as `previous`, and the current position as `origin`.
* `edited` (every `InputChar` / `InputBackspace` / `InputDelete` / `InputPaste`):
  * empty input ⇒ the scope's query = `None`, return to `origin`;
  * compile error ⇒ `error` set, the scope's query = `None` (no highlights), position
    unchanged;
  * ok ⇒ the scope's query = new query, hits recomputed **right away** (`refresh`), then
    jump to the first hit at or after `origin` (wrapping around; in the file, the first
    match whose row ≥ the origin cursor row). No hit ⇒ return to `origin`.
* `confirm`: invalid regex ⇒ the prompt stays open. Empty ⇒ close and clear the scope's
  search (Info "search cleared" if there was one). No hit ⇒ close, keep the query, Warn
  "pattern not found: <q>". Otherwise close.
* `cancel`: restore `previous` as the scope's query, restore `origin` (timeline:
  re-select `selected` by `RevisionId`, fallback `idx`; file: cursor/top/hscroll), close.
* While the prompt is open, the panes render normally with the live highlights. Other
  keys and the mouse are swallowed, like any overlay.

## Navigation
* `next(forward)` uses the **focused pane's** search. No query ⇒ Info "no search — press
  / to search". No hit ⇒ Warn "pattern not found: <q>".
* **Timeline: by row.** Next matching row strictly after (or before) `selected_idx`,
  then `update::select(idx)`. Content follows through the normal anchoring.
* **File: by match, not by row.** A line with several matches is visited once per
  match, in reading order:
  * if `current_match()` is some (the cursor is still on its row) ⇒ `current ± 1`;
  * otherwise the cursor is treated as standing **before the start** of its row: `n` ⇒
    the first match with `row ≥ cursor`, `N` ⇒ the last match with `row < cursor`.
* Past the end it **wraps** with Info "search wrapped to top" / "… to bottom". This
  works across one revision only: it never moves to another revision.
* File jump (`jump_file(m)`) ⇒ `current = m`, `cursor = row`, `pending_anchor = None`,
  then **reveal that match**:
  * vertically: `viewport::reveal(s, d, row, segment)`, where `segment` = the wrapped
    screen row holding the match start (`wrap_breaks`; 0 when wrap is off). For a line
    taller than the body this scrolls *inside* the line (`file.top_skip`), so every
    match of a very long line comes on screen in turn;
  * horizontally (wrap off only): if the match is not fully inside `[hscroll, hscroll
    + text cols)` (columns from `ui::content_view::col_of_byte`, text cols =
    `viewport::text_cols`), then `hscroll = start col − 8` (saturating).
* Moving the cursor by other means doesn't clear `current`, but it stops being
  "current" (`current_match()` is None) once the cursor leaves its row.
* The current match is drawn in `search_current` (stronger than `search_match`), so
  you can tell which of several matches on one line `n` stopped at.

## Hit lists (`refresh`, run by `after_batch` and `edited`)
* Timeline: no query ⇒ empty. Otherwise it scans `entries[scanned..]`, appends the
  matching indices and sets `scanned = len`. `reset_timeline_hits` (reload,
  `LocalState`, query change) ⇒ rescan from 0. That's needed because pseudo-entries
  shift the indices.
* File: no query, or `file_hits_key(displayed())` is `None` ⇒ empty. Otherwise
  recompute when the key differs (new revision, view switch between text and diff, new
  diff) or the query changed (`key = None`): every `find_iter` range of every row.
  Recomputing resets `current = None`. Content and Blame share a key (same rows).
* Revision changes keep both searches active: the file hits are recomputed for the new
  version, and the cursor stays on the logical line given by the anchor. It does not
  jump to a match; `n` does that.

## Clearing
* `Esc` in the timeline ⇒ `clear(Timeline)` if there is a search (Info "search
  cleared").
* `Esc` in the file pane ⇒ `clear(File)` if there is a file search, otherwise
  `FocusTimeline` (as before).
* Confirming an empty prompt clears too.
