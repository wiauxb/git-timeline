//! The reducer. Synchronous and IO-free: it mutates state and returns effects.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::action::{Action, Effect, OpResult};
use super::search::{self, Scope};
use super::state::{diffable, AppState, Displayed, Focus, JobKey, Level, Overlay, StatusMessage, TextInput, ViewMode};
use crate::model::line_map::map_line;
use crate::model::{
    BlobSource, ContentKey, DiffKey, DiffRow, FileContent, FileMode, LineAnchor, RevisionId,
    TimelineEntry,
};
use crate::ops;
use crate::worker::{Job, JobResult, Priority};

const PREFETCH: usize = 2;
const LIST_MARGIN: usize = 2;

pub fn update(s: &mut AppState, a: Action) -> Vec<Effect> {
    s.dirty = true;
    if s.overlay.is_some() {
        if let Some(fx) = overlay_action(s, &a) {
            return fx;
        }
    }
    let mut fx = Vec::new();
    match a {
        Action::SelectNext => select_delta(s, 1),
        Action::SelectPrev => select_delta(s, -1),
        Action::SelectFirst => select(s, 0),
        Action::SelectLast => select(s, s.timeline.len().saturating_sub(1)),
        Action::SelectPageDown => select_delta(s, s.timeline_rows() as isize),
        Action::SelectPageUp => select_delta(s, -(s.timeline_rows() as isize)),
        Action::SelectIndex(i) => {
            s.focus = Focus::Timeline;
            if i < s.timeline.len() {
                select(s, i);
            }
        }
        Action::SelectRevision(rev) => {
            if let Some(i) = s.timeline.position_of(&rev) {
                select(s, i);
            }
        }
        Action::WheelTimeline(d) => select_delta(s, d),

        Action::CursorDown(n) => move_cursor(s, n as isize),
        Action::CursorUp(n) => move_cursor(s, -(n as isize)),
        Action::WheelFile(d) => move_cursor(s, d),
        Action::HalfPageDown => move_cursor(s, (s.file_rows() / 2) as isize),
        Action::HalfPageUp => move_cursor(s, -((s.file_rows() / 2) as isize)),
        Action::PageDown => move_cursor(s, s.file_rows() as isize),
        Action::PageUp => move_cursor(s, -(s.file_rows() as isize)),
        Action::Top => move_cursor(s, isize::MIN / 2),
        Action::Bottom => move_cursor(s, isize::MAX / 2),
        Action::ClickFileRow(row) => {
            s.focus = Focus::File;
            let target = s.file.top + row;
            move_cursor(s, target as isize - s.file.cursor as isize);
        }
        Action::ScrollLeft => s.file.hscroll = s.file.hscroll.saturating_sub(8),
        Action::ScrollRight => s.file.hscroll += 8,
        Action::NextHunk => jump_hunk(s, true),
        Action::PrevHunk => jump_hunk(s, false),
        Action::BlameJump => blame_jump(s),

        Action::ToggleFocus => {
            s.focus = match s.focus {
                Focus::Timeline => Focus::File,
                Focus::File => Focus::Timeline,
            }
        }
        Action::FocusFile => s.focus = Focus::File,
        Action::FocusTimeline => s.focus = Focus::Timeline,
        Action::ToggleZoom => s.zoomed = !s.zoomed,
        Action::SetView(v) => set_view(s, v),
        Action::ToggleMark => toggle_mark(s),
        Action::ToggleLineNumbers => s.file.show_line_numbers = !s.file.show_line_numbers,

        Action::StartSearch => search::start(s),
        Action::SearchNext => search::next(s, true),
        Action::SearchPrev => search::next(s, false),
        Action::Escape => match s.focus {
            Focus::Timeline => {
                search::clear(s, Scope::Timeline);
            }
            Focus::File => {
                if !search::clear(s, Scope::File) {
                    s.focus = Focus::Timeline;
                }
            }
        },

        Action::ShowHelp => s.overlay = Some(Overlay::Help { scroll: 0 }),
        Action::ShowDetails => fx.extend(show_details(s)),

        Action::CopyHash => match s.selected_entry().and_then(|e| e.oid()) {
            Some(oid) => fx.push(Effect::Copy(oid.to_string())),
            None => info(s, Level::Warn, "no commit hash for this row"),
        },
        Action::StartExport => start_export(s),
        Action::StartRestore => start_restore(s),
        Action::Reload => fx.extend(reload(s)),
        Action::Quit => fx.push(Effect::Quit),

        Action::JobDone(r) => fx.extend(job_done(s, r)),
        Action::OpDone(r) => fx.extend(op_done(s, r)),
        Action::Layout(l) => {
            s.layout = l;
            clamp_cursor(s);
            ensure_list_visible(s);
        }
        Action::Tick => {
            s.dirty = false;
            if s.timeline_loading || !s.in_flight.is_empty() {
                s.spinner_frame = s.spinner_frame.wrapping_add(1);
                s.dirty = true;
            }
            if let Some(m) = &s.status {
                if m.expires.is_some_and(|t| t <= Instant::now()) {
                    s.status = None;
                    s.dirty = true;
                }
            }
        }

        // overlay-only actions arriving without overlay
        Action::InputChar(_)
        | Action::InputBackspace
        | Action::InputDelete
        | Action::InputLeft
        | Action::InputRight
        | Action::InputHome
        | Action::InputEnd
        | Action::InputPaste(_)
        | Action::Confirm
        | Action::Cancel => {}
    }
    fx
}

