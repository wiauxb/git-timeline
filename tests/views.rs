//! Reducer + views without a terminal: jobs are run inline.

mod common;

use std::sync::Arc;

use common::*;
use git_timeline::app::action::{Action, Effect};
use git_timeline::app::state::{AppState, Displayed, Focus, Overlay, ViewMode};
use git_timeline::app::update::{after_batch, update};
use git_timeline::cli::{InitialView, UiOptions};
use git_timeline::git::blob::BlobReader;
use git_timeline::git::repo::RepoContext;
use git_timeline::highlight::Highlighter;
use git_timeline::model::{BlobSource, DiffLine, DiffRow, FileContent, FileMode, RevisionId};
use git_timeline::ui::{self, LayoutInfo};
use git_timeline::worker::{run_inline, Job, JobResult};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

struct Harness {
    s: AppState,
    ctx: Arc<RepoContext>,
    effects: Vec<Effect>,
}

impl Harness {
    fn new(f: &Fixture, args: &[&str]) -> Self {
        let ctx = Arc::new(f.context("", args).unwrap());
        let opts = UiOptions { initial_view: InitialView::Content, tab_width: 4 };
        let mut s = AppState::new(ctx.clone(), opts, false);
        s.layout = LayoutInfo {
            timeline: Rect::new(0, 0, 40, 30),
            file: Rect::new(40, 0, 80, 30),
            file_body: Rect::new(41, 2, 78, 20),
            file_body_rows: 20,
            file_body_cols: 78,
        };
        let mut h = Harness { s, ctx, effects: Vec::new() };
        h.feed(Job::LoadLocalState);
        h.feed(Job::LoadTimeline { gen: 0 });
        h.settle();
        h
    }

    fn feed(&mut self, job: Job) {
        for r in run_inline(self.ctx.clone(), Highlighter::disabled(), job) {
            let fx = update(&mut self.s, Action::JobDone(r));
            self.run_effects(fx);
        }
    }

    fn run_effects(&mut self, fx: Vec<Effect>) {
        for e in fx {
            match e {
                Effect::Submit(job, _) => self.feed(job),
                other => self.effects.push(other),
            }
        }
    }

    /// Run requests until nothing is missing anymore.
    fn settle(&mut self) {
        for _ in 0..20 {
            let fx = after_batch(&mut self.s);
            if fx.is_empty() && !self.s.needs_requests {
                return;
            }
            self.run_effects(fx);
        }
        panic!("did not settle");
    }

    fn act(&mut self, a: Action) {
        let fx = update(&mut self.s, a);
        self.run_effects(fx);
        self.settle();
    }

