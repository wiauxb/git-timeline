# `src/terminal.rs`

## Purpose
Own the terminal lifecycle so it is **always** restored: normal exit, error, or panic.

## Contents

```rust
pub type Backend = ratatui::prelude::CrosstermBackend<Stdout>;

pub struct Terminal { inner: ratatui::Terminal<Backend>, active: bool }

impl Terminal {
    /// install panic hook, enable_raw_mode, EnterAlternateScreen, EnableMouseCapture,
    /// EnableBracketedPaste, hide cursor, clear.
    pub fn enter() -> Result<Self>;
    /// Idempotent restore.
    pub fn leave(&mut self) -> Result<()>;
    pub fn draw(&mut self, f: impl FnOnce(&mut ratatui::Frame)) -> Result<()>;
}
impl Drop for Terminal { /* leave() */ }

fn restore_raw();          // DisableBracketedPaste, DisableMouseCapture, LeaveAlternateScreen,
                           // disable_raw_mode, show cursor
fn install_panic_hook();   // once (std::sync::Once)
```

## Specs
* All crossterm items come from `ratatui::crossterm`.
* Panic hook: on threads named `worker-*` it does **nothing** (worker panics are caught
  with `catch_unwind` and reported in the UI; tearing the terminal down would break the
  running TUI). On any other thread: `restore_raw()` then the previous hook.
* The input thread must only start **after** `enter()` (see `app/mod.rs.md`).
* Resize: handled by the event loop (redraw). Ctrl-Z suspend not supported.
