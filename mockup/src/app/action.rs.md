# `src/app/action.rs`

## Purpose
Vocabulary between the input layer and the reducer (`Action`), and between the reducer
and the outside world (`Effect`, answered by `OpResult`). Keeps `update.rs` IO-free.

## Contents

```rust
pub enum Action {
    // timeline
    SelectNext, SelectPrev, SelectFirst, SelectLast, SelectPageDown, SelectPageUp,
    SelectIndex(usize),                 // mouse click (also focuses timeline)
    SelectRevision(RevisionId),

    // file pane
    CursorDown(usize), CursorUp(usize),
    HalfPageDown, HalfPageUp, PageDown, PageUp, Top, Bottom,
    ScrollLeft, ScrollRight,            // 8 columns per step
    NextHunk, PrevHunk,
    BlameJump,
    ClickFileRow(usize),                // row inside file body (also focuses file)
    WheelTimeline(isize), WheelFile(isize),   // wheel, no focus change; + = down

    // layout / view
    ToggleFocus, FocusFile, FocusTimeline, ToggleZoom,
    SetView(ViewMode), ToggleMark, ToggleLineNumbers,

    // search (app/search.rs), scope = focused pane
    StartSearch, SearchNext, SearchPrev,
    Escape,                             // file: clear search, else FocusTimeline; timeline: clear search

    // overlays
    ShowHelp, ShowDetails,
    InputChar(char), InputBackspace, InputDelete, InputLeft, InputRight,
    InputHome, InputEnd, InputPaste(String),   // export prompt and search prompt
    Confirm, Cancel,

    // operations
    CopyHash, StartExport, StartRestore, Reload, Quit,

    // internal
    JobDone(JobResult),
    OpDone(OpResult),
    Layout(ui::LayoutInfo),             // pane rects returned by ui::draw (also on resize)
    Tick,                               // produced by the loop on a 120 ms recv timeout
}

#[allow(clippy::large_enum_variant)]    // short-lived, one per action
pub enum Effect {
    Submit(Job, Priority),
    Copy(String),
    Export  { bytes: Arc<[u8]>, dest: PathBuf, overwrite: bool },
    Restore { bytes: Arc<[u8]>, mode: FileMode, target: PathBuf },
    Quit,
}

pub enum OpResult {
    Copied { text: String, method: CopyMethod },   // Clipboard | Osc52
    Exported { dest: PathBuf },
    ExportExists { dest: PathBuf },                // → ConfirmOverwrite overlay
    Restored { target: PathBuf },
    Failed { op: &'static str, error: String },    // → Error overlay
}
```