    fn render(&self) -> String {
        let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
        term.draw(|f| {
            ui::draw(f, &self.s);
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn numbered(range: std::ops::Range<usize>) -> String {
    range.map(|i| format!("line {i}\n")).collect()
}

#[test]
fn anchor_follows_code() {
    let f = Fixture::new();
    f.edit("a.txt", &numbered(0..50), "base");
    let top: String = (0..10).map(|i| format!("new {i}\n")).collect();
    f.edit("a.txt", &(top + &numbered(0..50)), "prepend 10 lines");

    let mut h = Harness::new(&f, &["a.txt"]);
    assert_eq!(h.s.selected_idx, 0);
    h.act(Action::FocusFile);
    h.act(Action::CursorDown(45)); // "line 35" at HEAD
    h.act(Action::SelectNext); // older commit
    assert_eq!(h.s.selected_idx, 1);
    assert_eq!(h.s.file.cursor, 35, "cursor follows 'line 35' to its old position");
    h.act(Action::SelectPrev);
    assert_eq!(h.s.file.cursor, 45);
}

#[test]
fn anchor_same_blob_keeps_position() {
    let (f, _) = renamed_history();
    let mut h = Harness::new(&f, &["dir/b.txt"]);
    h.act(Action::SelectNext); // rename commit (R100: same blob as the one below)
    h.act(Action::CursorDown(2));
    let before = (h.s.file.cursor, h.s.file.top);
    h.act(Action::SelectNext);
    assert_eq!((h.s.file.cursor, h.s.file.top), before);
}

#[test]
fn diff_prev_matches_git_numstat() {
    let (f, oids) = renamed_history();
    let mut h = Harness::new(&f, &["dir/b.txt"]);
    h.act(Action::SetView(ViewMode::DiffPrev));
    for (i, oid) in oids.iter().rev().enumerate() {
        h.act(Action::SelectIndex(i));
        let Displayed::Diff { diff, .. } = h.s.displayed() else { panic!("no diff for {i}") };
        let numstat = f.git(&["show", "--format=", "--numstat", "-M", oid]);
        let mut it = numstat.split_whitespace();
        let (a, r): (usize, usize) = (it.next().unwrap().parse().unwrap(), it.next().unwrap().parse().unwrap());
        assert_eq!((diff.added, diff.removed), (a, r), "commit {i}");
    }
}

#[test]
fn diff_marked_is_ordered_old_to_new() {
    let (f, _) = linear_history();
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::ToggleMark); // mark newest
    h.act(Action::SelectLast); // select oldest
    h.act(Action::SetView(ViewMode::DiffMarked));
    let Displayed::Diff { diff, .. } = h.s.displayed() else { panic!("no diff") };
    assert_eq!((diff.added, diff.removed), (4, 0));
}

#[test]
fn blame_attributes_lines() {
    let (f, oids) = linear_history();
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::SetView(ViewMode::Blame));
    let blame = h.s.blame().expect("blame loaded").unwrap();
    assert_eq!(blame.lines.len(), 5);
    for (i, l) in blame.lines.iter().enumerate() {
        assert_eq!(l.commit.to_string(), oids[i]);
    }
    // jump to the commit of line 2
    h.act(Action::FocusFile);
    h.act(Action::CursorDown(1));
    h.act(Action::BlameJump);
    assert_eq!(h.s.selected, Some(RevisionId::Commit(git_timeline::model::Oid::parse(&oids[1]).unwrap())));
}

#[test]
fn blame_working_tree_and_index() {
    let (f, _) = linear_history();
    f.write("a.txt", "1\n2\nstaged\n").stage("a.txt");
    f.write("a.txt", "1\n2\nstaged\nworktree\n");
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::SetView(ViewMode::Blame));
    let b = h.s.blame().unwrap().unwrap();
    assert!(b.lines[3].commit.is_null());
    h.act(Action::SelectNext); // Staged row
    assert_eq!(h.s.selected, Some(RevisionId::Index));
    let b = h.s.blame().unwrap().unwrap();
    assert_eq!(b.lines.len(), 3);
    assert!(b.lines[2].commit.is_null());
}

#[test]
fn restore_flow() {
    let (f, _) = renamed_history();
    let mut h = Harness::new(&f, &["dir/b.txt"]);
    h.act(Action::SelectLast); // oldest, still named a.txt
    h.act(Action::StartRestore);
    assert!(matches!(h.s.overlay, Some(Overlay::ConfirmRestore { overwrites_local_changes: false, .. })));
    h.act(Action::Confirm);
    let Some(Effect::Restore { bytes, target, mode }) = h.effects.pop() else { panic!("no restore effect") };
    assert_eq!(target, h.ctx.toplevel.join("dir/b.txt"));
    assert_eq!(&*bytes, b"one\ntwo\nthree\n");
    let r = git_timeline::ops::restore::restore(&bytes, mode, &target);
    let fx = update(&mut h.s, Action::OpDone(r));
    h.run_effects(fx);
    h.settle();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "one\ntwo\nthree\n");
    assert_eq!(h.s.timeline.entries[0].rev, RevisionId::WorkingTree);
    assert!(f.git(&["diff", "--cached", "--name-only"]).is_empty(), "index untouched");
}

#[test]
fn restore_refused_for_working_tree_and_deleted() {
    let (f, _) = linear_history();
    f.write("a.txt", "local\n");
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::StartRestore);
    assert!(h.s.overlay.is_none());

    let g = Fixture::new();
    g.edit("x.txt", "x\n", "add");
    g.rm("x.txt");
    g.commit("delete");
    let mut h = Harness::new(&g, &["x.txt"]);
    h.act(Action::StartRestore);
    assert!(h.s.overlay.is_none());
}

#[cfg(unix)]
#[test]
fn restore_sets_exec_bit() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("s.sh");
    std::fs::write(&p, "old").unwrap();
    git_timeline::ops::restore::restore(b"#!/bin/sh\n", FileMode::Executable, &p);
    assert_ne!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o111, 0);
    git_timeline::ops::restore::restore(b"x", FileMode::Regular, &p);
    assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o111, 0);
}

