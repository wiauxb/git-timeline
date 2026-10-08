# `src/app/state.rs`

## Purpose
`AppState` — the **single source of truth** for everything displayed. `ui/` reads it,
only `update.rs` mutates it. Also hosts the derived "what is displayed right now"
resolution (`Displayed`) shared by the reducer and the renderer.

## Contents

```rust
pub enum Focus { Timeline, File }
pub enum ViewMode { Content, DiffPrev, DiffMarked, Blame }
impl ViewMode { pub fn is_diff(self) -> bool; }

#[derive(Default)]
pub struct FileViewState {
    pub cursor: usize,             // row index in the rendered rows of the current view
    pub top: usize,                // first visible row
    pub top_skip: usize,           // wrap: screen rows of `top` hidden above the body
                                   // (only set by viewport::reveal; 0 otherwise)
    pub hscroll: usize,            // columns
    pub show_line_numbers: bool,   // initial: true
    pub wrap: bool,                // soft wrap at word boundaries; initial: false
    pub pending_anchor: Option<LineAnchor>,
}

pub struct TextInput { pub value: String, pub cursor: usize /* char index */ }
impl TextInput { new(String) /* cursor at end */, insert(&str), backspace(), delete() }

pub enum Overlay {
    Help { scroll: usize },
    Details { oid: Oid, scroll: usize },
    ConfirmRestore { rev: RevisionId, overwrites_local_changes: bool },
    ExportPrompt { rev: RevisionId, input: TextInput, error: Option<String> },
    ConfirmOverwrite { rev: RevisionId, dest: PathBuf },
    Error { message: String },
    Search(SearchPrompt),            // app/search.rs; drawn in the status bar, not a popup
}

pub enum Level { Info, Warn, Error }
pub struct StatusMessage { pub text: String, pub level: Level, pub expires: Option<Instant> }

#[derive(Hash, Eq, Clone)]
pub enum JobKey { Content(BlobSource), Highlight(HighlightKey), Diff(DiffKey),
                  Blame(BlameKey), Details(Oid) }

pub struct AppState {
    pub ctx: Arc<RepoContext>,
    pub opts: UiOptions,
    pub highlight_enabled: bool,

    pub timeline: Timeline,
    pub timeline_loading: bool,          // initial true
    pub timeline_gen: u64,               // bumped by reload; stale chunks ignored

    pub selected: Option<RevisionId>,    // stable identity
    pub selected_idx: usize,             // derived position of `selected`
    pub list_offset: usize,
    pub mark: Option<RevisionId>,        // compare base for DiffMarked
    pub focus: Focus,                    // initial Timeline
    pub zoomed: bool,

    pub view: ViewMode,                  // from --view: content / diff (DiffPrev) / blame
    pub file: FileViewState,
    pub search: SearchState,             // app/search.rs: one query per pane + hit lists

    pub in_flight: HashSet<JobKey>,      // dedupe submitted jobs
    pub cache: Caches,
    pub needs_requests: bool,            // set by actions; consumed by update::after_batch

    pub overlay: Option<Overlay>,
    pub status: Option<StatusMessage>,
    pub dirty: bool,
    pub spinner_frame: u8,
    pub clock: fn() -> DateTime<Local>,  // Local::now; replaceable in tests
    pub layout: LayoutInfo,              // last pane rects, fed back via Action::Layout
    pub fatal: Option<String>,           // app quits, main prints it
}
```

There is **no job ticket / generation for content**: every job result is cached and the
current needs are re-derived from state (see `update.rs`). Only the timeline stream has a
generation (`timeline_gen`).

## `Displayed` — what the file pane shows, resolved from caches

