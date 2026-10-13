//! `AppState`: the single source of truth. `ui/` reads it, only `update.rs` mutates it.

use std::borrow::Cow;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Local};

use super::cache::{absent, Caches};
use super::search::{SearchPrompt, SearchState};
use crate::cli::{InitialView, UiOptions};
use crate::git::repo::RepoContext;
use crate::highlight::{HighlightOutcome, HighlightedText};
use crate::model::{
    diff, BlameData, BlameKey, BlobSource, ContentKey, DiffKey, DiffLine, DiffRow, FileContent, FileDiff,
    FileMode, LineAnchor, Oid, RepoPath, RevisionId, Timeline, TimelineEntry,
};
use crate::ui::LayoutInfo;
use crate::worker::HighlightKey;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Timeline,
    File,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Content,
    DiffPrev,
    DiffMarked,
    Blame,
}

impl ViewMode {
    pub fn is_diff(self) -> bool {
        matches!(self, ViewMode::DiffPrev | ViewMode::DiffMarked)
    }
}

#[derive(Default, Debug)]
pub struct FileViewState {
    /// Row index in the rendered rows of the current view.
    pub cursor: usize,
    pub top: usize,
    /// Wrap: screen rows of `top` hidden above the body (set by `viewport::reveal` only).
    pub top_skip: usize,
    pub hscroll: usize,
    pub show_line_numbers: bool,
    /// Soft wrap at word boundaries.
    pub wrap: bool,
    pub pending_anchor: Option<LineAnchor>,
}

#[derive(Debug, Default, Clone)]
pub struct TextInput {
    pub value: String,
    /// Char index.
    pub cursor: usize,
}

impl TextInput {
    pub fn new(s: String) -> Self {
        let cursor = s.chars().count();
        TextInput { value: s, cursor }
    }
    fn byte_at(&self, char_idx: usize) -> usize {
        self.value.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(self.value.len())
    }
    pub fn insert(&mut self, s: &str) {
        let at = self.byte_at(self.cursor);
        self.value.insert_str(at, s);
        self.cursor += s.chars().count();
    }
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let at = self.byte_at(self.cursor);
            self.value.remove(at);
        }
    }
    pub fn delete(&mut self) {
        if self.cursor < self.value.chars().count() {
            let at = self.byte_at(self.cursor);
            self.value.remove(at);
        }
    }
}

#[derive(Debug)]
pub enum Overlay {
    Help { scroll: usize },
    Details { oid: Oid, scroll: usize },
    ConfirmRestore { rev: RevisionId, overwrites_local_changes: bool },
    ExportPrompt { rev: RevisionId, input: TextInput, error: Option<String> },
    ConfirmOverwrite { rev: RevisionId, dest: PathBuf },
    Error { message: String },
    /// `/` prompt; drawn in the status bar, not as a popup.
    Search(SearchPrompt),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug)]
pub struct StatusMessage {
    pub text: String,
    pub level: Level,
    pub expires: Option<Instant>,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub enum JobKey {
    Content(BlobSource),
    Highlight(HighlightKey),
    Diff(DiffKey),
    Blame(BlameKey),
    Details(Oid),
}

/// What the file pane shows right now, resolved from the caches.
pub enum Displayed {
    NoSelection,
    Waiting,
    Failed(String),
    NoMark,
    SameRevision,
    /// Content / blame view of a version that is not text (absent, binary, …).
    NotText { content: Arc<FileContent>, path: RepoPath },
    Text {
        key: ContentKey,
        content: Arc<FileContent>,
        hl: Option<Arc<HighlightedText>>,
        hl_note: Option<&'static str>,
        path: RepoPath,
    },
    Diff {
        diff: Arc<FileDiff>,
        rows: Vec<DiffRow>,
        old: Arc<FileContent>,
        new: Arc<FileContent>,
        old_hl: Option<Arc<HighlightedText>>,
        new_hl: Option<Arc<HighlightedText>>,
        old_key: ContentKey,
        new_key: ContentKey,
        old_label: String,
        new_label: String,
        old_mode: FileMode,
        new_mode: FileMode,
    },
    /// Diff of versions that are not both text (binary, submodule, symlink…).
    DiffSummary { old: Arc<FileContent>, new: Arc<FileContent>, old_label: String, new_label: String },
}

impl Displayed {
    pub fn row_count(&self) -> Option<usize> {
        match self {
            Displayed::Text { content, .. } => content.as_text().map(|t| t.line_count()),
            Displayed::Diff { rows, .. } => Some(rows.len()),
            _ => None,
        }
    }

    /// The key + line the cursor row stands for (anchoring across revisions).
    pub fn anchor_at(&self, row: usize) -> Option<(ContentKey, usize)> {
        match self {
            Displayed::Text { key, .. } => Some((key.clone(), row)),
            Displayed::Diff { rows, new_key, .. } => {
                Some((new_key.clone(), diff::source_line_of_row(rows, row).unwrap_or(0)))
            }
            _ => None,
        }
    }