/// Called once per batch of events: turns needs into jobs, re-anchors the view.
pub fn after_batch(s: &mut AppState) -> Vec<Effect> {
    if !s.needs_requests {
        return Vec::new();
    }
    s.needs_requests = false;
    let mut fx = request_missing(s);
    fx.extend(resolve_anchor(s));
    fx.extend(prefetch(s));
    search::refresh(s);
    clamp_cursor(s);
    touch_displayed(s);
    fx
}

// ───────────────────────────── info ─────────────────────────────

pub fn info(s: &mut AppState, level: Level, text: impl Into<String>) {
    let ttl = match level {
        Level::Info => Some(Duration::from_secs(3)),
        Level::Warn => Some(Duration::from_secs(5)),
        Level::Error => None,
    };
    s.status = Some(StatusMessage { text: text.into(), level, expires: ttl.map(|t| Instant::now() + t) });
}

// ─────────────────────────── selection ───────────────────────────

fn select_delta(s: &mut AppState, d: isize) {
    if s.timeline.is_empty() {
        return;
    }
    let max = s.timeline.len() as isize - 1;
    let i = (s.selected_idx as isize + d).clamp(0, max) as usize;
    select(s, i);
}

pub fn select(s: &mut AppState, idx: usize) {
    if s.selected.is_some() && idx == s.selected_idx {
        return;
    }
    let anchor = capture_anchor(s);
    s.selected_idx = idx;
    s.selected = s.timeline.entries.get(idx).map(|e| e.rev.clone());
    s.file.pending_anchor = anchor;
    s.needs_requests = true;
    ensure_list_visible(s);
}

fn capture_anchor(s: &AppState) -> Option<LineAnchor> {
    if let Some(a) = &s.file.pending_anchor {
        // still travelling: keep anchoring from the last version actually shown
        return Some(a.clone());
    }
    let (content, line) = s.displayed().anchor_at(s.file.cursor)?;
    Some(LineAnchor { content, line, viewport_offset: s.file.cursor.saturating_sub(s.file.top) })
}

fn ensure_list_visible(s: &mut AppState) {
    let rows = s.timeline_rows();
    let margin = LIST_MARGIN.min(rows.saturating_sub(1) / 2);
    let i = s.selected_idx;
    if i < s.list_offset + margin {
        s.list_offset = i.saturating_sub(margin);
    } else if i + margin >= s.list_offset + rows {
        s.list_offset = (i + margin + 1).saturating_sub(rows);
    }
    let max_off = s.timeline.len().saturating_sub(rows);
    s.list_offset = s.list_offset.min(max_off);
}

/// Re-point `selected_idx` after the timeline changed.
fn reselect(s: &mut AppState) {
    match s.selected.clone().and_then(|r| s.timeline.position_of(&r)) {
        Some(i) => s.selected_idx = i,
        None => {
            s.selected = None;
            if !s.timeline.is_empty() {
                select(s, s.selected_idx.min(s.timeline.len() - 1));
            }
        }
    }
    if s.mark.as_ref().is_some_and(|m| s.timeline.position_of(m).is_none()) && s.timeline.complete {
        s.mark = None;
        info(s, Level::Warn, "marked revision disappeared after reload; mark cleared");
    }
    ensure_list_visible(s);
}

