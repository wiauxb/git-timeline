# `src/app/action.rs`

## Purpose
Vocabulary between the input layer and the reducer (`Action`), and between the reducer
and the outside world (`Effect`). Keeps `update.rs` pure and testable: it never performs
IO, it only returns effects.

## Contents

```rust
pub enum Action {
    // navigation – timeline
    SelectNext, SelectPrev, SelectFirst, SelectLast,
    SelectPageDown, SelectPageUp,
    SelectIndex(usize),                  // mouse click
    SelectRevision(RevisionId),          // blame "jump to commit"

    // navigation – file pane
    CursorDown(usize), CursorUp(usize),
    HalfPageDown, HalfPageUp, PageDown, PageUp, Top, Bottom,
    ScrollLeft, ScrollRight,
    NextHunk, PrevHunk,
    BlameJump,                           // Enter on a blame line

    // layout / view
    ToggleFocus, FocusFile, FocusTimeline,
    ToggleZoom, SetView(ViewMode),
    ToggleMark,
    ToggleLineNumbers, ToggleWrap,

    // overlays
    ShowHelp, ShowDetails, CloseOverlay,
    InputChar(char), InputBackspace, InputLeft, InputRight, InputPaste(String),
    Confirm, Cancel,

    // operations
    CopyHash, StartExport, StartRestore,
    Reload,
    Quit,

    // internal
    JobDone(JobResult),
    OpDone(OpResult),
    Layout(ui::LayoutInfo),             // pane rects returned by ui::draw (resize, zoom)
    Tick,
}

pub enum Effect {
    Submit(Job, Priority),
    CancelTimeline,
    Copy(String),
    Export  { bytes: Arc<[u8]>, dest: PathBuf, overwrite: bool },
    Restore { bytes: Arc<[u8]>, mode: FileMode, target: PathBuf },
    Quit,
}

pub enum OpResult {
    Copied { method: CopyMethod },          // Clipboard | Osc52
    Exported { dest: PathBuf },
    Restored { target: PathBuf },
    Failed { op: &'static str, error: String },
    ExportExists { dest: PathBuf },         // → ConfirmOverwrite overlay
}
```
