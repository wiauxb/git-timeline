//! git-timeline: browse every version of a file through its git history.
//!
//! The library holds every module so integration tests can use them; `main.rs` is a
//! thin binary on top. Not a public API.

pub mod app;
pub mod cli;
pub mod error;
pub mod git;
pub mod highlight;
pub mod model;
pub mod ops;
pub mod terminal;
pub mod ui;
pub mod worker;

use std::io::IsTerminal;

pub use error::{Error, Result};

/// Discover the repository, run the TUI until quit.
pub fn run(cli: cli::Cli) -> anyhow::Result<()> {
    let ctx = git::repo::RepoContext::discover(&cli, std::env::current_dir()?)?;
    if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        return Err(Error::NotATty.into());
    }
    let highlighter = highlight::Highlighter::new(&cli.theme_name(), cli.no_highlight);
    let app = app::App::new(ctx, cli.ui_options(), highlighter);
    let mut term = terminal::Terminal::enter()?;
    let result = app.run(&mut term);
    term.leave()?;
    match result? {
        Some(fatal) => Err(anyhow::anyhow!(fatal)),
        None => Ok(()),
    }
}