// ─────────────────────────── file cursor ───────────────────────────

fn move_cursor(s: &mut AppState, d: isize) {
    s.file.pending_anchor = None;
    let Some(n) = s.displayed().row_count() else { return };
    let max = n.saturating_sub(1) as isize;
    s.file.cursor = (s.file.cursor as isize).saturating_add(d).clamp(0, max.max(0)) as usize;
    scroll_to_cursor(s);
}

pub(super) fn scroll_to_cursor(s: &mut AppState) {
    let rows = s.file_rows();
    if s.file.cursor < s.file.top {
        s.file.top = s.file.cursor;
    } else if s.file.cursor >= s.file.top + rows {
        s.file.top = s.file.cursor + 1 - rows;
    }
}

fn clamp_cursor(s: &mut AppState) {
    if s.file.pending_anchor.is_some() {
        return;
    }
    if let Some(n) = s.displayed().row_count() {
        s.file.cursor = s.file.cursor.min(n.saturating_sub(1));
        s.file.top = s.file.top.min(n.saturating_sub(1));
        scroll_to_cursor(s);
    }
}

fn jump_hunk(s: &mut AppState, forward: bool) {
    let Displayed::Diff { rows, .. } = s.displayed() else { return };
    let headers: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r, DiffRow::HunkHeader(_)))
        .map(|(i, _)| i)
        .collect();
    let cur = s.file.cursor;
    let target = if forward {
        headers.iter().copied().find(|&h| h > cur)
    } else {
        headers.iter().rev().copied().find(|&h| h < cur)
    };
    match target {
        Some(t) => {
            s.file.pending_anchor = None;
            s.file.cursor = t;
            // show the hunk from its header
            s.file.top = t.saturating_sub(1);
            scroll_to_cursor(s);
        }
        None => info(s, Level::Info, if forward { "last hunk" } else { "first hunk" }),
    }
}

fn blame_jump(s: &mut AppState) {
    if s.view != ViewMode::Blame {
        return;
    }
    let Some(Ok(blame)) = s.blame() else { return };
    let Some(line) = blame.lines.get(s.file.cursor) else { return };
    if line.commit.is_null() {
        info(s, Level::Info, "line not committed yet");
        return;
    }
    let rev = RevisionId::Commit(line.commit.clone());
    match s.timeline.position_of(&rev) {
        Some(i) => {
            select(s, i);
            info(s, Level::Info, format!("jumped to {}", line.commit.short()));
        }
        None => info(s, Level::Warn, format!("{} is not in the current timeline", line.commit.short())),
    }
}

// ─────────────────────────── view & mark ───────────────────────────

fn set_view(s: &mut AppState, v: ViewMode) {
    if s.view == v {
        return;
    }
    let anchor = capture_anchor(s);
    s.view = v;
    s.file.pending_anchor = anchor;
    s.needs_requests = true;
    if v == ViewMode::DiffMarked && s.mark.is_none() {
        info(s, Level::Info, "press m on a revision to mark it as the compare base");
    }
}

fn toggle_mark(s: &mut AppState) {
    let Some(sel) = s.selected.clone() else { return };
    if s.mark.as_ref() == Some(&sel) {
        s.mark = None;
        info(s, Level::Info, "mark cleared");
    } else {
        let label = s.selected_entry().map(|e| e.label()).unwrap_or_default();
        s.mark = Some(sel);
        if s.view != ViewMode::DiffMarked {
            info(s, Level::Info, format!("marked {label} — press 3 to compare with it"));
        }
    }
    s.needs_requests = true;
}

fn show_details(s: &mut AppState) -> Vec<Effect> {
    let Some(oid) = s.selected_entry().and_then(|e| e.oid()).cloned() else {
        info(s, Level::Info, "no commit details for this row");
        return Vec::new();
    };
    s.overlay = Some(Overlay::Details { oid: oid.clone(), scroll: 0 });
    if s.cache.details.peek(&oid).is_none() && s.in_flight.insert(JobKey::Details(oid.clone())) {
        return vec![Effect::Submit(Job::LoadDetails { oid }, Priority::High)];
    }
    Vec::new()
}

