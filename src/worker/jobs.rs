use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crossbeam_channel::Sender;

use super::{Job, JobResult};
use crate::app::AppEvent;
use crate::git::blob::BlobReader;
use crate::git::repo::RepoContext;
use crate::git::{blame, log, show, status};
use crate::highlight::{HighlightOutcome, Highlighter};
use crate::model::{diff, FileContent};

pub(super) struct WorkerCtx {
    ctx: Arc<RepoContext>,
    blobs: BlobReader,
    hl: Highlighter,
    out: Sender<AppEvent>,
    timeline_gen: Arc<AtomicU64>,
}

impl WorkerCtx {
    pub fn new(ctx: Arc<RepoContext>, hl: Highlighter, out: Sender<AppEvent>, timeline_gen: Arc<AtomicU64>) -> Self {
        let blobs = BlobReader::new(ctx.git.clone());
        WorkerCtx { ctx, blobs, hl, out, timeline_gen }
    }

    pub fn send(&self, r: JobResult) {
        let _ = self.out.send(AppEvent::Job(r));
    }

    /// After a panic: drop the cat-file process, it may be mid-response.
    pub fn reset(&mut self) {
        self.blobs = BlobReader::new(self.ctx.git.clone());
    }

    pub fn run(&mut self, job: &Job) {
        let result = match job {
            Job::LoadTimeline { gen } => return self.load_timeline(*gen),
            Job::LoadLocalState => JobResult::LocalState(self.load_local_state()),
            Job::LoadContent { source, mode } => JobResult::Content {
                source: source.clone(),
                result: self
                    .blobs
                    .read(source, *mode)
                    .map(|(k, c)| (k, Arc::new(c)))
                    .map_err(|e| e.to_string()),
            },
            Job::Highlight { key, content, path } => JobResult::Highlight {
                key: key.clone(),
                result: match content.as_text() {
                    Some(t) => self.hl.highlight(t, path),
                    None => HighlightOutcome::Plain { reason: "not text" },
                },
            },
            Job::ComputeDiff { key, old, new } => JobResult::Diff {
                key: key.clone(),
                diff: Arc::new(diff::compute(old.as_text(), new.as_text())),
            },
            Job::LoadBlame { key, entry, staged } => {
                let staged_bytes = staged.as_deref().and_then(FileContent::bytes);
                JobResult::Blame {
                    key: key.clone(),
                    result: blame::blame(&self.ctx.git, entry, staged_bytes.as_deref())
                        .map(Arc::new)
                        .map_err(|e| e.to_string()),
                }
            }
            Job::LoadDetails { oid } => JobResult::Details {
                oid: oid.clone(),
                result: show::commit_details(&self.ctx.git, oid).map(Arc::new).map_err(|e| e.to_string()),
            },
        };
        self.send(result);
    }

    fn load_local_state(&self) -> Result<Vec<crate::model::TimelineEntry>, String> {
        if !self.ctx.show_local {
            return Ok(Vec::new());
        }
        status::read_local_state(&self.ctx.git, &self.ctx.target)
            .map(|st| status::local_entries(&st, &self.ctx.target))
            .map_err(|e| e.to_string())
    }

    fn load_timeline(&self, gen: u64) {
        // Bridge the generation counter to the AtomicBool the log streamer polls.
        let cancel = AtomicBool::new(false);
        let current = || self.timeline_gen.load(Ordering::SeqCst) == gen;
        let result = log::stream_timeline(&self.ctx.git, &self.ctx.query, &cancel, &mut |entries| {
            if !current() {
                cancel.store(true, Ordering::Relaxed);
                return;
            }
            self.send(JobResult::TimelineChunk { gen, entries });
        });
        self.send(JobResult::TimelineDone { gen, result: result.map_err(|e| e.to_string()) });
    }
}
