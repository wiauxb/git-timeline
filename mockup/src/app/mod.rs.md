# `src/app/mod.rs`

## Purpose
The application loop: events → actions → reducer → effects → redraw. Owns the
`WorkerPool`, the event channel and (once running) the input thread.

## Contents

```rust
pub mod action; pub mod cache; pub mod keymap; pub mod search; pub mod state; pub mod update;

const TICK: Duration = Duration::from_millis(120);

pub enum AppEvent {
    Input(ratatui::crossterm::event::Event),
    Job(worker::JobResult),
}

pub struct App {
    state: AppState,
    workers: WorkerPool,
    events: Receiver<AppEvent>,
    input_tx: Option<Sender<AppEvent>>,   // handed to the input thread in `run`
}

impl App {
    pub fn new(ctx: RepoContext, ui: UiOptions, hl: Highlighter) -> Self;
    /// Returns Ok(Some(msg)) on a fatal error (printed by main), Ok(None) on normal quit.
    pub fn run(self, term: &mut Terminal) -> Result<Option<String>>;
    fn execute(&mut self, effects: Vec<Effect>) -> bool;   // true = quit
}
```

## `App::new`
1. Wrap `ctx` in `Arc`, create the unbounded `AppEvent` channel, build `AppState`.
2. Highlighter theme warning (unknown theme) → `Warn` status message.
3. **Local state is read synchronously** (`git/status.rs`, a few ms) when
   `ctx.show_local`: the `Working tree` / `Staged` rows are on top before the first commits
   stream in, and index 0 is selected. Failure → `Error` status message, not fatal.
4. Spawn the `WorkerPool` (shares the event sender).
   **No input thread yet** — see below.

## `App::run`
1. Spawn the input thread: `loop { crossterm::event::read() → send(AppEvent::Input) }`,
   exits when the send fails. It is started **here, after the terminal is set up**:
   when it was started earlier (in `new`), its blocking `read()` swallowed the
   terminal's reply to ratatui's cursor-position query and startup failed with
   "The cursor position could not be read within a normal duration".
2. Submit `Job::LoadTimeline { gen: state.timeline_gen }` (High).
3. Loop:
   ```
   effects = update::after_batch(&mut state)      // turn needs into jobs, re-anchor
   execute(effects) → quit?            → break
   state.fatal.is_some()               → break
   if state.dirty:
       dirty = false; layout = ui::draw(frame, &state)
       if layout != state.layout → update(Action::Layout(layout)); continue (redraw)
   recv_timeout(TICK):
       timeout      → update(Action::Tick); continue
       disconnected → break
       event        → pending.push(ev)
   pending += try_iter().take(512)               // coalesce bursts (key repeat, chunks)
   for ev in pending: Input → keymap::map (0/1 action), Job → Action::JobDone
                      update(action); execute(effects); stop on quit
   ```
   Because `after_batch` runs once per drained batch, a held `j` produces job requests only
   for the selection reached at the end of the batch (debounce).
4. `workers.shutdown()`, return `state.fatal.take()`.

## Effect execution (`execute`)
| Effect | Done by | Follow-up |
|---|---|---|
| `Quit` | return `true` immediately | – |
| `Submit(job, prio)` | `workers.submit` | – |
| `Copy(text)` | `ops::clipboard::copy` | `OpResult::Copied{text, method}` / `Failed{op:"copy"}` |
| `Export{bytes, dest, overwrite}` | `ops::export::write` (sync) | its `OpResult` |
| `Restore{bytes, mode, target}` | `ops::restore::restore` (sync) | its `OpResult` |

Each `OpResult` is fed back as `Action::OpDone` through `update`; resulting effects are
executed in turn, up to a depth of 8.

## Fatal path
`update` sets `state.fatal` (timeline failed or empty with no rows at all); the loop breaks,
`run` returns `Ok(Some(msg))`, `lib::run` restores the terminal and returns it as an error,
`main` prints `git-timeline: <msg>` and exits 1.