// ─────────────────────────── operations ───────────────────────────

fn selected_bytes(s: &AppState) -> Result<(TimelineEntry, std::sync::Arc<[u8]>), String> {
    let e = s.selected_entry().ok_or("nothing selected")?.clone();
    let content = match s.cache.content_for(&e.blob)? {
        Some((_, c)) => c,
        None => return Err("content not loaded yet".into()),
    };
    match &*content {
        FileContent::Absent => Err("the file does not exist at this revision".into()),
        FileContent::Submodule { .. } => Err("submodules have no file content".into()),
        FileContent::TooLarge { .. } => Err("file too large".into()),
        c => Ok((e, c.bytes().expect("text/binary/symlink have bytes"))),
    }
}

fn start_export(s: &mut AppState) {
    match selected_bytes(s) {
        Ok((e, _)) => {
            let name = ops::export::default_name(&e);
            s.overlay = Some(Overlay::ExportPrompt { rev: e.rev, input: TextInput::new(name), error: None });
        }
        Err(msg) => info(s, Level::Warn, format!("cannot export: {msg}")),
    }
}

fn start_restore(s: &mut AppState) {
    if s.selected_entry().is_some_and(|e| e.rev == RevisionId::WorkingTree) {
        info(s, Level::Warn, "this is already the working-tree version");
        return;
    }
    match selected_bytes(s) {
        Ok((e, _)) => {
            let overwrites = s.timeline.entries.iter().any(|x| x.rev == RevisionId::WorkingTree);
            s.overlay = Some(Overlay::ConfirmRestore { rev: e.rev, overwrites_local_changes: overwrites });
        }
        Err(msg) => info(s, Level::Warn, format!("cannot restore: {msg}")),
    }
}

fn resolve_dest(s: &AppState, input: &str) -> PathBuf {
    let p = match input.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(rest),
        None => PathBuf::from(input),
    };
    if p.is_absolute() {
        p
    } else {
        s.ctx.invocation_dir.join(p)
    }
}

/// Handles actions while an overlay is open. `None` = fall through to normal handling.
fn overlay_action(s: &mut AppState, a: &Action) -> Option<Vec<Effect>> {
    let overlay = s.overlay.as_mut()?;
    let mut fx = Vec::new();
    match (overlay, a) {
        (_, Action::JobDone(_) | Action::OpDone(_) | Action::Layout(_) | Action::Tick | Action::Quit) => return None,

        (Overlay::Help { scroll } | Overlay::Details { scroll, .. }, Action::CursorDown(n)) => *scroll += n,
        (Overlay::Help { scroll } | Overlay::Details { scroll, .. }, Action::CursorUp(n)) => {
            *scroll = scroll.saturating_sub(*n)
        }
        (Overlay::Help { .. } | Overlay::Details { .. } | Overlay::Error { .. }, Action::Cancel | Action::Confirm) => {
            s.overlay = None
        }

        (Overlay::Search(p), a) => match a {
            Action::Confirm => search::confirm(s),
            Action::Cancel => search::cancel(s),
            a => {
                if edit_input(&mut p.input, a) {
                    search::edited(s);
                }
            }
        },

        (Overlay::ExportPrompt { input, error, .. }, a) => {
            match a {
                Action::Cancel => s.overlay = None,
                Action::Confirm => {
                    let value = input.value.trim().to_string();
                    if value.is_empty() {
                        *error = Some("enter a file name".into());
                        return Some(fx);
                    }
                    let dest = resolve_dest(s, &value);
                    match selected_bytes(s) {
                        Ok((_, bytes)) => {
                            s.overlay = None;
                            fx.push(Effect::Export { bytes, dest, overwrite: false });
                        }
                        Err(m) => {
                            if let Some(Overlay::ExportPrompt { error, .. }) = &mut s.overlay {
                                *error = Some(m);
                            }
                        }
                    }
                }
                a => {
                    edit_input(input, a);
                }
            }
        }

        (Overlay::ConfirmRestore { .. }, Action::Confirm) => {
            s.overlay = None;
            match selected_bytes(s) {
                Ok((e, bytes)) => fx.push(Effect::Restore {
                    bytes,
                    mode: if e.mode == FileMode::Absent { FileMode::Regular } else { e.mode },
                    target: s.ctx.target.to_fs_path(&s.ctx.toplevel),
                }),
                Err(m) => info(s, Level::Error, format!("cannot restore: {m}")),
            }
        }
        (Overlay::ConfirmOverwrite { dest, .. }, Action::Confirm) => {
            let dest = dest.clone();
            s.overlay = None;
            match selected_bytes(s) {
                Ok((_, bytes)) => fx.push(Effect::Export { bytes, dest, overwrite: true }),
                Err(m) => info(s, Level::Error, format!("cannot export: {m}")),
            }
        }
        (Overlay::ConfirmRestore { .. } | Overlay::ConfirmOverwrite { .. }, Action::Cancel) => s.overlay = None,

        // everything else is swallowed while a modal is open
        _ => {}
    }
    Some(fx)
}

