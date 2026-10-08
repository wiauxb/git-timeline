# `src/worker/jobs.rs`

## Purpose
Implementation of each `Job`: glue between the protocol in `worker/mod.rs` and the
`git/`, `model/`, `highlight` functions.

## Contents

```rust
pub(super) struct WorkerCtx {
    ctx: Arc<RepoContext>,
    blobs: BlobReader,             // own persistent `git cat-file --batch-command`
    hl: Highlighter,
    out: Sender<AppEvent>,
    timeline_gen: Arc<AtomicU64>,
}

impl WorkerCtx {
    pub fn new(ctx, hl, out, timeline_gen) -> Self;
    pub fn send(&self, r: JobResult);      // wraps in AppEvent::Job; send errors ignored
    pub fn reset(&mut self);               // fresh BlobReader (after a panic)
    pub fn run(&mut self, job: &Job);
    fn load_local_state(&self) -> Result<Vec<TimelineEntry>, String>;
    fn load_timeline(&self, gen: u64);
}
```

## Behaviors
| Job | Implementation |
|---|---|
| `LoadTimeline{gen}` | `git::log::stream_timeline`; each batch → `TimelineChunk{gen}` while `timeline_gen == gen`, otherwise sets the streamer's cancel flag (git is killed); always ends with `TimelineDone{gen, result}` |
| `LoadLocalState` | empty when `!ctx.show_local`; else `status::read_local_state` + `local_entries` |
| `LoadContent` | `blobs.read(source, mode)` (resolves `Unresolved` itself) → `(ContentKey, Arc<FileContent>)`; returns the original `source` so the app records `source → key` |
| `Highlight` | `hl.highlight(text, path)` for `Text`, else `Plain{"not text"}` |
| `ComputeDiff` | `model::diff::compute(old.as_text(), new.as_text())` (`None` = absent) |
| `LoadBlame` | `git::blame::blame(git, entry, staged bytes)` (staged bytes only for Index rows) |
| `LoadDetails` | `git::show::commit_details` |

All errors are converted with `to_string()` before crossing the channel.
