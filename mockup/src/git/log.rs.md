# `src/git/log.rs`

## Purpose
Produce the **committed** part of the timeline: one `TimelineEntry` per commit that
touched the file, newest first, with the file's path, blob and previous blob at each
commit. Output is **streamed** in chunks.

## Contents

```rust
pub struct LogQuery {
    pub path: RepoPath,
    pub follow: bool,                 // default true; false with --no-follow
    pub line_range: Option<String>,   // "-L" value without ":path"
    pub filters: LogFilters,
}
pub struct LogFilters {
    pub range: Option<String>,
    pub authors: Vec<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

/// Runs git log and calls `sink` with batches of ≤ 256 entries.
/// Returns when git exits. Dropping/cancel ⇒ git is killed (GitStream::drop).
pub fn stream_timeline(git: &Git, q: &LogQuery,
                       cancel: &AtomicBool,
                       sink: &mut dyn FnMut(Vec<TimelineEntry>)) -> Result<()>;

pub(crate) fn build_args(q: &LogQuery) -> Vec<OsString>;

/// Incremental parser; fed arbitrary byte chunks; yields complete entries.
pub(crate) struct LogParser { buf: Vec<u8>, current: Option<PartialEntry> }
impl LogParser {
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<TimelineEntry>) -> Result<()>;
    pub fn finish(self, out: &mut Vec<TimelineEntry>) -> Result<()>;
}
```

## Command lines

Format `F` = `%x1e%H%x1f%P%x1f%an%x1f%ae%x1f%aI%x1f%cn%x1f%ce%x1f%cI%x1f%s`

Normal (follow):
```
git log --follow --raw --no-abbrev -z --format=F
        [--author=A]… [--since=S] [--until=U] [RANGE] -- :(literal)PATH
```
`--no-follow`: same without `--follow` (renames then end the history).

Line range:
```
git log -LSTART,END:PATH --no-patch --raw --no-abbrev -z --format=F
        [--author=A]… [--since=S] [--until=U] [RANGE]
```
(`-L` forbids a pathspec, and it tracks the lines across renames by itself. Verified:
pure-rename commits do **not** appear in `-L` output since they don't change the lines.)

## Output format and parsing (verified on git 2.55)

```
\x1e<H>\x1f<P>\x1f…\x1f<subject>\0
\n:100644 100644 <old-oid> <new-oid> M\0<path>\0
\x1e<H>…\0
\n:100644 100644 <old-oid> <new-oid> R100\0<old-path>\0<new-path>\0
```
Parser rules:
* Split on `\0` tokens. A token starting with `\x1e` (after trimming a leading `\n`)
  starts a new commit header; split it on `\x1f` into the 9 fields.
* A token starting with `:` is a raw record:
  `:<mode_old> <mode_new> <oid_old> <oid_new> <status>`; status letter `A M D T R C`
  (+ similarity digits for R/C). Then 1 path token (A/M/D/T) or 2 path tokens (R/C).
* With `--follow` there is at most one raw record per commit for our file; if there are
  several (should not happen), pick the one whose new path equals the currently tracked
  path.
* A commit with **no** raw record is a merge (git prints no raw output for merges
  without `-m`/`-c`) ⇒ `ChangeKind::Merge`, `BlobSource::Unresolved`.

## Path tracking
Commits come newest → oldest. Keep `tracked_path` (starts as `q.path`):
* entry `path` = new path of the raw record (= `tracked_path`),
* on `R`/`C`, `prev_path` = old path and `tracked_path` becomes the old path for older
  commits; otherwise `prev_path = path`.
* In `-L` mode there are **no `R` records** (rename-only commits are skipped and older
  commits simply report the old path — verified: `sub/g.txt` history lists `M f.txt`).
  So in `-L` mode, `path = prev_path =` the raw record's path, no tracking state.

## Mapping raw record → entry
| status | change | blob | prev_blob |
|---|---|---|---|
| `A` | Added | Object(new) | Absent |
| `M` | Modified | Object(new) | Object(old) |
| `D` | Deleted | Absent | Object(old) |
| `T` | TypeChanged | Object(new) | Object(old) |
| `Rnn` | Renamed{from, nn} | Object(new) | Object(old) (at `from`) |
| `Cnn` | Copied{from, nn} | Object(new) | Object(old) (at `from`) |
| none | Merge | Unresolved{H, path} | Unresolved{P[0], path} |

`mode` comes from `mode_new` (`000000` ⇒ Absent).

## Specs
* Ordering = git's default (reverse chronological by commit date, topology-aware with
  `--follow`). We don't re-sort.
* Streaming read buffer: 64 KiB; flush a batch to `sink` every 256 entries **or** every
  50 ms, so the first screen fills fast on huge repos.
* `cancel` is checked between reads; on cancel the stream is dropped (git killed).
* A non-zero exit after some entries were produced is reported as a non-fatal error
  (entries kept, status bar message).