/// Text-input editing shared by the export and search prompts. True = the text changed.
fn edit_input(input: &mut TextInput, a: &Action) -> bool {
    match a {
        Action::InputChar(c) => input.insert(&c.to_string()),
        Action::InputPaste(t) => input.insert(&t.replace(['\n', '\r'], "")),
        Action::InputBackspace => input.backspace(),
        Action::InputDelete => input.delete(),
        Action::InputLeft => input.cursor = input.cursor.saturating_sub(1),
        Action::InputRight => input.cursor = (input.cursor + 1).min(input.value.chars().count()),
        Action::InputHome => input.cursor = 0,
        Action::InputEnd => input.cursor = input.value.chars().count(),
        _ => return false,
    }
    matches!(a, Action::InputChar(_) | Action::InputPaste(_) | Action::InputBackspace | Action::InputDelete)
}

fn op_done(s: &mut AppState, r: OpResult) -> Vec<Effect> {
    match r {
        OpResult::Copied { text, method } => {
            let via = match method {
                ops::clipboard::CopyMethod::Clipboard => "",
                ops::clipboard::CopyMethod::Osc52 => " (via terminal OSC 52)",
            };
            info(s, Level::Info, format!("copied {text}{via}"));
        }
        OpResult::Exported { dest } => info(s, Level::Info, format!("exported to {}", short_path(s, &dest))),
        OpResult::ExportExists { dest } => {
            if let Some(rev) = s.selected.clone() {
                s.overlay = Some(Overlay::ConfirmOverwrite { rev, dest });
            }
        }
        OpResult::Restored { target } => {
            let label = s.selected_entry().map(|e| e.label()).unwrap_or_default();
            info(s, Level::Info, format!("restored {} from {label} (not staged)", short_path(s, &target)));
            s.cache.invalidate_local();
            s.needs_requests = true;
            return vec![Effect::Submit(Job::LoadLocalState, Priority::High)];
        }
        OpResult::Failed { op, error } => {
            s.overlay = Some(Overlay::Error { message: format!("{op} failed: {error}") });
        }
    }
    Vec::new()
}

fn short_path(s: &AppState, p: &Path) -> String {
    p.strip_prefix(&s.ctx.invocation_dir).unwrap_or(p).display().to_string()
}

fn reload(s: &mut AppState) -> Vec<Effect> {
    s.timeline_gen += 1;
    s.timeline.entries.retain(|e| e.is_local());
    s.timeline.complete = false;
    s.timeline_loading = true;
    s.cache.invalidate_local();
    s.in_flight.retain(|k| !matches!(k, JobKey::Content(BlobSource::WorkingTree(_))));
    search::reset_timeline_hits(s);
    s.needs_requests = true;
    info(s, Level::Info, "reloading…");
    vec![
        Effect::Submit(Job::LoadLocalState, Priority::High),
        Effect::Submit(Job::LoadTimeline { gen: s.timeline_gen }, Priority::High),
    ]
}

// ─────────────────────────── job results ───────────────────────────