#[test]
fn export_flow_and_default_name() {
    let (f, oids) = renamed_history();
    let mut h = Harness::new(&f, &["dir/b.txt"]);
    h.act(Action::SelectLast);
    h.act(Action::StartExport);
    let Some(Overlay::ExportPrompt { input, .. }) = &h.s.overlay else { panic!("no prompt") };
    assert_eq!(input.value, format!("a@{}.txt", &oids[0][..7]));
    h.act(Action::Confirm);
    let Some(Effect::Export { dest, overwrite: false, .. }) = h.effects.pop() else { panic!("no export") };
    assert_eq!(dest, h.ctx.invocation_dir.join(format!("a@{}.txt", &oids[0][..7])));
}

#[test]
fn content_reader_matches_git_show() {
    let (f, oids) = renamed_history();
    let mut h = Harness::new(&f, &["dir/b.txt"]);
    let mut reader = BlobReader::new(h.ctx.git.clone());
    for (i, e) in h.s.timeline.entries.clone().iter().enumerate() {
        let (_, c) = reader.read(&e.blob, e.mode).unwrap();
        let expected = f.git(&["show", &format!("{}:{}", oids[oids.len() - 1 - i], e.path.display())]);
        assert_eq!(c.as_text().unwrap().line_str(0), expected.lines().next().unwrap());
    }
    h.act(Action::Reload);
    assert_eq!(h.s.timeline.len(), 4);
}

