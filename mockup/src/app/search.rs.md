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

/// Matching row indices of the current file view, ascending. `key` = what they were
/// computed on; a different key ⇒ recompute.
#[derive(Default)]
pub struct FileHits { pub key: Option<FileHitsKey>, pub rows: Vec<usize> }

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
    pub fn hits(&self, scope: Scope) -> &[usize];
}
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
/// Searchable text of a file-view row: Text ⇒ the line, Diff ⇒ the line of the
/// row's side; hunk headers ⇒ None. Used for the hit lists and the horizontal reveal
/// (renderers search their own line text via `content_view::search_marks`).
pub fn row_text(d: &Displayed, row: usize) -> Option<Cow<'_, str>>;
pub fn entry_matches(q: &Query, e: &TimelineEntry) -> bool;
fn file_hits_key(d: &Displayed) -> Option<FileHitsKey>;   // Diff ⇒ DiffKey{old_key, new_key}
fn set_query(s: &mut AppState, scope: Scope, q: Option<Arc<Query>>);   // + reset that scope's hits
fn origin_row(s: &AppState, o: &Origin) -> usize;
fn restore_origin(s: &mut AppState, o: &Origin);
fn jump_file(s: &mut AppState, row: usize);        // cursor + vertical/horizontal reveal
```

## What matches
* **Timeline entry**: commit ⇒ `q.hash_prefix(full oid)` or `q.is_match(subject)`;
  pseudo-entry ⇒ `q.is_match(label)` (`Working tree` / `Staged`). Author is not searched.
* **File row** (all views): Content / Blame ⇒ each text line (`line_str`, lossy UTF-8;
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
    jump to the first hit at or after `origin` (wrapping around). No hit ⇒ return to
    `origin`.
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
* Next hit strictly after (or before) the current row (timeline: `selected_idx`; file:
  `cursor`). Past the end it **wraps** with Info "search wrapped to top" / "… to
  bottom". This works across one revision only: it never moves to another revision.
* Timeline jump ⇒ `update::select(idx)`. Content follows through the normal anchoring.
* File jump (`jump_file`) ⇒ `cursor = row`, `pending_anchor = None`, vertical scroll as
  for a cursor move. **Horizontal reveal**: if the first match on the row is not fully
  inside `[hscroll, hscroll + text cols)` (columns from
  `ui::content_view::col_of_byte`, text cols = `file_cols() −
  ui::file_pane::gutter_cols(s, d)`), then `hscroll = start col − 8` (saturating).

## Hit lists (`refresh`, run by `after_batch` and `edited`)
* Timeline: no query ⇒ empty. Otherwise it scans `entries[scanned..]`, appends the
  matching indices and sets `scanned = len`. `reset_timeline_hits` (reload,
  `LocalState`, query change) ⇒ rescan from 0. That's needed because pseudo-entries
  shift the indices.
* File: no query, or `file_hits_key(displayed())` is `None` ⇒ empty. Otherwise
  recompute when the key differs (new revision, view switch between text and diff, new
  diff) or the query changed (`key = None`). Content and Blame share a key (same rows).
* Revision changes keep both searches active: the file hits are recomputed for the new
  version, and the cursor stays on the logical line given by the anchor. It does not
  jump to a match; `n` does that.

## Clearing
* `Esc` in the timeline ⇒ `clear(Timeline)` if there is a search (Info "search
  cleared").
* `Esc` in the file pane ⇒ `clear(File)` if there is a file search, otherwise
  `FocusTimeline` (as before).
* Confirming an empty prompt clears too.
