# `src/worker/mod.rs`

## Purpose
Run everything slow (git processes, diffing, highlighting) **off the main thread**.
Defines the job protocol between `app/` and the worker threads.

## Contents

```rust
mod jobs;

pub enum Priority { High, Low }

/// Highlight cache key: the file name decides the syntax (renames can change it).
#[derive(Clone, Hash, Eq)]
pub struct HighlightKey { pub content: ContentKey, pub name: Vec<u8> }

pub enum Job {
    LoadTimeline { gen: u64 },                       // streams TimelineChunk
    LoadLocalState,
    LoadContent { source: BlobSource, mode: FileMode },
    Highlight   { key: HighlightKey, content: Arc<FileContent>, path: RepoPath },
    ComputeDiff { key: DiffKey, old: Arc<FileContent>, new: Arc<FileContent> },
    LoadBlame   { key: BlameKey, entry: TimelineEntry, staged: Option<Arc<FileContent>> },
    LoadDetails { oid: Oid },
}

pub enum JobResult {                                 // errors cross threads as String
    TimelineChunk { gen: u64, entries: Vec<TimelineEntry> },
    TimelineDone  { gen: u64, result: Result<(), String> },
    LocalState(Result<Vec<TimelineEntry>, String>),
    Content   { source: BlobSource, result: Result<(ContentKey, Arc<FileContent>), String> },
    Highlight { key: HighlightKey, result: HighlightOutcome },
    Diff      { key: DiffKey, diff: Arc<FileDiff> },
    Blame     { key: BlameKey, result: Result<Arc<BlameData>, String> },
    Details   { oid: Oid, result: Result<Arc<CommitDetails>, String> },
}

impl Job { fn failure(&self, msg: String) -> JobResult; }   // result reported on panic

pub struct WorkerPool {
    high: Option<Sender<Job>>, low: Option<Sender<Job>>,
    threads: Vec<JoinHandle<()>>,
    timeline_gen: Arc<AtomicU64>,
}
impl WorkerPool {
    /// n = clamp(available_parallelism - 1, 2, 4) threads named "worker-<i>".
    pub fn spawn(ctx: Arc<RepoContext>, hl: Highlighter, results: Sender<AppEvent>) -> Self;
    pub fn submit(&self, job: Job, prio: Priority);
    pub fn shutdown(self);
}

#[doc(hidden)]   // tests: run one job synchronously, collect its results
pub fn run_inline(ctx: Arc<RepoContext>, hl: Highlighter, job: Job) -> Vec<JobResult>;
```

## Scheduling
* Each thread loops on `select_biased!{ recv(high), recv(low) }`: high-priority jobs
  always first; the thread exits when its channels disconnect.
* **No per-job tickets.** Results are always sent and cached by the app, which re-derives
  what it still needs (see `app/update.rs`). Dedupe of submissions is done by the app's
  `in_flight` set.
* **Timeline generation:** `submit(LoadTimeline{gen})` stores `gen` in `timeline_gen`; a
  running timeline job compares it at each chunk and stops (killing git) when outdated.
  Results carry `gen`; the app ignores stale ones.
* **Panics:** each job runs under `catch_unwind`; on panic the thread sends
  `job.failure("internal error: …")` (so the app never waits forever on an `in_flight` key)
  and resets its `BlobReader` (the cat-file process may be mid-response). Failure of
  `ComputeDiff` is an empty `FileDiff`; of `Highlight`, `Plain{"highlighting failed"}`.
  The terminal panic hook ignores `worker-*` threads, so a caught panic does not tear the
  UI down.
* **Shutdown:** bump `timeline_gen` (stops a running log), drop both senders, detach the
  threads (quitting does not wait for a slow git command).
* `run_inline` creates a private channel and `WorkerCtx`, runs the job on the calling
  thread, and returns all `JobResult`s sent (used by `tests/views.rs`).
