mod common;

use std::sync::atomic::AtomicBool;

use common::*;
use git_timeline::error::Error;
use git_timeline::git::log::stream_timeline;
use git_timeline::git::repo::RepoContext;
use git_timeline::git::status::{local_entries, read_local_state};
use git_timeline::model::{BlobSource, ChangeKind, RevisionId, TimelineEntry};

fn commits(ctx: &RepoContext) -> Vec<TimelineEntry> {
    let mut all = Vec::new();
    stream_timeline(&ctx.git, &ctx.query, &AtomicBool::new(false), &mut |c| all.extend(c)).unwrap();
    all
}

fn locals(ctx: &RepoContext) -> Vec<TimelineEntry> {
    local_entries(&read_local_state(&ctx.git, &ctx.target).unwrap(), &ctx.target)
}

fn oid_of(e: &TimelineEntry) -> String {
    e.oid().unwrap().to_string()
}

#[test]
fn linear() {
    let (f, oids) = linear_history();
    let ctx = f.context("", &["a.txt"]).unwrap();
    let t = commits(&ctx);
    assert_eq!(t.len(), 5);
    assert_eq!(oid_of(&t[0]), oids[4]);
    assert_eq!(t[4].change, ChangeKind::Added);
    assert_eq!(t[4].prev_blob, BlobSource::Absent);
    for i in 0..4 {
        assert_eq!(t[i].change, ChangeKind::Modified);
        assert_eq!(t[i].prev_blob, t[i + 1].blob, "prev_blob of {i}");
    }
    assert!(locals(&ctx).is_empty());
}

#[test]
fn follow_rename() {
    let (f, oids) = renamed_history();
    let ctx = f.context("", &["dir/b.txt"]).unwrap();
    let t = commits(&ctx);
    assert_eq!(t.len(), 4);
    assert_eq!(oid_of(&t[1]), oids[2]);
    match &t[1].change {
        ChangeKind::Renamed { from, .. } => assert_eq!(from.as_bytes(), b"a.txt"),
        c => panic!("expected rename, got {c:?}"),
    }
    assert_eq!(t[0].path.as_bytes(), b"dir/b.txt");
    assert_eq!(t[1].path.as_bytes(), b"dir/b.txt");
    assert_eq!(t[2].path.as_bytes(), b"a.txt");
    assert_eq!(t[3].path.as_bytes(), b"a.txt");
}

#[test]
fn no_follow() {
    let (f, _) = renamed_history();
    let ctx = f.context("", &["--no-follow", "dir/b.txt"]).unwrap();
    let t = commits(&ctx);
    assert_eq!(t.len(), 2);
    assert_eq!(t[1].change, ChangeKind::Added);
}

#[test]
fn line_range() {
    let f = Fixture::new();
    f.edit("a.txt", "a\nb\nc\nd\ne\n", "add");
    let touching = f.edit("a.txt", "a\nB\nc\nd\ne\n", "touch line 2");
    f.edit("a.txt", "a\nB\nc\nd\nE\n", "touch line 5");
    f.mv("a.txt", "b.txt");
    f.commit("rename");
    let ctx = f.context("", &["-L", "2,3", "b.txt"]).unwrap();
    let t = commits(&ctx);
    assert_eq!(t.len(), 2, "{t:?}");
    assert_eq!(oid_of(&t[0]), touching);
    assert_eq!(t[0].path.as_bytes(), b"a.txt");
}

#[test]
fn author_filter_keeps_real_parent() {
    let f = Fixture::new();
    f.write("a.txt", "1\n").stage("a.txt");
    f.commit_as("Alice", "a1");
    f.write("a.txt", "1\n2\n").stage("a.txt");
    f.commit_as("Bob", "b1");
    f.write("a.txt", "1\n2\n3\n").stage("a.txt");
    f.commit_as("Alice", "a2");
    let ctx = f.context("", &["--author=Alice", "a.txt"]).unwrap();
    let t = commits(&ctx);
    assert_eq!(t.len(), 2);
    let bob_blob = f.git(&["rev-parse", "HEAD~1:a.txt"]);
    assert_eq!(t[0].prev_blob, BlobSource::Object(git_timeline::model::Oid::parse(&bob_blob).unwrap()));
}

#[test]
fn range() {
    let (f, _) = linear_history();
    let ctx = f.context("", &["a.txt", "HEAD~2..HEAD"]).unwrap();
    assert_eq!(commits(&ctx).len(), 2);
    assert!(!ctx.show_local);
}

