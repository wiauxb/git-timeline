//! Everything slow (git processes, diffing, highlighting) runs here, off the main thread.

mod jobs;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{select_biased, unbounded, Receiver, Sender};

use crate::app::AppEvent;
use crate::git::repo::RepoContext;
use crate::git::show::CommitDetails;
use crate::highlight::{HighlightOutcome, Highlighter};
use crate::model::{
    BlameData, BlameKey, BlobSource, ContentKey, DiffKey, FileContent, FileDiff, FileMode, Oid,
    RepoPath, TimelineEntry,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    High,
    Low,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct HighlightKey {
    pub content: ContentKey,
    /// File name decides the syntax (renames can change the extension).
    pub name: Vec<u8>,
}

#[derive(Debug)]
pub enum Job {
    LoadTimeline { gen: u64 },
    LoadLocalState,
    LoadContent { source: BlobSource, mode: FileMode },
    Highlight { key: HighlightKey, content: Arc<FileContent>, path: RepoPath },
    ComputeDiff { key: DiffKey, old: Arc<FileContent>, new: Arc<FileContent> },
    LoadBlame { key: BlameKey, entry: TimelineEntry, staged: Option<Arc<FileContent>> },
    LoadDetails { oid: Oid },
}

#[derive(Debug)]
pub enum JobResult {
    TimelineChunk { gen: u64, entries: Vec<TimelineEntry> },
    TimelineDone { gen: u64, result: Result<(), String> },
    LocalState(Result<Vec<TimelineEntry>, String>),
    Content { source: BlobSource, result: Result<(ContentKey, Arc<FileContent>), String> },
    Highlight { key: HighlightKey, result: HighlightOutcome },
    Diff { key: DiffKey, diff: Arc<FileDiff> },
    Blame { key: BlameKey, result: Result<Arc<BlameData>, String> },
    Details { oid: Oid, result: Result<Arc<CommitDetails>, String> },
}

impl Job {
    /// Result reported if the job panics, so the app never waits forever.
    fn failure(&self, msg: String) -> JobResult {
        match self {
            Job::LoadTimeline { gen } => JobResult::TimelineDone { gen: *gen, result: Err(msg) },
            Job::LoadLocalState => JobResult::LocalState(Err(msg)),
            Job::LoadContent { source, .. } => JobResult::Content { source: source.clone(), result: Err(msg) },
            Job::Highlight { key, .. } => {
                JobResult::Highlight { key: key.clone(), result: HighlightOutcome::Plain { reason: "highlighting failed" } }
            }
            Job::ComputeDiff { key, .. } => JobResult::Diff { key: key.clone(), diff: Arc::default() },
            Job::LoadBlame { key, .. } => JobResult::Blame { key: key.clone(), result: Err(msg) },
            Job::LoadDetails { oid } => JobResult::Details { oid: oid.clone(), result: Err(msg) },
        }
    }
}

pub struct WorkerPool {
    high: Option<Sender<Job>>,
    low: Option<Sender<Job>>,
    threads: Vec<JoinHandle<()>>,
    timeline_gen: Arc<AtomicU64>,
}

impl WorkerPool {
    pub fn spawn(ctx: Arc<RepoContext>, hl: Highlighter, results: Sender<AppEvent>) -> Self {
        let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).saturating_sub(1).clamp(2, 4);
        let (high_tx, high_rx) = unbounded::<Job>();
        let (low_tx, low_rx) = unbounded::<Job>();
        let timeline_gen = Arc::new(AtomicU64::new(0));
        let threads = (0..n)
            .map(|i| {
                let (high_rx, low_rx) = (high_rx.clone(), low_rx.clone());
                let mut w = jobs::WorkerCtx::new(ctx.clone(), hl.clone(), results.clone(), timeline_gen.clone());
                std::thread::Builder::new()
                    .name(format!("worker-{i}"))
                    .spawn(move || worker_loop(&mut w, high_rx, low_rx))
                    .expect("spawn worker thread")
            })
            .collect();
        WorkerPool { high: Some(high_tx), low: Some(low_tx), threads, timeline_gen }
    }

    pub fn submit(&self, job: Job, prio: Priority) {
        if let Job::LoadTimeline { gen } = job {
            // any older timeline job stops at its next read
            self.timeline_gen.store(gen, Ordering::SeqCst);
        }
        let tx = match prio {
            Priority::High => &self.high,
            Priority::Low => &self.low,
        };
        if let Some(tx) = tx {
            let _ = tx.send(job);
        }
    }

    pub fn shutdown(mut self) {
        self.timeline_gen.fetch_add(1, Ordering::SeqCst);
        self.high.take();
        self.low.take();
        // Threads finish their current job; don't block quitting on a slow git command.
        self.threads.clear();
    }
}

fn worker_loop(w: &mut jobs::WorkerCtx, high: Receiver<Job>, low: Receiver<Job>) {
    loop {
        let job = select_biased! {
            recv(high) -> j => match j { Ok(j) => j, Err(_) => return },
            recv(low) -> j => match j { Ok(j) => j, Err(_) => return },
        };
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.run(&job)));
        if let Err(panic) = caught {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "worker panicked".into());
            w.send(job.failure(format!("internal error: {msg}")));
            w.reset();
        }
    }
}

/// Run one job synchronously and collect its results (tests, no threads).
#[doc(hidden)]
pub fn run_inline(ctx: Arc<RepoContext>, hl: Highlighter, job: Job) -> Vec<JobResult> {
    let (tx, rx) = unbounded();
    let gen = Arc::new(AtomicU64::new(match job {
        Job::LoadTimeline { gen } => gen,
        _ => 0,
    }));
    jobs::WorkerCtx::new(ctx, hl, tx, gen).run(&job);
    rx.try_iter()
        .filter_map(|e| match e {
            AppEvent::Job(r) => Some(r),
            AppEvent::Input(_) => None,
        })
        .collect()
}
