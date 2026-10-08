# `src/app/update.rs`

## Purpose
The reducer. Synchronous, deterministic, IO-free: mutates `AppState` and returns
`Effect`s. Unit/integration-tested by feeding actions (`tests/views.rs` runs jobs inline).

## Contents

```rust
const PREFETCH: usize = 2;      // neighbours ±2 prefetched
const LIST_MARGIN: usize = 2;   // rows kept above/below the selection

pub fn update(s: &mut AppState, a: Action) -> Vec<Effect>;
pub fn after_batch(s: &mut AppState) -> Vec<Effect>;   // once per event batch
pub fn info(s: &mut AppState, level: Level, text: impl Into<String>);
pub fn select(s: &mut AppState, idx: usize);
// private: select_delta, capture_anchor, ensure_list_visible, reselect, move_cursor,
// scroll_to_cursor, clamp_cursor, jump_hunk, blame_jump, set_view, toggle_mark,
// show_details, selected_bytes, start_export, start_restore, resolve_dest,
// overlay_action, op_done, reload, job_done, want_content, want_highlight, want_diff,
// request_missing, prefetch, resolve_anchor, touch_displayed
```

## `update`
* Sets `dirty = true` (except `Tick`, which only redraws for the spinner/expiry).
* If an overlay is open, `overlay_action` handles the action first; it lets through only
  `JobDone`, `OpDone`, `Layout`, `Tick`, `Quit` and swallows everything else.
* Status messages: `info()` — Info expires after 3 s, Warn 5 s, Error never (until replaced).

## Request model (no tickets)
Actions never submit content jobs directly; they set `needs_requests = true`.
`after_batch` (called by the loop once per drained batch of events) then:
1. `request_missing` — for the current view, submits (High) whatever is not cached and not
   in `in_flight`:
   * Content/Blame: content of `blob`; highlight of it; Blame view: once content is `Text`
     and `blame_key` exists → `LoadBlame` (Index rows pass the staged content for
     `--contents -`).
   * Diff views: content of both `diff_sides`, highlights of both, then `ComputeDiff`
     when both contents are `diffable`.
2. `resolve_anchor` (below).
3. `prefetch` — Low priority content of entries `idx±2` (and their `prev_blob` in DiffPrev).
4. `clamp_cursor`, `touch_displayed` (promote displayed keys + anchor key in the LRU).

Every `JobResult` is **always cached** and removed from `in_flight`; staleness needs no
bookkeeping because the next `after_batch` re-derives needs from the current selection.
Holding `j` therefore only requests what is selected at the end of each batch.

## Selection & logical line anchoring
* `select(idx)`: no-op if unchanged; captures an anchor, sets `selected_idx/selected`,
  `pending_anchor = anchor`, `needs_requests`, `ensure_list_visible`.
* `capture_anchor`: reuses an existing `pending_anchor` (fast travel keeps anchoring from
  the last version actually shown); otherwise `displayed().anchor_at(cursor)` →
  `LineAnchor{content, line, viewport_offset: cursor - top}`.
* `set_view` captures an anchor the same way (switching views keeps the line).
* `resolve_anchor`: if the new display is `Waiting` → wait. Not text (no anchor key) →
  cursor/top = 0, **keep** the anchor for the next text version. Same key → same line.
  Otherwise use cached `FileDiff(anchor.content → new key)` + `line_map::map_line`; if
  missing, request `ComputeDiff` and wait; if impossible (old content evicted / not text)
  → keep the raw line number. Then cursor = `row_of_line`, `top = cursor - offset`,
  clamped so the content fills the screen; in diff views, if the row above `top` is a
  hunk header it is included.
* Any manual cursor move clears `pending_anchor`; `clamp_cursor` is skipped while an
  anchor is pending.
* `reselect` after timeline changes: re-find `selected` by `RevisionId`; if gone, select
  the clamped old index.

## Other behaviors
* **Hunks** `n`/`N`: jump to next/previous `HunkHeader` row (top = row - 1); at the end →
  Info "last hunk"/"first hunk" (no wrap to another revision).
* **BlameJump** (Blame view only): null oid → "line not committed yet"; commit in timeline →
  select it + Info "jumped to abc1234"; else Warn "… is not in the current timeline".
* **ToggleMark**: toggles `mark = selected`; hint "marked X — press 3 to compare with it";
  `SetView(DiffMarked)` without mark → hint "press m on a revision…".
* **ShowDetails**: commit rows only → `Overlay::Details` + `LoadDetails` if not cached.
* **CopyHash**: full oid → `Effect::Copy`; pseudo-rows → Warn "no commit hash for this row".
* **Export**: `selected_bytes` (content loaded; refuses Absent/Submodule/TooLarge) →
  `ExportPrompt` prefilled with `ops::export::default_name`. Confirm: empty → inline error;
  `resolve_dest` (`~/` expanded, relative to `ctx.invocation_dir`) → `Effect::Export
  {overwrite:false}`. `OpDone(ExportExists)` → `ConfirmOverwrite`; `y` → overwrite.
* **Restore**: refused on the WorkingTree row and via `selected_bytes`; opens
  `ConfirmRestore{overwrites_local_changes: a WorkingTree row exists}`; Confirm →
  `Effect::Restore{target: ctx.target in toplevel, mode (Absent→Regular)}`. `Restored` →
  Info "restored X from Y (not staged)", `invalidate_local`, submit `LoadLocalState`.
* **Failed op** → `Overlay::Error`.
* **Reload** `r`: `timeline_gen += 1`, drop committed rows (keep local ones), loading,
  `invalidate_local`, drop in-flight WorkingTree content, Info "reloading…", submit
  `LoadLocalState` + `LoadTimeline{gen}`.

## Job results (`job_done`)
| Result | Handling |
|---|---|
| `TimelineChunk{gen}` | ignored unless `gen == timeline_gen`; append + `reselect` |
| `TimelineDone{gen}` | stale gen ignored; loading off, complete. Err with empty timeline → `fatal`; Err otherwise → Error "history incomplete"; Ok with empty timeline → `fatal` "no history for …"; clears "reloading…" |
| `LocalState(Ok)` | `timeline.set_local` + `reselect`; Err → Error status |
| `Content` | cache content (or `content_errors`) |
| `Highlight` / `Diff` / `Blame` / `Details` | cache; `Diff.timed_out` → Warn "diff simplified…" |
