# `src/git/log.rs`

## Purpose
Produce the **committed** part of the timeline: one `TimelineEntry` per commit that touched
the file, newest first, with path, blob, previous blob and modes at each commit.
Output is **streamed** in chunks.

## Contents

```rust
const FORMAT: &str = "--format=%x1e%H%x1f%P%x1f%an%x1f%ae%x1f%aI%x1f%cn%x1f%ce%x1f%cI%x1f%s";
const BATCH: usize = 256;
const FLUSH_EVERY: Duration = Duration::from_millis(50);

#[derive(Clone, Debug)]
pub struct LogQuery {
    pub path: RepoPath,
    pub follow: bool,                 // false with --no-follow
    pub line_range: Option<String>,   // "-L" value without ":path"
    pub filters: LogFilters,
}
#[derive(Clone, Debug, Default)]
pub struct LogFilters {
    pub range: Option<String>,
    pub authors: Vec<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

pub fn build_args(q: &LogQuery) -> Vec<OsString>;

/// Runs git log; calls `sink` with batches (≥ BATCH entries, or every FLUSH_EVERY).
/// `cancel` is checked between reads; returning early drops the stream (kills git).
pub fn stream_timeline(git: &Git, q: &LogQuery, cancel: &AtomicBool,
                       sink: &mut dyn FnMut(Vec<TimelineEntry>)) -> Result<()>;

/// Incremental parser, fed arbitrary byte chunks.
pub struct LogParser { buf, current: Option<PartialEntry>, pending_raw: Option<RawRecord>,
                       tracked_path: RepoPath, line_mode: bool }
impl LogParser {
    pub fn new(q: &LogQuery) -> Self;
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<TimelineEntry>) -> Result<()>;
    pub fn finish(self, out: &mut Vec<TimelineEntry>) -> Result<()>;
}
```

## Command lines

Normal:
```
git log [--follow] --diff-merges=first-parent --raw --no-abbrev -z --format=F
        [--author=A]… [--since=S] [--until=U] [RANGE] -- :(literal)PATH
```
Line range:
```
git log -LSTART,END:PATH --no-patch --raw --no-abbrev -z --format=F
        [--author=A]… [--since=S] [--until=U] [RANGE]
```

Verified git facts (2.55) behind these choices:
* **`--follow` silently drops merge commits**, even merges that produce a new version of the
  file (the newest row would then not match HEAD). `--diff-merges=first-parent` makes git
  list them with one raw record against the first parent. Without `--follow`, git already
  shows merges but with no raw record; the flag gives them one too.
* `-L` forbids a pathspec and tracks the lines across renames itself, but it emits **no `R`
  records** and **skips rename-only commits**; older commits just report the old path.
  `-L` gets neither `--follow` nor `--diff-merges`.
* `--author=…` etc. use the `=` form so values starting with `-` aren't parsed as options.

## Output format and parsing
```
\x1e<H>\x1f<P>\x1f…\x1f<subject>\0
\n:100644 100644 <old-oid> <new-oid> M\0<path>\0
\x1e<H>…\0
\n:100644 100644 <old-oid> <new-oid> R100\0<old-path>\0<new-path>\0
```
* Tokens are split on `\0`. If a raw record is waiting for paths, the token is a path
  (1 path, 2 for `R`/`C`). Otherwise leading `\n`s are trimmed: `\x1e…` starts a commit
  (flushing the previous one; header split on `\x1f` into exactly 9 fields, dates parsed as
  RFC 3339), `:…` starts a raw record, empty is skipped, anything else ⇒ `Error::Parse`.
* Raw record: `<mode_old> <mode_new> <oid_old> <oid_new> <status>[similarity]`; null oids ⇒
  `None` ⇒ `BlobSource::Absent`.
* Several raw records for one commit (not expected): pick the one whose new path equals the
  tracked path, else the first.

## Path tracking
Commits arrive newest → oldest; `tracked_path` starts as `q.path`. With a raw record:
`path` = new path, `prev_path` = first path (old path for R/C). Outside `-L` mode,
`tracked_path = prev_path` afterwards, so older commits are matched under the old name.
In `-L` mode there is no tracking state.

## Mapping raw record → entry
| status | change | blob | prev_blob |
|---|---|---|---|
| any, commit has > 1 parent | Merge | Object(new) | Object(old) (first parent) |
| `A` | Added | Object(new) | Absent |
| `M` / other | Modified | Object(new) | Object(old) |
| `D` | Deleted | Absent | Object(old) |
| `T` | TypeChanged | Object(new) | Object(old) |
| `Rnn` / `Cnn` | Renamed / Copied {from, nn} | Object(new) | Object(old) at `from` |
| no raw record (fallback) | Merge if > 1 parent else Modified | Unresolved{H, tracked} | Unresolved{P[0], tracked} or Absent |

`mode` / `prev_mode` come from the raw record (`Regular` in the fallback).

## Specs
* Order is git's default; never re-sorted.
* Read buffer 64 KiB.
* A non-zero exit is returned by `stream_timeline` after all parsed entries were sunk; the
  app treats it as fatal only if the timeline is empty.

## Tests
Rename history parsing, byte-by-byte feeding gives the same result, merge without raw record.
Integration (tests/timeline.rs): linear, follow, no-follow, `-L` across a rename, author
filter keeps the real parent blob, range, merge row = HEAD blob vs first parent, deleted
file, 300-commit streaming.
