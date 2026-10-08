//! Terminal lifecycle: always restored, on normal exit, error or panic.

use std::io::{stdout, Stdout};
use std::sync::Once;

use ratatui::crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::CrosstermBackend;

use crate::error::Result;

pub type Backend = CrosstermBackend<Stdout>;

pub struct Terminal {
    inner: ratatui::Terminal<Backend>,
    active: bool,
}

impl Terminal {
    pub fn enter() -> Result<Self> {
        install_panic_hook();
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
        let mut inner = ratatui::Terminal::new(CrosstermBackend::new(stdout()))?;
        inner.hide_cursor()?;
        inner.clear()?;
        Ok(Terminal { inner, active: true })
    }

    pub fn leave(&mut self) -> Result<()> {
        if self.active {
            self.active = false;
            restore_raw();
            self.inner.show_cursor()?;
        }
        Ok(())
    }

    pub fn draw(&mut self, f: impl FnOnce(&mut ratatui::Frame)) -> Result<()> {
        self.inner.draw(f)?;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

fn restore_raw() {
    let _ = execute!(stdout(), DisableBracketedPaste, DisableMouseCapture, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), ratatui::crossterm::cursor::Show);
}

/// Panics on the main thread restore the terminal before printing. Worker panics are
/// caught and reported in the UI, so they must not tear the terminal down.
fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let name = std::thread::current().name().map(str::to_owned);
            if name.as_deref().is_some_and(|n| n.starts_with("worker-")) {
                return;
            }
            restore_raw();
            previous(info);
        }));
    });
}