    pub fn row_of_line(&self, line: usize) -> usize {
        match self {
            Displayed::Diff { rows, .. } => diff::row_of_source_line(rows, line),
            _ => line,
        }
    }

    /// Text of a row: the line (Text) or the line of the row's side (Diff); hunk headers
    /// and non-text displays have none. Shared by the search and the wrap heights.
    pub fn row_text(&self, row: usize) -> Option<Cow<'_, str>> {
        match self {
            Displayed::Text { content, .. } => {
                let t = content.as_text()?;
                (row < t.line_count()).then(|| t.line_str(row))
            }
            Displayed::Diff { rows, old, new, .. } => match rows.get(row)? {
                DiffRow::HunkHeader(_) => None,
                DiffRow::Line(DiffLine::Removed { old: l }) => Some(old.as_text()?.line_str(*l)),
                DiffRow::Line(DiffLine::Context { new: l, .. } | DiffLine::Added { new: l }) => {
                    Some(new.as_text()?.line_str(*l))
                }
            },
            _ => None,
        }
    }

    /// Key of the version the anchor is expressed in.
    pub fn anchor_key(&self) -> Option<&ContentKey> {
        match self {
            Displayed::Text { key, .. } => Some(key),
            Displayed::Diff { new_key, .. } => Some(new_key),
            _ => None,
        }
    }
}

pub struct AppState {
    pub ctx: Arc<RepoContext>,
    pub opts: UiOptions,
    pub highlight_enabled: bool,

    pub timeline: Timeline,
    pub timeline_loading: bool,
    pub timeline_gen: u64,

    pub selected: Option<RevisionId>,
    pub selected_idx: usize,
    pub list_offset: usize,
    pub mark: Option<RevisionId>,
    pub focus: Focus,
    pub zoomed: bool,

    pub view: ViewMode,
    pub file: FileViewState,
    pub search: SearchState,

    pub in_flight: HashSet<JobKey>,
    pub cache: Caches,
    /// Set by actions; the app turns it into job submissions once per event batch.
    pub needs_requests: bool,

    pub overlay: Option<Overlay>,
    pub status: Option<StatusMessage>,
    pub dirty: bool,
    pub spinner_frame: u8,
    pub clock: fn() -> DateTime<Local>,
    pub layout: LayoutInfo,
    /// Fatal error: the app quits and `main` prints it.
    pub fatal: Option<String>,
}

impl AppState {
    pub fn new(ctx: Arc<RepoContext>, opts: UiOptions, highlight_enabled: bool) -> Self {
        let view = match opts.initial_view {
            InitialView::Content => ViewMode::Content,
            InitialView::Diff => ViewMode::DiffPrev,
            InitialView::Blame => ViewMode::Blame,
        };
        AppState {
            ctx,
            opts,
            highlight_enabled,
            timeline: Timeline::default(),
            timeline_loading: true,
            timeline_gen: 0,
            selected: None,
            selected_idx: 0,
            list_offset: 0,
            mark: None,
            focus: Focus::Timeline,
            zoomed: false,
            view,
            file: FileViewState { show_line_numbers: true, wrap: true, ..Default::default() },
            search: SearchState::default(),
            in_flight: HashSet::new(),
            cache: Caches::default(),
            needs_requests: true,
            overlay: None,
            status: None,
            dirty: true,
            spinner_frame: 0,
            clock: Local::now,
            layout: LayoutInfo::default(),
            fatal: None,
        }
    }

    pub fn selected_entry(&self) -> Option<&TimelineEntry> {
        self.selected.as_ref().and_then(|_| self.timeline.entries.get(self.selected_idx))
    }

    pub fn marked_entry(&self) -> Option<(usize, &TimelineEntry)> {
        let m = self.mark.as_ref()?;
        let i = self.timeline.position_of(m)?;
        Some((i, &self.timeline.entries[i]))
    }

    pub fn spinner(&self) -> char {
        const FRAMES: [char; 4] = ['⠋', '⠙', '⠸', '⠴'];
        FRAMES[self.spinner_frame as usize % FRAMES.len()]
    }

    pub fn highlight_key(&self, key: &ContentKey, path: &RepoPath) -> HighlightKey {
        HighlightKey { content: key.clone(), name: path.file_name().to_vec() }
    }

    fn highlight_of(&self, key: &ContentKey, path: &RepoPath) -> (Option<Arc<HighlightedText>>, Option<&'static str>) {
        if !self.highlight_enabled {
            return (None, None);
        }
        match self.cache.highlight.peek(&self.highlight_key(key, path)) {
            Some(HighlightOutcome::Highlighted(h)) => (Some(h.clone()), None),
            Some(HighlightOutcome::Plain { reason }) => (None, Some(reason)),
            None => (None, None),
        }
    }

