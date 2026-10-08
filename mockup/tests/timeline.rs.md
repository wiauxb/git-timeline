# `tests/timeline.rs`

## Purpose
Integration tests of history building (`git/repo.rs`, `git/log.rs`, `git/status.rs`)
against real fixture repositories.

## Helpers
`commits(ctx)` (runs `stream_timeline` to completion), `locals(ctx)`
(`read_local_state` + `local_entries`), `oid_of(entry)`.

## Tests

| Test | Scenario | Asserts |
|---|---|---|
| `linear` | 5 edits of `a.txt` | 5 entries newest first; oldest `Added` with `prev_blob = Absent`; others `Modified` with `prev_blob ==` next entry's `blob`; no pseudo-entries |
| `follow_rename` | `renamed_history`, queried as `dir/b.txt` | 4 entries; rename row `Renamed { from: "a.txt" }`; paths `dir/b.txt` above, `a.txt` below |
| `no_follow` | same, `--no-follow` | 2 entries, oldest `Added` |
| `line_range` | edits on lines 2 and 5, then rename | `-L 2,3 b.txt` → 2 entries, newest is the line-2 commit, path `a.txt` |
| `author_filter_keeps_real_parent` | Alice / Bob / Alice | `--author=Alice` → 2 entries; newest `prev_blob` is Bob's version (`HEAD~1:a.txt`) |
| `range` | `HEAD~2..HEAD` | 2 entries, `show_local == false` |
| `merge` | edits on two branches + `--no-ff` merge | 4 entries; newest is `Merge` with `blob = HEAD:a.txt`, `prev_blob = HEAD^1:a.txt` |
| `deleted_file` | add then delete | newest `Deleted`, `blob = Absent` |
| `pseudo_entries` | staged change + different worktree change | `WorkingTree` then `Index`; worktree `prev_blob ==` index `blob`; worktree blob is `WorkingTree(_)` |
| `untracked_and_unborn` | new file, no commits | untracked → one `WorkingTree` row `Untracked`; once staged → one `Index` row `Added` |
| `weird_names_and_subdir` | `sub/a*b [x].txt` + `sub/other.txt`; run from `sub` | literal pathspec (1 commit, not globbed); `../top.txt` from `sub` → `top.txt` |
| `errors` | directory, unknown file, bad rev, outside repo, `-L 50,60` past EOF | `IsADirectory`, `NoHistory`, `BadRevision`, `OutsideRepository`, `Git` |
| `streaming_many_commits` | 300 commits via `git fast-import` | total 300 entries through the sink |

Parser-level streaming (byte-by-byte feeding) is covered by unit tests in `src/git/log.rs`.
