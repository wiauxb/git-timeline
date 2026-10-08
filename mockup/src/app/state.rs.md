# `src/app/state.rs`

## Purpose
`AppState` — the **single source of truth** for everything displayed. `ui/` reads it,
only `update.rs` mutates it.

## Contents

```rust
pub struct AppState {
    pub ctx: Arc<RepoContext>,
    pub opts: UiOptions,

    // ── timeline ─────────────────────────────────────────────
    pub timeline: Timeline,
    pub timeline_loading: bool,
    pub local_entries_loaded: bool,

    // ── selection & navigation ───────────────────────────────
    pub selected: Option<RevisionId>,      // stable identity
    pub selected_idx: usize,               // derived, recomputed when timeline changes
    pub list_offset: usize,                // first visible row of the timeline pane
    pub mark: Option<RevisionId>,          // compare base for ViewMode::DiffMarked
    pub focus: Focus,
    pub zoomed: bool,                      // file pane full width

    // ── file pane ────────────────────────────────────────────
    pub view: ViewMode,
    pub file: FileViewState,

    // ── async bookkeeping ───────────────────────────────────
    pub ticket: Ticket,                    // bumped on selection/view change
    pub in_flight: HashSet<JobKey>,        // dedupe submitted jobs
    pub cache: cache::Caches,

    // ── overlays & messages ─────────────────────────────────
    pub overlay: Option<Overlay>,
    pub status: Option<StatusMessage>,
    pub dirty: bool,                       // needs redraw
    pub spinner_frame: u8,
    pub clock: fn() -> chrono::DateTime<chrono::Local>, // "now" for relative dates; fixed in tests
    pub layout: ui::LayoutInfo,            // last pane rects; fed back via Action::Layout
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus { Timeline, File }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode { Content, DiffPrev, DiffMarked, Blame }

pub struct FileViewState {
    pub cursor: usize,          // row index in the *rendered rows* of the current view
    pub top: usize,             // first visible row
    pub hscroll: usize,         // columns scrolled horizontally
    pub show_line_numbers: bool,
    pub wrap: bool,             // soft wrap (default false)
    pub pending_anchor: Option<LineAnchor>,  // waiting for content to re-anchor
}

pub enum Overlay {
    Help { scroll: usize },
    Details { oid: Oid },                              // data pulled from cache
    ConfirmRestore { rev: RevisionId, overwrites_local_changes: bool },
    ExportPrompt { rev: RevisionId, input: TextInput, error: Option<String> },
    ConfirmOverwrite { rev: RevisionId, dest: PathBuf },
    Error { message: String },
}

pub struct TextInput { pub value: String, pub cursor: usize }   // char index

pub struct StatusMessage { pub text: String, pub level: Level, pub expires: Instant }
pub enum Level { Info, Warn, Error }

#[derive(Hash, PartialEq, Eq)]
pub enum JobKey { Content(BlobSource), Highlight(ContentKey), Diff(DiffKey),
                  Blame(BlameKey), Details(Oid) }
```

## Derived queries (methods, no mutation)

```rust
impl AppState {
    pub fn selected_entry(&self) -> Option<&TimelineEntry>;
    pub fn marked_entry(&self) -> Option<&TimelineEntry>;
    /// The pair (old, new) to diff for the current view, ordered by timeline position
    /// (older = larger index). None if the view isn't a diff view or mark missing.
    pub fn diff_pair(&self) -> Option<(&TimelineEntry /*old side source*/, &TimelineEntry)>;
    /// What the file pane needs right now and whether each piece is ready.
    pub fn view_requirements(&self) -> ViewRequirements;
}

pub struct ViewRequirements {
    pub contents: Vec<(BlobSource, FileMode)>,
    pub highlights: Vec<ContentKey>,
    pub diff: Option<DiffKey>,
    pub blame: Option<BlameKey>,
}
```

## Invariants
* `selected_idx` always points at `selected` when the timeline is non-empty.
* `mark` may refer to an entry that disappeared after reload ⇒ cleared, status message.
* `DiffMarked` with `mark == selected` shows "same revision" instead of an empty diff.
