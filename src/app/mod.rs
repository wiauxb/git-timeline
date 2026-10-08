//! The application loop: events → actions → reducer → effects → redraw.

pub mod action;
pub mod cache;
pub mod keymap;
pub mod search;
pub mod state;
pub mod update;
pub mod viewport;

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError, Sender};

use crate::cli::UiOptions;
use crate::error::Result;
use crate::git::repo::RepoContext;
use crate::git::status;
use crate::highlight::Highlighter;
use crate::ops;
use crate::terminal::Terminal;
use crate::worker::{Job, JobResult, Priority, WorkerPool};
use action::{Action, Effect, OpResult};
use state::{AppState, Level};

const TICK: Duration = Duration::from_millis(120);

#[derive(Debug)]
pub enum AppEvent {
    Input(ratatui::crossterm::event::Event),
    Job(JobResult),
}

pub struct App {
    state: AppState,
    workers: WorkerPool,
    events: Receiver<AppEvent>,
    input_tx: Option<Sender<AppEvent>>,
}

impl App {
    pub fn new(ctx: RepoContext, ui: UiOptions, hl: Highlighter) -> Self {
        let ctx = Arc::new(ctx);
        let (tx, rx) = unbounded();
        let mut state = AppState::new(ctx.clone(), ui, hl.enabled());
        if let Some(w) = &hl.theme_warning {
            update::info(&mut state, Level::Warn, w.clone());
        }

        // Local state first and synchronously (a few ms), so the working-tree row is
        // already on top when the first commits arrive and gets selected by default.
        if ctx.show_local {
            match status::read_local_state(&ctx.git, &ctx.target) {
                Ok(st) => state.timeline.set_local(status::local_entries(&st, &ctx.target)),
                Err(e) => update::info(&mut state, Level::Error, format!("git status failed: {e}")),
            }
            if !state.timeline.is_empty() {
                update::select(&mut state, 0);
            }
        }

        let workers = WorkerPool::spawn(ctx, hl, tx.clone());
        App { state, workers, events: rx, input_tx: Some(tx) }
    }

    /// Call only once the terminal is set up: the input thread must not race the
    /// terminal's own queries (cursor position) during setup.
    pub fn run(mut self, term: &mut Terminal) -> Result<Option<String>> {
        if let Some(input_tx) = self.input_tx.take() {
            std::thread::Builder::new()
                .name("input".into())
                .spawn(move || {
                    while let Ok(ev) = ratatui::crossterm::event::read() {
                        if input_tx.send(AppEvent::Input(ev)).is_err() {
                            break;
                        }
                    }
                })
                .expect("spawn input thread");
        }
        self.workers.submit(Job::LoadTimeline { gen: self.state.timeline_gen }, Priority::High);
        let mut pending: Vec<AppEvent> = Vec::new();
        loop {
            // flush requests & redraw
            let fx = update::after_batch(&mut self.state);
            if self.execute(fx) {
                break;
            }
            if self.state.fatal.is_some() {
                break;
            }
            if self.state.dirty {
                self.state.dirty = false;
                let state = &self.state;
                let mut layout = state.layout;
                term.draw(|f| layout = crate::ui::draw(f, state))?;
                if layout != self.state.layout {
                    let fx = update::update(&mut self.state, Action::Layout(layout));
                    if self.execute(fx) {
                        break;
                    }
                    continue;
                }
            }

            // wait for at least one event, then drain everything available (coalescing)
            match self.events.recv_timeout(TICK) {
                Ok(ev) => pending.push(ev),
                Err(RecvTimeoutError::Timeout) => {
                    let fx = update::update(&mut self.state, Action::Tick);
                    if self.execute(fx) {
                        break;
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
            pending.extend(self.events.try_iter().take(512));
            let mut quit = false;
            for ev in pending.drain(..) {
                let action = match ev {
                    AppEvent::Input(e) => keymap::map(&self.state, e),
                    AppEvent::Job(r) => Some(Action::JobDone(r)),
                };
                if let Some(a) = action {
                    let fx = update::update(&mut self.state, a);
                    quit |= self.execute(fx);
                }
                if quit {
                    break;
                }
            }
            if quit {
                break;
            }
        }
        self.workers.shutdown();
        Ok(self.state.fatal.take())
    }

    /// Returns true to quit.
    fn execute(&mut self, effects: Vec<Effect>) -> bool {
        let mut queue = effects;
        let mut depth = 0;
        while !queue.is_empty() && depth < 8 {
            depth += 1;
            let mut follow_ups = Vec::new();
            for fx in queue.drain(..) {
                let op = match fx {
                    Effect::Quit => return true,
                    Effect::Submit(job, prio) => {
                        self.workers.submit(job, prio);
                        None
                    }
                    Effect::Copy(text) => Some(match ops::clipboard::copy(&text) {
                        Ok(method) => OpResult::Copied { text, method },
                        Err(error) => OpResult::Failed { op: "copy", error },
                    }),
                    Effect::Export { bytes, dest, overwrite } => Some(ops::export::write(&bytes, &dest, overwrite)),
                    Effect::Restore { bytes, mode, target } => Some(ops::restore::restore(&bytes, mode, &target)),
                };
                if let Some(r) = op {
                    follow_ups.extend(update::update(&mut self.state, Action::OpDone(r)));
                }
            }
            queue = follow_ups;
        }
        false
    }
}
