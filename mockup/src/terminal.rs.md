# `src/terminal.rs`

## Purpose
Own the terminal lifecycle so that it is **always** restored: normal exit, error, or
panic.

## Contents

```rust
pub type Backend = ratatui::backend::CrosstermBackend<std::io::Stdout>;

pub struct Terminal { inner: ratatui::Terminal<Backend>, active: bool }

impl Terminal {
    /// enable_raw_mode, EnterAlternateScreen, EnableMouseCapture,
    /// EnableBracketedPaste (for the export path input), hide cursor,
    /// install panic hook.
    pub fn enter() -> crate::error::Result<Self>;

    /// Reverse of `enter`. Idempotent.
    pub fn leave(&mut self) -> crate::error::Result<()>;

    pub fn draw(&mut self, f: impl FnOnce(&mut ratatui::Frame)) -> crate::error::Result<()>;
    pub fn size(&self) -> ratatui::layout::Size;
}

impl Drop for Terminal { fn drop(&mut self) { let _ = self.leave(); } }

fn install_panic_hook();   // wraps the previous hook: restore terminal, then call it
```

## Specs
* The panic hook only restores the terminal via raw crossterm calls (it cannot borrow
  `Terminal`); it is installed once (guarded by `std::sync::Once`).
* `leave` order: `DisableBracketedPaste`, `DisableMouseCapture`, `LeaveAlternateScreen`,
  `disable_raw_mode`, show cursor.
* Ctrl-Z (suspend) is **not** supported in v1 (raw mode swallows it; documented).
* Resize events are handled by the event loop (just redraw; ratatui autoresizes).