```rust
pub enum Displayed {
    NoSelection,
    Waiting,                               // some needed content/diff not loaded yet
    Failed(String),                        // content load error (cached)
    NoMark,                                // DiffMarked without mark
    SameRevision,                          // DiffMarked with mark == selected
    NotText { content: Arc<FileContent>, path: RepoPath },   // Content/Blame of absent/binary/…
    Text { key: ContentKey, content: Arc<FileContent>, hl: Option<Arc<HighlightedText>>,
           hl_note: Option<&'static str>, path: RepoPath },
    Diff { diff: Arc<FileDiff>, rows: Vec<DiffRow>, old: Arc<FileContent>, new: Arc<FileContent>,
           old_hl, new_hl: Option<Arc<HighlightedText>>, old_key, new_key: ContentKey,
           old_label: String, new_label: String, old_mode: FileMode, new_mode: FileMode },
    DiffSummary { old: Arc<FileContent>, new: Arc<FileContent>, old_label: String, new_label: String },
}
impl Displayed {
    pub fn row_count(&self) -> Option<usize>;                     // Text: lines, Diff: rows
    pub fn anchor_at(&self, row) -> Option<(ContentKey, usize)>;  // Diff: new-side line
    pub fn row_of_line(&self, line) -> usize;                     // Diff: nearest new-side row
    pub fn anchor_key(&self) -> Option<&ContentKey>;              // Text key / Diff new_key
    /// Text of a row: Text ⇒ the line; Diff ⇒ the line of the row's side (old for
    /// removed rows, new otherwise); hunk headers / non-text ⇒ None. Shared by the
    /// search (hits, reveal) and the viewport (wrap heights). Moved from `search.rs`.
    pub fn row_text(&self, row: usize) -> Option<Cow<'_, str>>;
}
```

Resolution (`AppState::displayed()`):
* **Content / Blame**: content of `entry.blob` → `Failed` / `Waiting` / `Text` (with
  highlight + note when cached) / `NotText`.
* **DiffPrev / DiffMarked**: `NoMark` / `SameRevision` checks, then both sides from
  `diff_sides()`; any error → `Failed`, any missing → `Waiting`; a side that is not
  `Text`/`Absent` (`diffable()` false) → `DiffSummary`; else the cached diff for
  `DiffKey{old_key, new_key}` → `Diff`, missing → `Waiting`.
* Labels: `a/<path> @ <label>` / `b/<path> @ <label>`.

## Other methods

```rust
impl AppState {
    pub fn new(ctx, opts, highlight_enabled) -> Self;
    pub fn selected_entry(&self) -> Option<&TimelineEntry>;
    pub fn marked_entry(&self) -> Option<(usize, &TimelineEntry)>;
    pub fn spinner(&self) -> char;                       // ⠋ ⠙ ⠸ ⠴
    pub fn highlight_key(&self, key, path) -> HighlightKey;   // (content, file name)
    /// DiffPrev: (prev_blob, prev_mode, prev_path, parent label) → (blob, mode, path, label).
    /// DiffMarked: ordered by timeline index; older (larger index) = old side.
    pub fn diff_sides(&self) -> Option<Side2<'_>>;
    pub fn displayed(&self) -> Displayed;
    /// Commit → Commit{oid, path} (None if blob Absent);
    /// local rows → Local{rev, content key} once the key is known (None if Absent).
    pub fn blame_key(&self, e: &TimelineEntry) -> Option<BlameKey>;
    pub fn blame(&self) -> Option<Result<Arc<BlameData>, String>>;
    pub fn timeline_rows(&self) -> usize;   // layout.timeline.height - 2, min 1
    pub fn file_rows(&self) -> usize;       // layout.file_body_rows, min 1
    pub fn file_cols(&self) -> usize;
}
pub struct Side2<'a> { pub old, pub new: (&'a BlobSource, FileMode, &'a RepoPath, String) }
pub fn diffable(c: &FileContent) -> bool;   // Text | Absent
pub fn absent_content() -> Arc<FileContent>;
```

Parent label for `DiffPrev`: WorkingTree → `index`, Index → `HEAD`, commit → first
parent's short oid, root commit → `∅`.

## Invariants
* `selected_idx` points at `selected` whenever the timeline is non-empty (`reselect`).
* A `mark` whose revision vanished after a *complete* reload is cleared with a warning.
* Not implemented: vim count prefix; moving by screen row inside a wrapped line (`gj`).
