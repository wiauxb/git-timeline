//! Startup discovery: repository, repo-relative path, validation of options.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use super::log::{self, LogQuery};
use super::Git;
use crate::cli::Cli;
use crate::error::{Error, Result};
use crate::model::RepoPath;

pub const MIN_GIT: (u32, u32, u32) = (2, 36, 0);

#[derive(Clone, Debug)]
pub struct RepoContext {
    pub git: Git,
    pub toplevel: PathBuf,
    /// cwd at launch (default export directory).
    pub invocation_dir: PathBuf,
    /// The file, at the tip of the explored history.
    pub target: RepoPath,
    pub query: LogQuery,
    /// Pseudo-entries enabled (no revision range given).
    pub show_local: bool,
    pub git_version: (u32, u32, u32),
}

impl RepoContext {
    pub fn discover(cli: &Cli, cwd: PathBuf) -> Result<Self> {
        Self::discover_with(cli, cwd, Git::new(PathBuf::new()))
    }

    /// `base` supplies environment settings (tests); its directory is ignored.
    pub fn discover_with(cli: &Cli, cwd: PathBuf, base: Git) -> Result<Self> {
        let probe = base.at(cwd.clone());
        let git_version = git_version(&probe)?;
        if git_version < MIN_GIT {
            return Err(Error::GitTooOld {
                found: format!("{}.{}.{}", git_version.0, git_version.1, git_version.2),
                required: "2.36",
            });
        }

        let top = probe
            .probe(&["rev-parse", "--show-toplevel"])?
            .ok_or(Error::NotARepository)?;
        let top = String::from_utf8_lossy(&top).trim_end_matches('\n').to_string();
        if top.is_empty() {
            return Err(Error::NotARepository);
        }
        let toplevel = PathBuf::from(top).canonicalize()?;
        let git = base.at(toplevel.clone());

        let abs = normalize(&cwd.join(&cli.path));
        if abs.is_dir() {
            return Err(Error::IsADirectory(cli.path.clone()));
        }
        let resolved = canonicalize_existing_prefix(&abs);
        let rel = resolved
            .strip_prefix(&toplevel)
            .map_err(|_| Error::OutsideRepository(cli.path.clone()))?;
        if rel.as_os_str().is_empty() {
            return Err(Error::IsADirectory(cli.path.clone()));
        }
        let target = to_repo_path(rel);

        let filters = cli.log_filters();
        if let Some(range) = &filters.range {
            let ok = if range.contains("..") {
                git.probe(&["rev-list", "--max-count=1", range.as_str(), "--"])?.is_some()
            } else {
                git.probe(&["rev-parse", "--verify", "-q", &format!("{range}^{{commit}}")])?.is_some()
            };
            if !ok {
                return Err(Error::BadRevision(range.clone()));
            }
        }

        let query = LogQuery {
            path: target.clone(),
            follow: !cli.no_follow,
            line_range: cli.line_range.clone(),
            filters,
        };
        let show_local = query.filters.range.is_none();
        let head_born = git.probe(&["rev-parse", "--verify", "-q", "HEAD"])?.is_some();

        // Any history at all? (cheap: one commit, -L validated by git itself)
        let has_history = if head_born || query.filters.range.is_some() {
            let mut args = log::build_args(&query);
            args.insert(1, "--max-count=1".into());
            if query.line_range.is_some() {
                !git.output(&args)?.is_empty()
            } else {
                // without --follow: --follow is slow to start on big repos
                let args: Vec<OsString> = vec![
                    "log".into(),
                    "--max-count=1".into(),
                    "--format=%H".into(),
                ]
                .into_iter()
                .chain(query.filters.range.iter().map(OsString::from))
                .chain(["--".into(), target.as_literal_pathspec()])
                .collect();
                !git.output(&args)?.is_empty()
            }
        } else {
            false
        };

        let exists_locally = show_local
            && (target.to_fs_path(&toplevel).symlink_metadata().is_ok()
                || !git
                    .output(&["ls-files".into(), "-z".into(), "--".into(), target.as_literal_pathspec()])?
                    .is_empty());

        if !has_history && !exists_locally {
            return Err(Error::NoHistory(target));
        }

        Ok(RepoContext {
            git,
            toplevel,
            invocation_dir: cwd,
            target,
            query,
            show_local,
            git_version,
        })
    }
}

fn git_version(git: &Git) -> Result<(u32, u32, u32)> {
    let out = git.output(&["--version"])?;
    let s = String::from_utf8_lossy(&out);
    let v = s.trim().strip_prefix("git version ").ok_or(Error::Parse { context: "git --version" })?;
    let mut nums = v.split(|c: char| !c.is_ascii_digit()).map(|n| n.parse::<u32>().unwrap_or(0));
    Ok((nums.next().unwrap_or(0), nums.next().unwrap_or(0), nums.next().unwrap_or(0)))
}

/// Lexical normalization of `.` and `..` (the file may not exist).
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Canonicalize the deepest existing ancestor and re-append the rest.
fn canonicalize_existing_prefix(p: &Path) -> PathBuf {
    let mut existing = p.to_path_buf();
    let mut rest = Vec::new();
    loop {
        // The file itself may be a symlink: never resolve the last component.
        if !rest.is_empty() || existing.symlink_metadata().map(|m| !m.file_type().is_symlink()).unwrap_or(false) {
            if let Ok(c) = existing.canonicalize() {
                let mut out = c;
                for part in rest.iter().rev() {
                    out.push(part);
                }
                return out;
            }
        }
        match (existing.file_name().map(|s| s.to_os_string()), existing.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                existing = parent.to_path_buf();
            }
            _ => return p.to_path_buf(),
        }
    }
}

fn to_repo_path(rel: &Path) -> RepoPath {
    let mut bytes = Vec::new();
    for (i, c) in rel.components().enumerate() {
        if i > 0 {
            bytes.push(b'/');
        }
        bytes.extend_from_slice(c.as_os_str().as_encoded_bytes());
    }
    RepoPath::from_git_bytes(&bytes)
}
