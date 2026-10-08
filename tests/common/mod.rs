#![allow(dead_code)]

use std::cell::Cell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;
use git_timeline::cli::Cli;
use git_timeline::git::repo::RepoContext;
use git_timeline::git::Git;

pub struct Fixture {
    pub dir: tempfile::TempDir,
    day: Cell<u32>,
}

pub fn env() -> Vec<(OsString, OsString)> {
    [
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_AUTHOR_NAME", "Alice"),
        ("GIT_AUTHOR_EMAIL", "alice@example.com"),
        ("GIT_COMMITTER_NAME", "Alice"),
        ("GIT_COMMITTER_EMAIL", "alice@example.com"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}

pub fn base_git() -> Git {
    Git::with_env(PathBuf::new(), env())
}

impl Fixture {
    pub fn new() -> Self {
        let f = Fixture { dir: tempfile::tempdir().unwrap(), day: Cell::new(1) };
        f.git(&["init", "-q", "-b", "main"]);
        f.git(&["config", "commit.gpgsign", "false"]);
        f.git(&["config", "core.autocrlf", "false"]);
        f
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn git(&self, args: &[&str]) -> String {
        let day = self.day.get();
        let date = format!("2024-01-{:02}T12:00:00+00:00", day.min(28));
        let out = Command::new("git")
            .current_dir(self.path())
            .args(args)
            .envs(env())
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    pub fn write(&self, rel: &str, content: &str) -> &Self {
        self.write_bytes(rel, content.as_bytes())
    }

    pub fn write_bytes(&self, rel: &str, content: &[u8]) -> &Self {
        let p = self.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
        self
    }

    pub fn stage(&self, rel: &str) -> &Self {
        self.git(&["add", "--", rel]);
        self
    }

    pub fn mv(&self, from: &str, to: &str) -> &Self {
        if let Some(parent) = Path::new(to).parent() {
            std::fs::create_dir_all(self.path().join(parent)).unwrap();
        }
        self.git(&["mv", from, to]);
        self
    }

    pub fn rm(&self, rel: &str) -> &Self {
        self.git(&["rm", "-q", rel]);
        self
    }

    pub fn commit(&self, msg: &str) -> String {
        self.git(&["commit", "-q", "--allow-empty", "-m", msg]);
        self.day.set(self.day.get() + 1);
        self.git(&["rev-parse", "HEAD"])
    }

    pub fn commit_as(&self, author: &str, msg: &str) -> String {
        let a = format!("{author} <{}@example.com>", author.to_lowercase());
        self.git(&["commit", "-q", "--author", &a, "-m", msg]);
        self.day.set(self.day.get() + 1);
        self.git(&["rev-parse", "HEAD"])
    }

    /// Write, stage and commit in one go.
    pub fn edit(&self, rel: &str, content: &str, msg: &str) -> String {
        self.write(rel, content).stage(rel);
        self.commit(msg)
    }

    pub fn context(&self, cwd_rel: &str, args: &[&str]) -> git_timeline::Result<RepoContext> {
        let mut argv = vec!["git-timeline"];
        argv.extend_from_slice(args);
        let cli = Cli::parse_from(argv);
        RepoContext::discover_with(&cli, self.path().canonicalize().unwrap().join(cwd_rel), base_git())
    }
}

/// a.txt edited 5 times: "1\n", "1\n2\n", …
pub fn linear_history() -> (Fixture, Vec<String>) {
    let f = Fixture::new();
    let mut oids = Vec::new();
    let mut content = String::new();
    for i in 1..=5 {
        content += &format!("{i}\n");
        oids.push(f.edit("a.txt", &content, &format!("edit {i}")));
    }
    (f, oids)
}

/// a.txt (2 edits) → git mv to dir/b.txt → 1 edit.
pub fn renamed_history() -> (Fixture, Vec<String>) {
    let f = Fixture::new();
    let mut oids = Vec::new();
    oids.push(f.edit("a.txt", "one\ntwo\nthree\n", "add a"));
    oids.push(f.edit("a.txt", "one\ntwo\nthree\nfour\n", "edit a"));
    f.mv("a.txt", "dir/b.txt");
    oids.push(f.commit("move a to b"));
    oids.push(f.edit("dir/b.txt", "zero\none\ntwo\nthree\nfour\n", "edit b"));
    (f, oids)
}

pub fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}