#[test]
fn binary_symlink_too_large() {
    let f = Fixture::new();
    let mut bin = vec![b'x'; 100];
    bin[5] = 0;
    f.write_bytes("b.bin", &bin).stage("b.bin");
    f.write("big.txt", &"y".repeat(5000)).stage("big.txt");
    #[cfg(unix)]
    std::os::unix::fs::symlink("b.bin", f.path().join("link")).unwrap();
    f.git(&["add", "-A"]);
    f.commit("add");
    let ctx = f.context("", &["b.bin"]).unwrap();
    let mut r = BlobReader::with_limit(ctx.git.clone(), 1000);
    let oid = |p: &str| BlobSource::Object(git_timeline::model::Oid::parse(&f.git(&["rev-parse", &format!("HEAD:{p}")])).unwrap());
    assert!(matches!(r.read(&oid("b.bin"), FileMode::Regular).unwrap().1, FileContent::Binary { .. }));
    assert!(matches!(r.read(&oid("big.txt"), FileMode::Regular).unwrap().1, FileContent::TooLarge { size: 5000, .. }));
    #[cfg(unix)]
    match r.read(&oid("link"), FileMode::Symlink).unwrap().1 {
        FileContent::Symlink { target } => assert_eq!(target, "b.bin"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn renders_every_view() {
    let (f, _) = renamed_history();
    f.write("dir/b.txt", "zero\none\ntwo\nthree\nfour\nfive\n");
    let mut h = Harness::new(&f, &["dir/b.txt"]);
    let screen = h.render();
    assert!(screen.contains("Working tree"), "{screen}");
    assert!(screen.contains("move a to b"), "{screen}");
    assert!(screen.contains("five"), "{screen}");

    h.act(Action::SetView(ViewMode::DiffPrev));
    let screen = h.render();
    assert!(screen.contains("+ five"), "{screen}");
    assert!(screen.contains("@@"), "{screen}");

    h.act(Action::SetView(ViewMode::Blame));
    assert!(h.render().contains("not committed yet"));

    h.act(Action::SetView(ViewMode::DiffMarked));
    assert!(h.render().contains("press m"));

    h.act(Action::SelectIndex(2));
    h.act(Action::ShowDetails);
    assert!(h.render().contains("Commit details"));
    h.act(Action::Cancel);
    h.act(Action::ShowHelp);
    assert!(h.render().contains("Keys"));
    h.act(Action::Cancel);
    h.act(Action::FocusFile);
    h.act(Action::ToggleZoom);
    assert!(!h.render().contains("Timeline"));
    assert_eq!(h.s.focus, Focus::File);
}

#[test]
fn stale_results_are_cached_but_harmless() {
    let (f, _) = linear_history();
    let mut h = Harness::new(&f, &["a.txt"]);
    let far = h.s.timeline.entries[4].clone();
    h.act(Action::FocusFile);
    h.act(Action::CursorDown(3));
    let cursor = h.s.file.cursor;
    for r in run_inline(h.ctx.clone(), Highlighter::disabled(), Job::LoadContent { source: far.blob.clone(), mode: far.mode }) {
        assert!(matches!(r, JobResult::Content { .. }));
        let fx = update(&mut h.s, Action::JobDone(r));
        h.run_effects(fx);
    }
    h.settle();
    assert_eq!(h.s.file.cursor, cursor);
    assert!(matches!(h.s.cache.content_for(&far.blob), Ok(Some(_))));
}

// ───────────────────────────── search ─────────────────────────────

impl Harness {
    fn type_str(&mut self, text: &str) {
        for c in text.chars() {
            self.act(Action::InputChar(c));
        }
    }

    fn status(&self) -> String {
        self.s.status.as_ref().map(|m| m.text.clone()).unwrap_or_default()
    }
}

fn three_commits() -> (Fixture, Vec<String>) {
    let f = Fixture::new();
    let a = f.edit("a.txt", "1\n", "first: alpha");
    let b = f.edit("a.txt", "2\n", "second: beta");
    let c = f.edit("a.txt", "3\n", "third: gamma");
    (f, vec![a, b, c])
}

#[test]
fn search_timeline_hash_and_subject() {
    let (f, oids) = three_commits();
    let mut h = Harness::new(&f, &["a.txt"]);
    assert_eq!(h.s.selected_idx, 0);

    h.act(Action::StartSearch);
    h.type_str("bet");
    assert_eq!(h.s.selected_idx, 1, "live jump while typing");
    h.act(Action::Confirm);
    assert!(h.s.overlay.is_none());
    h.act(Action::SearchNext);
    assert_eq!(h.s.selected_idx, 1, "single hit: wraps onto itself");
    assert_eq!(h.status(), "search wrapped to top");

    // hash prefix, case-insensitive; then Esc restores selection and previous query
    h.act(Action::StartSearch);
    h.type_str(&oids[0][..6].to_uppercase());
    assert_eq!(h.s.selected_idx, 2);
    h.act(Action::Cancel);
    assert_eq!(h.s.selected_idx, 1);
    assert_eq!(h.s.search.timeline.as_ref().map(|q| q.pattern().to_string()).as_deref(), Some("bet"));

    // a hash *suffix* does not match
    h.act(Action::StartSearch);
    h.type_str(&oids[0][34..]);
    assert!(h.s.search.timeline_hits.rows.is_empty());
    h.act(Action::Cancel);

    h.act(Action::Escape);
    assert!(h.s.search.timeline.is_none());
    assert_eq!(h.status(), "search cleared");
}

#[test]
fn search_file_all_views() {
    let f = Fixture::new();
    f.edit("a.txt", &numbered(0..30), "base");
    let changed = numbered(0..30).replace("line 20\n", "line 20 needle\n");
    f.edit("a.txt", &changed, "add needle");
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::FocusFile);

    h.act(Action::StartSearch);
    h.type_str("needle");
    h.act(Action::Confirm);
    assert_eq!(h.s.file.cursor, 20, "content view");

    h.act(Action::SetView(ViewMode::DiffPrev));
    h.act(Action::SearchNext);
    let d = h.s.displayed();
    let Displayed::Diff { rows, .. } = &d else { panic!("diff expected") };
    assert!(matches!(rows[h.s.file.cursor], DiffRow::Line(DiffLine::Added { new: 20 })));
    for &r in &h.s.search.file_hits.rows {
        assert!(!matches!(rows[r], DiffRow::HunkHeader(_)), "hunk headers are not searched");
    }
    drop(d);
    h.act(Action::StartSearch);
    h.type_str("line 20");
    h.act(Action::Confirm);
    let Displayed::Diff { rows, .. } = h.s.displayed() else { panic!() };
    let removed = h.s.search.file_hits.rows.iter().any(|&r| matches!(rows[r], DiffRow::Line(DiffLine::Removed { .. })));
    assert!(removed, "removed rows are searched on the old side");

    // blame: the gutter (hashes, author names) is not searched
    h.act(Action::SetView(ViewMode::Blame));
    h.act(Action::StartSearch);
    h.type_str("Alice");
    h.act(Action::Confirm);
    assert!(h.s.search.file_hits.rows.is_empty(), "{:?}", h.s.search.file_hits.rows);
    h.act(Action::StartSearch);
    h.type_str("needle");
    h.act(Action::Confirm);
    assert_eq!(h.s.search.file_hits.rows, vec![20]);
    h.act(Action::CursorDown(3));

    // older revision: query kept, hits recomputed, cursor anchored (not moved to a match)
    h.act(Action::SelectNext);
    assert_eq!(h.s.selected_idx, 1);
    assert!(h.s.search.file.is_some());
    assert!(h.s.search.file_hits.rows.is_empty());
    assert_eq!(h.s.file.cursor, 23);

    h.act(Action::Escape);
    assert!(h.s.search.file.is_none());
    assert_eq!(h.s.focus, Focus::File, "first Esc only clears the search");
    h.act(Action::Escape);
    assert_eq!(h.s.focus, Focus::Timeline);
}

#[test]
fn search_horizontal_reveal() {
    let f = Fixture::new();
    let long = format!("{}needle\nshort\n", "x".repeat(200));
    f.edit("a.txt", &long, "long line");
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::FocusFile);
    h.act(Action::StartSearch);
    h.type_str("needle");
    h.act(Action::Confirm);
    assert_eq!(h.s.file.hscroll, 192);
    assert!(h.render().contains("needle"));
}

#[test]
fn search_invalid_regex_and_no_match() {
    let (f, _) = three_commits();
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::StartSearch);
    h.type_str("(");
    let Some(Overlay::Search(p)) = &h.s.overlay else { panic!("prompt expected") };
    assert!(p.error.is_some());
    assert!(h.render().contains("invalid regex"));
    h.act(Action::Confirm);
    assert!(matches!(h.s.overlay, Some(Overlay::Search(_))), "invalid regex keeps the prompt open");

    h.act(Action::InputBackspace);
    h.type_str("zzz");
    assert_eq!(h.s.selected_idx, 0);
    h.act(Action::Confirm);
    assert!(h.s.overlay.is_none());
    assert_eq!(h.status(), "pattern not found: zzz");
    h.act(Action::SearchNext);
    assert_eq!(h.status(), "pattern not found: zzz");
}

