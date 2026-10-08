# `src/worker/jobs.rs`

## Purpose
Implementation of each `Job` variant: the glue between the protocol in `worker/mod.rs`
and the `git/`, `model/`, `highlight` functions. One function per job.

## Contents

```rust
pub(super) struct WorkerCtx {
    pub ctx: Arc<RepoContext>,
    pub blobs: BlobReader,           // owned per thread (own cat-file process)
    pub hl: Highlighter,
    pub out: Sender<AppEvent>,
    pub cancel_log: Arc<AtomicBool>,
}

pub(super) fn run(w: &mut WorkerCtx, job: Job);

fn load_timeline(w: &mut WorkerCtx);
fn load_local_state(w: &mut WorkerCtx);
fn load_content(w: &mut WorkerCtx, ticket: Ticket, src: BlobSource, mode: FileMode);
fn highlight(w: &WorkerCtx, ticket: Ticket, key: ContentKey, c: Arc<FileContent>, path: RepoPath);
fn compute_diff(w: &WorkerCtx, ticket: Ticket, key: DiffKey, old: Arc<FileContent>, new: Arc<FileContent>);
fn load_blame(w: &mut WorkerCtx, ticket: Ticket, key: BlameKey, e: TimelineEntry, staged: Option<Arc<FileContent>>);
fn load_details(w: &mut WorkerCtx, oid: Oid);
```

## Behaviors
* `load_timeline`: calls `git::log::stream_timeline` with a sink that sends
  `JobResult::TimelineChunk` per batch; then `TimelineDone`. Resets `cancel_log` first.
* `load_local_state`: `status::read_local_state` + `status::local_entries`; skipped
  (returns empty) when `!ctx.show_local`.
* `load_content`: `Unresolved` sources are resolved first, then `blobs.read(...)`.
  The result includes the original `source` so the main thread can map
  `BlobSource → ContentKey` (stored in `cache.source_keys`).
* `highlight`: only for `FileContent::Text`; computes syntax via
  `hl.syntax_for(path, first_line)` then `hl.highlight`.
* `compute_diff`: `model::diff::compute` on the two `Text` contents (`None` for Absent);
  non-text pairs produce an empty `FileDiff` and the view uses its summary path.
* `load_blame`: `git::blame::blame(...)`; for `Index` rows passes the staged bytes.
* Every send ignores `SendError` (main thread is shutting down).