#[test]
fn merge() {
    let f = Fixture::new();
    f.edit("a.txt", "a\nb\nc\n", "base");
    f.git(&["checkout", "-q", "-b", "side"]);
    f.edit("a.txt", "a\nb\nC\n", "side change");
    f.git(&["checkout", "-q", "main"]);
    f.edit("a.txt", "A\nb\nc\n", "main change");
    f.git(&["merge", "-q", "--no-ff", "-m", "merge side", "side"]);
    let ctx = f.context("", &["a.txt"]).unwrap();
    let t = commits(&ctx);
    assert_eq!(t.len(), 4);
    assert_eq!(t[0].change, ChangeKind::Merge);
    // the merge row holds the merged version (= HEAD), diffed against the first parent
    let head = f.git(&["rev-parse", "HEAD:a.txt"]);
    let first_parent = f.git(&["rev-parse", "HEAD^1:a.txt"]);
    assert_eq!(t[0].blob, BlobSource::Object(git_timeline::model::Oid::parse(&head).unwrap()));
    assert_eq!(t[0].prev_blob, BlobSource::Object(git_timeline::model::Oid::parse(&first_parent).unwrap()));
}

#[test]
fn deleted_file() {
    let f = Fixture::new();
    f.edit("a.txt", "x\n", "add");
    f.rm("a.txt");
    f.commit("delete");
    let ctx = f.context("", &["a.txt"]).unwrap();
    let t = commits(&ctx);
    assert_eq!(t[0].change, ChangeKind::Deleted);
    assert_eq!(t[0].blob, BlobSource::Absent);
}

#[test]
fn pseudo_entries() {
    let (f, _) = linear_history();
    f.write("a.txt", "staged\n").stage("a.txt");
    f.write("a.txt", "worktree\n");
    let ctx = f.context("", &["a.txt"]).unwrap();
    let l = locals(&ctx);
    assert_eq!(l.len(), 2);
    assert_eq!(l[0].rev, RevisionId::WorkingTree);
    assert_eq!(l[1].rev, RevisionId::Index);
    assert_eq!(l[0].prev_blob, l[1].blob);
    assert!(matches!(l[0].blob, BlobSource::WorkingTree(_)));
}

#[test]
fn untracked_and_unborn() {
    let f = Fixture::new();
    f.write("new.txt", "hello\n");
    let ctx = f.context("", &["new.txt"]).unwrap();
    let l = locals(&ctx);
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].change, ChangeKind::Untracked);

    f.stage("new.txt");
    let l = locals(&f.context("", &["new.txt"]).unwrap());
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].rev, RevisionId::Index);
    assert_eq!(l[0].change, ChangeKind::Added);
}

#[test]
fn weird_names_and_subdir() {
    let f = Fixture::new();
    f.edit("sub/a*b [x].txt", "1\n", "add");
    f.edit("sub/other.txt", "1\n", "glob trap");
    let ctx = f.context("sub", &["a*b [x].txt"]).unwrap();
    assert_eq!(ctx.target.as_bytes(), b"sub/a*b [x].txt");
    assert_eq!(commits(&ctx).len(), 1);

    f.edit("top.txt", "1\n", "top");
    let ctx = f.context("sub", &["../top.txt"]).unwrap();
    assert_eq!(ctx.target.as_bytes(), b"top.txt");
}

#[test]
fn errors() {
    let (f, _) = linear_history();
    std::fs::create_dir(f.path().join("d")).unwrap();
    assert!(matches!(f.context("", &["d"]), Err(Error::IsADirectory(_))));
    assert!(matches!(f.context("", &["nope.txt"]), Err(Error::NoHistory(_))));
    assert!(matches!(f.context("", &["a.txt", "nope"]), Err(Error::BadRevision(_))));
    assert!(matches!(f.context("", &["../../outside.txt"]), Err(Error::OutsideRepository(_))));
    assert!(matches!(f.context("", &["-L", "50,60", "a.txt"]), Err(Error::Git { .. })));
}

#[test]
fn streaming_many_commits() {
    let f = Fixture::new();
    // fast-import is much faster than 300 real commits
    let mut script = String::new();
    for i in 0..300 {
        let data = format!("{i}\n");
        script += &format!(
            "commit refs/heads/main\ncommitter A <a@x> {} +0000\ndata 2\nc\nM 100644 inline f.txt\ndata {}\n{}\n",
            1_700_000_000 + i,
            data.len(),
            data
        );
    }
    let mut child = std::process::Command::new("git")
        .current_dir(f.path())
        .args(["fast-import", "--quiet"])
        .envs(env())
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success());
    f.git(&["checkout", "-q", "main"]);

    let ctx = f.context("", &["f.txt"]).unwrap();
    let mut chunks = 0;
    let mut total = 0;
    stream_timeline(&ctx.git, &ctx.query, &AtomicBool::new(false), &mut |c| {
        chunks += 1;
        total += c.len();
    })
    .unwrap();
    assert_eq!(total, 300);
    assert!(chunks >= 1);
}
