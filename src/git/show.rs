//! On-demand commit details for the details popup.

use super::Git;
use crate::error::Result;
use crate::model::{Oid, RepoPath};

const MAX_FILES: usize = 200;

#[derive(Debug, Clone)]
pub struct CommitDetails {
    pub oid: Oid,
    pub body: String,
    pub refs: Vec<String>,
    pub files: Vec<(char, RepoPath)>,
    pub files_truncated: bool,
}

pub fn commit_details(git: &Git, oid: &Oid) -> Result<CommitDetails> {
    let out = git.output(&["show", "-s", "--format=%B%x00%D", oid.as_str()])?;
    let text = String::from_utf8_lossy(&out);
    let (body, refs) = text.split_once('\0').unwrap_or((&text, ""));
    let refs = refs
        .trim()
        .split(", ")
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let out = git.output(&[
        "diff-tree", "--no-commit-id", "-r", "--name-status", "-z", "-M", "--root", oid.as_str(),
    ])?;
    let mut files = Vec::new();
    let mut tokens = out.split(|&b| b == 0).filter(|t| !t.is_empty());
    let mut truncated = false;
    while let Some(status) = tokens.next() {
        let letter = status[0] as char;
        let mut path = tokens.next().unwrap_or_default();
        if matches!(letter, 'R' | 'C') {
            path = tokens.next().unwrap_or(path);
        }
        if files.len() == MAX_FILES {
            truncated = true;
            break;
        }
        files.push((letter, RepoPath::from_git_bytes(path)));
    }

    Ok(CommitDetails {
        oid: oid.clone(),
        body: body.trim_end().to_string(),
        refs,
        files,
        files_truncated: truncated,
    })
}
