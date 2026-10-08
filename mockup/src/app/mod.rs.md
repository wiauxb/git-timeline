# `src/app/mod.rs`

## Purpose
The application loop: receive events, translate them into actions, run the reducer,
execute the resulting effects, redraw. Owns the `WorkerPool` and the input thread.

## Contents

```rust
pub mod state; pub mod action; pub mod update; pub mod keymap; pub mod cache;

pub enum AppEvent {
    Input(crossterm::event::Event),
    Job(worker::JobResult),
    Tick,                         // 250 ms: expire status messages, spinner animation
}

pub struct App {
    state: state::AppState,
    workers: worker::WorkerPool,
    events: crossbeam_channel::Receiver<AppEvent>,
    _input: std::thread::JoinHandle<()>,
}

impl App {
    pub fn new(ctx: RepoContext, ui: UiOptions, hl: Highlighter) -> Self;
    pub fn run(self, term: &mut Terminal) -> Result<()>;
    fn execute(&mut self, effects: Vec<Effect>) -> ControlFlow<()>;
}
```

## Loop

```
submit LoadLocalState, LoadTimeline
draw
loop {
    ev = events.recv_timeout(250ms) → Tick on timeout
    drain all immediately available events (coalesce: many TimelineChunks / key repeats
      are processed before a single redraw)
    for each ev:
        actions = match ev {
            Input(e) => keymap::map(&state, e) (0 or 1 action),
            Job(r)   => [Action::JobDone(r)],
            Tick     => [Action::Tick],
        }
        effects += update::update(&mut state, action)
    if execute(effects) is Break → break
    if state.dirty → layout = ui::draw(frame, &state); state.dirty = false;
                     if layout != state.layout → update(Action::Layout(layout))
}
workers.shutdown()
```

## Input thread
`loop { crossterm::event::read() → tx.send(AppEvent::Input) }`; it exits when the send
fails. (Blocking `read` is fine on its own thread; no polling needed.)

## Effect execution (`execute`)
| Effect | Done by |
|---|---|
| `Submit(job, prio)` | `workers.submit` |
| `CancelTimeline` | `workers.cancel_timeline` |
| `Copy(text)` | `ops::clipboard::copy` → result fed back as `Action::OpDone` |
| `Export{bytes, dest, overwrite}` | `ops::export::write` (synchronous, small) → `Action::OpDone` |
| `Restore{...}` | `ops::restore::restore` → `Action::OpDone`, then reload local state |
| `Quit` | break |

Effects that produce follow-up actions call `update` again immediately (bounded depth).

## Notes
* The first draw happens before any data arrives (empty list with "loading…").
* If `TimelineDone(Err)` arrives with 0 entries and no pseudo-entries, the loop quits
  and `run` returns that error (fatal, printed by `main`).