fn job_done(s: &mut AppState, r: JobResult) -> Vec<Effect> {
    s.needs_requests = true;
    match r {
        JobResult::TimelineChunk { gen, entries } => {
            if gen == s.timeline_gen {
                s.timeline.entries.extend(entries);
                reselect(s);
            }
        }
        JobResult::TimelineDone { gen, result } => {
            if gen != s.timeline_gen {
                return Vec::new();
            }
            s.timeline_loading = false;
            s.timeline.complete = true;
            match result {
                Err(e) if s.timeline.is_empty() => s.fatal = Some(e),
                Err(e) => info(s, Level::Error, format!("history incomplete: {e}")),
                Ok(()) => {
                    if s.timeline.is_empty() {
                        s.fatal = Some(format!("no history for '{}'", s.ctx.target.display()));
                    } else if matches!(&s.status, Some(m) if m.text == "reloading…") {
                        s.status = None;
                    }
                }
            }
            reselect(s);
        }
        JobResult::LocalState(Ok(local)) => {
            s.timeline.set_local(local);
            search::reset_timeline_hits(s);
            reselect(s);
        }
        JobResult::LocalState(Err(e)) => info(s, Level::Error, format!("git status failed: {e}")),
        JobResult::Content { source, result } => {
            s.in_flight.remove(&JobKey::Content(source.clone()));
            match result {
                Ok((key, c)) => s.cache.insert_content(source, key, c),
                Err(e) => {
                    s.cache.content_errors.insert(source, e);
                }
            }
        }
        JobResult::Highlight { key, result } => {
            s.in_flight.remove(&JobKey::Highlight(key.clone()));
            s.cache.highlight.put(key, result);
        }
        JobResult::Diff { key, diff } => {
            s.in_flight.remove(&JobKey::Diff(key.clone()));
            if diff.timed_out {
                info(s, Level::Warn, "diff simplified: file too large for an exact diff");
            }
            s.cache.diff.put(key, diff);
        }
        JobResult::Blame { key, result } => {
            s.in_flight.remove(&JobKey::Blame(key.clone()));
            s.cache.blame.put(key, result);
        }
        JobResult::Details { oid, result } => {
            s.in_flight.remove(&JobKey::Details(oid.clone()));
            s.cache.details.put(oid, result);
        }
    }
    Vec::new()
}

// ─────────────────────────── requests ───────────────────────────

fn want_content(s: &mut AppState, src: &BlobSource, mode: FileMode, prio: Priority, fx: &mut Vec<Effect>) {
    if *src == BlobSource::Absent || !matches!(s.cache.content_for(src), Ok(None)) {
        return;
    }
    if s.in_flight.insert(JobKey::Content(src.clone())) {
        fx.push(Effect::Submit(Job::LoadContent { source: src.clone(), mode }, prio));
    }
}

fn want_highlight(s: &mut AppState, src: &BlobSource, path: &crate::model::RepoPath, fx: &mut Vec<Effect>) {
    if !s.highlight_enabled {
        return;
    }
    let Ok(Some((key, content))) = s.cache.content_for(src) else { return };
    if content.as_text().is_none() {
        return;
    }
    let hk = s.highlight_key(&key, path);
    if s.cache.highlight.peek(&hk).is_none() && s.in_flight.insert(JobKey::Highlight(hk.clone())) {
        fx.push(Effect::Submit(Job::Highlight { key: hk, content, path: path.clone() }, Priority::High));
    }
}

fn want_diff(s: &mut AppState, old: ContentKey, new: ContentKey, fx: &mut Vec<Effect>) {
    let (Some(o), Some(n)) = (s.cache.content_by_key(&old), s.cache.content_by_key(&new)) else { return };
    if !diffable(&o) || !diffable(&n) {
        return;
    }
    let key = DiffKey { old, new };
    if s.cache.diff.peek(&key).is_none() && s.in_flight.insert(JobKey::Diff(key.clone())) {
        fx.push(Effect::Submit(Job::ComputeDiff { key, old: o, new: n }, Priority::High));
    }
}

