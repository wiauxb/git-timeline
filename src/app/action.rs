use std::path::PathBuf;
use std::sync::Arc;

use crate::model::{FileMode, RevisionId};
use crate::ops::clipboard::CopyMethod;
use crate::ui::LayoutInfo;
use crate::worker::{Job, JobResult, Priority};

use super::state::ViewMode;

#[derive(Debug)]
pub enum Action {
    // timeline
    SelectNext,
    SelectPrev,
    SelectFirst,
    SelectLast,
    SelectPageDown,
    SelectPageUp,
    SelectIndex(usize),
    SelectRevision(RevisionId),

    // file pane
    CursorDown(usize),
    CursorUp(usize),
    HalfPageDown,
    HalfPageUp,
    PageDown,
    PageUp,
    Top,
    Bottom,
    ScrollLeft,
    ScrollRight,
    NextHunk,
    PrevHunk,
    BlameJump,
    ClickFileRow(usize),
    /// Mouse wheel over a pane, without changing focus. Positive = down.
    WheelTimeline(isize),
    WheelFile(isize),

    // layout / view
    ToggleFocus,
    FocusFile,
    FocusTimeline,
    ToggleZoom,
    SetView(ViewMode),
    ToggleMark,
    ToggleLineNumbers,
    ToggleWrap,

    // search (app/search.rs), scope = focused pane
    StartSearch,
    SearchNext,
    SearchPrev,
    /// File pane: clear the search, else focus the timeline. Timeline: clear the search.
    Escape,

    // overlays
    ShowHelp,
    ShowDetails,
    InputChar(char),
    InputBackspace,
    InputDelete,
    InputLeft,
    InputRight,
    InputHome,
    InputEnd,
    InputPaste(String),
    Confirm,
    Cancel,

    // operations
    CopyHash,
    StartExport,
    StartRestore,
    Reload,
    Quit,

    // internal
    JobDone(JobResult),
    OpDone(OpResult),
    Layout(LayoutInfo),
    Tick,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // short-lived, one per action
pub enum Effect {
    Submit(Job, Priority),
    Copy(String),
    Export { bytes: Arc<[u8]>, dest: PathBuf, overwrite: bool },
    Restore { bytes: Arc<[u8]>, mode: FileMode, target: PathBuf },
    Quit,
}

#[derive(Debug)]
pub enum OpResult {
    Copied { text: String, method: CopyMethod },
    Exported { dest: PathBuf },
    ExportExists { dest: PathBuf },
    Restored { target: PathBuf },
    Failed { op: &'static str, error: String },
}
