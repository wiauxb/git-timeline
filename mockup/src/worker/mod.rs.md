# `src/worker/mod.rs`

## Purpose
Run everything slow (git processes, diffing, highlighting) **off the main thread**, so
the UI never freezes. Defines the job protocol between `app/` and the workers.

## Contents

```rust
pub struct WorkerPool {
    high: crossbeam_channel::Sender<Job>,   // user-visible work (current selection)
    low:  crossbeam_channel::Sender<Job>,   // prefetch of neighbours
    threads: Vec<std::thread::JoinHandle<()>>,
    cancel_log: Arc<AtomicBool>,
}

impl WorkerPool {
    /// n = clamp(available_parallelism - 1, 2, 4). Results go to `results`.
    pub fn spawn(ctx: Arc<RepoContext>, hl: Highlighter,
                 results: Sender<AppEvent>) -> Self;
    pub fn submit(&self, job: Job, prio: Priority);
    pub fn cancel_timeline(&self);            // used by reload
    pub fn shutdown(self);                    // drop senders, join threads
}

pub enum Priority { High, Low }

/// Monotonic id of "what the user is looking at". Bumped on every selection/view change.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ticket(pub u64);

pub enum Job {
    LoadTimeline,                                          // streams TimelineChunk
    LoadLocalState,
    LoadContent { ticket: Ticket, source: BlobSource, mode: FileMode },
    Highlight   { ticket: Ticket, key: ContentKey, content: Arc<FileContent>, path: RepoPath },
    ComputeDiff { ticket: Ticket, key: DiffKey, old: Arc<FileContent>, new: Arc<FileContent> },
    LoadBlame   { ticket: Ticket, key: BlameKey, entry: TimelineEntry, staged: Option<Arc<FileContent>> },
    LoadDetails { oid: Oid },
}

pub enum JobResult {
    TimelineChunk(Vec<TimelineEntry>),
    TimelineDone(Result<(), Arc<Error>>),
    LocalState(Result<Vec<TimelineEntry>, Arc<Error>>),
    Content   { ticket: Ticket, source: BlobSource, result: Result<(ContentKey, Arc<FileContent>), Arc<Error>> },
    Highlight { ticket: Ticket, key: ContentKey, syntax: String, result: HighlightOutcome },
    Diff      { ticket: Ticket, key: DiffKey, result: Arc<FileDiff> },
    Blame     { ticket: Ticket, key: BlameKey, result: Result<Arc<BlameData>, Arc<Error>> },
    Details   { oid: Oid, result: Result<Arc<CommitDetails>, Arc<Error>> },
}
```

## Scheduling
* Each thread loops on `crossbeam_channel::select_biased!{ recv(high), recv(low) }` ⇒
  high-priority jobs always go first.
* `LoadTimeline` is a long job; it runs on one worker while others serve content.
* **Staleness:** workers don't cancel content/diff/blame jobs (they're short or worth
  caching). Results are always cached by `update.rs`; the `ticket` only decides whether
  the result should *also* drive the view (e.g. re-anchor scroll). A queued **Low** job
  whose result is already cached is skipped by the main thread before submission
  (dedupe set `in_flight: HashSet<JobKey>` lives in `AppState`).
* Workers hold no shared mutable state except their own `BlobReader`.
* A panic inside a job is caught (`catch_unwind`) and reported as an error result, so a
  bad file can't kill a worker thread.