fn request_missing(s: &mut AppState) -> Vec<Effect> {
    let mut fx = Vec::new();
    let Some(e) = s.selected_entry().cloned() else { return fx };
    match s.view {
        ViewMode::Content | ViewMode::Blame => {
            want_content(s, &e.blob, e.mode, Priority::High, &mut fx);
            want_highlight(s, &e.blob, &e.path, &mut fx);
            if s.view == ViewMode::Blame {
                let is_text = matches!(s.cache.content_for(&e.blob), Ok(Some((_, c))) if c.as_text().is_some());
                if let (true, Some(key)) = (is_text, s.blame_key(&e)) {
                    if s.cache.blame.peek(&key).is_none() && s.in_flight.insert(JobKey::Blame(key.clone())) {
                        let staged = match e.rev {
                            RevisionId::Index => s.cache.content_for(&e.blob).ok().flatten().map(|(_, c)| c),
                            _ => None,
                        };
                        fx.push(Effect::Submit(Job::LoadBlame { key, entry: e.clone(), staged }, Priority::High));
                    }
                }
            }
        }
        ViewMode::DiffPrev | ViewMode::DiffMarked => {
            let Some(sides) = s.diff_sides() else { return fx };
            let (ob, om, op) = (sides.old.0.clone(), sides.old.1, sides.old.2.clone());
            let (nb, nm, np) = (sides.new.0.clone(), sides.new.1, sides.new.2.clone());
            want_content(s, &ob, om, Priority::High, &mut fx);
            want_content(s, &nb, nm, Priority::High, &mut fx);
            want_highlight(s, &ob, &op, &mut fx);
            want_highlight(s, &nb, &np, &mut fx);
            if let (Some(ok), Some(nk)) = (s.cache.key_of(&ob), s.cache.key_of(&nb)) {
                want_diff(s, ok, nk, &mut fx);
            }
        }
    }
    fx
}

fn prefetch(s: &mut AppState) -> Vec<Effect> {
    let mut fx = Vec::new();
    let i = s.selected_idx;
    let lo = i.saturating_sub(PREFETCH);
    let hi = (i + PREFETCH).min(s.timeline.len().saturating_sub(1));
    for j in lo..=hi {
        if j == i || j >= s.timeline.len() {
            continue;
        }
        let e = s.timeline.entries[j].clone();
        want_content(s, &e.blob, e.mode, Priority::Low, &mut fx);
        if s.view == ViewMode::DiffPrev {
            want_content(s, &e.prev_blob, e.prev_mode, Priority::Low, &mut fx);
        }
    }
    fx
}

fn resolve_anchor(s: &mut AppState) -> Vec<Effect> {
    let Some(anchor) = s.file.pending_anchor.clone() else { return Vec::new() };
    let disp = s.displayed();
    let new_key = match &disp {
        Displayed::Waiting => return Vec::new(),
        d => match d.anchor_key() {
            Some(k) => k.clone(),
            None => {
                // not text here: keep the anchor for the next text version
                s.file.cursor = 0;
                s.file.top = 0;
                return Vec::new();
            }
        },
    };
    let line = if anchor.content == new_key {
        anchor.line
    } else {
        let key = DiffKey { old: anchor.content.clone(), new: new_key.clone() };
        match s.cache.diff.peek(&key).cloned() {
            Some(d) => map_line(&d, anchor.line, true).line(),
            None => {
                let mut fx = Vec::new();
                want_diff(s, anchor.content.clone(), new_key, &mut fx);
                if !fx.is_empty() || s.in_flight.contains(&JobKey::Diff(key)) {
                    return fx; // wait for the mapping diff
                }
                anchor.line // old version evicted or not text: best effort
            }
        }
    };
    let row = disp.row_of_line(line);
    let rows = disp.row_count().unwrap_or(0);
    s.file.pending_anchor = None;
    s.file.cursor = row.min(rows.saturating_sub(1));
    s.file.top = s.file.cursor.saturating_sub(anchor.viewport_offset);
    // don't leave empty rows at the bottom when the content could fill them
    s.file.top = s.file.top.min(rows.saturating_sub(s.file_rows()));
    if let Displayed::Diff { rows, .. } = &disp {
        if s.file.top > 0 && matches!(rows.get(s.file.top - 1), Some(DiffRow::HunkHeader(_))) {
            s.file.top -= 1;
        }
    }
    scroll_to_cursor(s);
    Vec::new()
}

fn touch_displayed(s: &mut AppState) {
    let mut keys = Vec::new();
    if let Some(e) = s.selected_entry() {
        for src in [&e.blob, &e.prev_blob] {
            if let Some(k) = s.cache.key_of(src) {
                keys.push(k);
            }
        }
    }
    if let Some(a) = &s.file.pending_anchor {
        keys.push(a.content.clone());
    }
    s.cache.touch(&keys);
}
