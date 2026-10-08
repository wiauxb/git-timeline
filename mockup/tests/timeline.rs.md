# `tests/timeline.rs`

## Purpose
Integration tests of history building (`git/repo.rs`, `git/log.rs`, `git/status.rs`)
against real fixture repositories.

## Test cases

| Test | Scenario | Asserts |
|---|---|---|
| `linear` | 5 commits editing `a.txt` | 5 entries, newest first; first is `Added` with `prev_blob = Absent`; others `Modified` with `prev_blob == blob` of the next entry |
| `follow_rename` | edit, `git mv a.txt dir/b.txt`, edit | queried as `dir/b.txt`: all commits present; rename row `Renamed{from:"a.txt"}`; older rows have `path == "a.txt"` |
| `no_follow` | same, `--no-follow` | history stops at the rename commit (`Added` there) |
| `line_range` | edits inside and outside lines 2-3 | only commits touching those lines; no rename rows; paths per raw record |
| `filters` | two authors | `--author` keeps only one author's commits; `prev_blob` still the real parent version (not the previous filtered row) |
| `range` | `HEAD~2..HEAD` | 2 entries, no pseudo-entries |
| `merge` | file edited on both branches then merged | merge row has `ChangeKind::Merge` and `Unresolved` blobs that resolve to the merge result / first-parent version |
| `deleted_file` | add, edit, delete; query deleted path | discover succeeds; top commit row is `Deleted` with `blob = Absent` |
| `pseudo_worktree` | modify file on disk | `WorkingTree` row first, `prev_blob` = index blob |
| `pseudo_staged_and_worktree` | stage one change, make another | rows `WorkingTree`, `Index`, then commits |
| `untracked_only` | new untracked file | only a `WorkingTree` row with `Untracked` |
| `unborn_head` | repo with no commit, file staged | only an `Index` row |
| `conflict` | merge conflict on file | `WorkingTree` row with `conflicted = true`, no `Index` row |
| `weird_names` | file named `a*b [x].txt`, non-UTF-8 name (Unix) | pathspec is literal, paths round-trip bytes |
| `subdir_invocation` | cwd = `repo/sub`, arg `../a.txt` | `target == "a.txt"` |
| `symlinked_cwd` | cwd reached through a symlink | path resolved correctly |
| `errors` | outside repo, directory, unknown path, bad range, `-L` beyond EOF | the matching `Error` variant, before any TUI |
| `streaming` | 1 000 commits | parser yields several chunks; total = 1 000; feeding the parser byte-by-byte gives the same result |