    /// (old, new) entries for the diff views; old = older in history.
    pub fn diff_sides(&self) -> Option<Side2<'_>> {
        let e = self.selected_entry()?;
        match self.view {
            ViewMode::DiffPrev => Some(Side2 {
                old: (&e.prev_blob, e.prev_mode, &e.prev_path, prev_label(e)),
                new: (&e.blob, e.mode, &e.path, e.label()),
            }),
            ViewMode::DiffMarked => {
                let (mi, m) = self.marked_entry()?;
                if m.rev == e.rev {
                    return None;
                }
                let (old, new) = if mi > self.selected_idx { (m, e) } else { (e, m) };
                Some(Side2 {
                    old: (&old.blob, old.mode, &old.path, old.label()),
                    new: (&new.blob, new.mode, &new.path, new.label()),
                })
            }
            _ => None,
        }
    }

    pub fn displayed(&self) -> Displayed {
        let Some(e) = self.selected_entry() else { return Displayed::NoSelection };
        match self.view {
            ViewMode::Content | ViewMode::Blame => match self.cache.content_for(&e.blob) {
                Err(msg) => Displayed::Failed(msg),
                Ok(None) => Displayed::Waiting,
                Ok(Some((key, content))) => {
                    if content.as_text().is_some() {
                        let (hl, hl_note) = self.highlight_of(&key, &e.path);
                        Displayed::Text { key, content, hl, hl_note, path: e.path.clone() }
                    } else {
                        Displayed::NotText { content, path: e.path.clone() }
                    }
                }
            },
            ViewMode::DiffPrev | ViewMode::DiffMarked => {
                if self.view == ViewMode::DiffMarked {
                    match self.marked_entry() {
                        None => return Displayed::NoMark,
                        Some((_, m)) if m.rev == e.rev => return Displayed::SameRevision,
                        _ => {}
                    }
                }
                let Some(s) = self.diff_sides() else { return Displayed::NoMark };
                let (old, new) = match (self.cache.content_for(s.old.0), self.cache.content_for(s.new.0)) {
                    (Err(m), _) | (_, Err(m)) => return Displayed::Failed(m),
                    (Ok(Some(o)), Ok(Some(n))) => (o, n),
                    _ => return Displayed::Waiting,
                };
                let old_label = format!("a/{} @ {}", s.old.2.display(), s.old.3);
                let new_label = format!("b/{} @ {}", s.new.2.display(), s.new.3);
                if !diffable(&old.1) || !diffable(&new.1) {
                    return Displayed::DiffSummary { old: old.1, new: new.1, old_label, new_label };
                }
                let key = DiffKey { old: old.0.clone(), new: new.0.clone() };
                let Some(d) = self.cache.diff.peek(&key).cloned() else { return Displayed::Waiting };
                let (old_hl, _) = self.highlight_of(&old.0, s.old.2);
                let (new_hl, _) = self.highlight_of(&new.0, s.new.2);
                Displayed::Diff {
                    rows: d.rows(),
                    diff: d,
                    old: old.1,
                    new: new.1,
                    old_hl,
                    new_hl,
                    old_key: old.0,
                    new_key: new.0,
                    old_label,
                    new_label,
                    old_mode: s.old.1,
                    new_mode: s.new.1,
                }
            }
        }
    }

    pub fn blame_key(&self, e: &TimelineEntry) -> Option<BlameKey> {
        match &e.rev {
            RevisionId::Commit(oid) => {
                (e.blob != BlobSource::Absent).then(|| BlameKey::Commit { oid: oid.clone(), path: e.path.clone() })
            }
            rev => {
                let key = self.cache.key_of(&e.blob)?;
                (key != ContentKey::Absent).then(|| BlameKey::Local { rev: rev.clone(), content: key })
            }
        }
    }

    pub fn blame(&self) -> Option<Result<Arc<BlameData>, String>> {
        let e = self.selected_entry()?;
        let key = self.blame_key(e)?;
        self.cache.blame.peek(&key).cloned()
    }

    pub fn timeline_rows(&self) -> usize {
        (self.layout.timeline.height as usize).saturating_sub(2).max(1)
    }

    pub fn file_rows(&self) -> usize {
        (self.layout.file_body_rows as usize).max(1)
    }

    pub fn file_cols(&self) -> usize {
        (self.layout.file_body_cols as usize).max(1)
    }
}

pub struct Side2<'a> {
    pub old: (&'a BlobSource, FileMode, &'a RepoPath, String),
    pub new: (&'a BlobSource, FileMode, &'a RepoPath, String),
}

fn prev_label(e: &TimelineEntry) -> String {
    match &e.rev {
        RevisionId::WorkingTree => "index".into(),
        RevisionId::Index => "HEAD".into(),
        RevisionId::Commit(_) => match e.meta.as_ref().and_then(|m| m.parents.first()) {
            Some(p) => p.short().to_string(),
            None => "∅".into(),
        },
    }
}

pub fn diffable(c: &FileContent) -> bool {
    matches!(c, FileContent::Text(_) | FileContent::Absent)
}

pub fn absent_content() -> Arc<FileContent> {
    absent()
}