#[test]
fn search_renders_highlights() {
    let (f, _) = three_commits();
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::StartSearch);
    h.type_str("bet");
    let screen = h.render();
    let last = screen.lines().last().unwrap();
    assert!(last.contains(" /bet") && last.contains("1 match"), "{last}");
    h.act(Action::Confirm);
    let screen = h.render();
    assert!(screen.lines().last().unwrap().contains("/bet 1/1"), "{screen}");

    // the matched cells carry the search style
    let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
    term.draw(|f| {
        ui::draw(f, &h.s);
    })
    .unwrap();
    let buf = term.backend().buffer().clone();
    // skip row 0: the file pane title also shows the subject
    let y = screen.lines().skip(1).position(|l| l.contains("second: beta")).unwrap() as u16 + 1;
    let row: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol().to_string()).collect();
    let x = row.find("beta").unwrap();
    let x = row[..x].chars().count() as u16;
    let hit = git_timeline::ui::theme::theme().search_match;
    let style = buf[(x, y)].style();
    assert!(hit.bg.is_none() || style.bg == hit.bg, "{style:?}");
    assert!(style.add_modifier.contains(hit.add_modifier));
    assert_eq!(buf[(x + 2, y)].style(), style, "'t' is marked");
    assert_ne!(buf[(x + 3, y)].style(), style, "'a' is not");
}

#[test]
fn hunk_keys_moved() {
    use git_timeline::app::keymap::map;
    use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    let (f, _) = three_commits();
    let mut h = Harness::new(&f, &["a.txt"]);
    h.act(Action::FocusFile);
    h.act(Action::SetView(ViewMode::DiffPrev));
    let key = |c| Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    assert!(matches!(map(&h.s, key('}')), Some(Action::NextHunk)));
    assert!(matches!(map(&h.s, key('{')), Some(Action::PrevHunk)));
    assert!(matches!(map(&h.s, key('n')), Some(Action::SearchNext)));
    assert!(matches!(map(&h.s, key('N')), Some(Action::SearchPrev)));
    assert!(matches!(map(&h.s, key('/')), Some(Action::StartSearch)));
    h.act(Action::SearchNext);
    assert_eq!(h.status(), "no search — press / to search");
}
