# `src/app/update.rs`

## Purpose
The reducer: `fn update(state: &mut AppState, action: Action) -> Vec<Effect>`.
All behavior of the app lives here; it is synchronous, deterministic and IO-free, so it is
unit-tested by feeding actions and asserting on state + effects.

## Contents

```rust
pub fn update(s: &mut AppState, a: Action) -> Vec<Effect>;

fn on_selection_changed(s: &mut AppState, old: Option<RevisionId>) -> Vec<Effect>;
fn on_view_changed(s: &mut AppState) -> Vec<Effect>;
fn request_missing(s: &mut AppState, prio: Priority) -> Vec<Effect>;   // uses view_requirements()
fn prefetch_neighbours(s: &mut AppState) -> Vec<Effect>;
fn on_job_done(s: &mut AppState, r: JobResult) -> Vec<Effect>;
fn resolve_anchor(s: &mut AppState) -> Vec<Effect>;
fn merge_timeline_chunk(s: &mut AppState, chunk: Vec<TimelineEntry>);
fn rows_of_current_view(s: &AppState) -> usize;                 // for cursor clamping
```

## Key behaviors

### Selection change (timeline move, `[`/`]`, click, blame jump)
1. Before changing: capture `LineAnchor { content: key of current displayed version,
   line: cursor's source line, viewport_offset: cursor - top }`
   (only if the current content is loaded).
2. Update `selected`, `selected_idx`, scroll `list_offset` to keep it visible.
3. `ticket += 1`; `file.pending_anchor = Some(anchor)`.
4. `request_missing(High)` then `prefetch_neighbours()` (Low): contents of
   `selected_idx ± 1..=2`, plus their `prev_blob` when view is `DiffPrev`.
5. **Debounce:** holding `j` generates many actions; requests are only emitted for the
   last selection of a coalesced event batch (the loop drains events before calling
   `request_missing` via a `needs_requests` flag processed at the end of the batch).

### Anchor resolution (`resolve_anchor`)
Called whenever a `Content`/`Diff` result arrives. If `pending_anchor` is set and the new
displayed content is loaded:
* same `ContentKey` ⇒ keep line as-is (file unchanged in this commit; very common).
* else need `FileDiff(anchor.content → new key)`: if cached ⇒ `line_map::map_line`,
  set cursor/top, clear anchor; if not ⇒ submit `ComputeDiff` (High) and wait.
* new version `Absent`/binary ⇒ keep anchor pending (re-applied when moving to a text
  version again), cursor = 0.

### View requirements per mode
| View | Contents | Diff | Blame |
|---|---|---|---|
| Content | `blob` | – | – |
| DiffPrev | `prev_blob`, `blob` | (prev_blob, blob) | – |
| DiffMarked | `older.blob`, `newer.blob` | (older, newer) | – |
| Blame | `blob` | – | `BlameKey` of entry |
Highlights are requested for each loaded text content.

### Job results
* Always insert into cache (even stale tickets), remove from `in_flight`.
* `ticket == s.ticket` ⇒ `resolve_anchor`, `dirty = true`.
* `TimelineChunk` ⇒ append; if nothing selected yet, select index 0 (the
  working-tree row if present, else HEAD's commit) and request content.
* `LocalState` ⇒ replace the pseudo-entries at the top, recompute `selected_idx`.

### Hunk navigation (`n`/`N`)
Only in diff views: move cursor to the first row of the next/previous hunk; at the end
of the file, wrap to the **next revision's** first hunk? → **No** (v1): stop and show
"last hunk".

### Mark (`m`)
Toggle `mark = selected`. If the view is not `DiffMarked` and a mark is set, status hint:
"marked abc1234 — press 3 to compare".

### Actions
* `CopyHash` ⇒ `Effect::Copy(full oid)`; pseudo-entries ⇒ status "no commit".
* `StartExport` ⇒ requires content loaded and `Text`/`Binary`/`Symlink` (bytes available);
  opens `ExportPrompt` prefilled with `ops::export::default_name(entry)`.
* `StartRestore` ⇒ refused for `WorkingTree` row, `Submodule`, and `Absent` (offer: "file
  doesn't exist at this revision"); else opens `ConfirmRestore` with
  `overwrites_local_changes = a WorkingTree pseudo-entry exists`.
* `Confirm` in `ConfirmRestore` ⇒ `Effect::Restore{ target = ctx.target.to_fs_path() }`.
  Restores always write to the **current** path (`ctx.target`), even if the version came
  from an older name.
* `OpDone(Restored)` ⇒ status message + `Submit(LoadLocalState)`.

### Reload (`r`)
`CancelTimeline`, clear timeline (keep caches: blobs are immutable by oid; working-tree
keys include mtime), resubmit `LoadLocalState` + `LoadTimeline`, keep `selected` / `mark`
by `RevisionId`.
