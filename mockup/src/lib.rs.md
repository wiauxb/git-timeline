# `src/lib.rs`

## Purpose
Library root (crate `git_timeline`). All modules live here so integration tests in
`tests/` can use them; `main.rs` is a thin binary on top. Not a public API (no semver
promise; `#[doc(hidden)]` test hooks such as `Git::with_env`, `BlobReader::with_limit`,
`worker::run_inline` are fine).

## Contents

```rust
pub mod app; pub mod cli; pub mod error; pub mod git; pub mod highlight;
pub mod model; pub mod ops; pub mod terminal; pub mod ui; pub mod worker;

pub use error::{Error, Result};

pub fn run(cli: cli::Cli) -> anyhow::Result<()>;
```

## `run` lifecycle
1. `RepoContext::discover(&cli, current_dir())` — fails before any TUI on: git missing,
   git < 2.36, not a repo, directory, outside repo, bad range, `-L` rejected by git,
   no history and no local existence.
2. stdin and stdout must be terminals (`IsTerminal`) → else `Error::NotATty`.
3. `Highlighter::new(&cli.theme_name(), cli.no_highlight)`.
4. `App::new(ctx, cli.ui_options(), highlighter)` — reads local state synchronously,
   spawns workers.
5. `Terminal::enter()`, then `app.run(&mut term)` (starts the input thread, blocks until
   quit), then `term.leave()`.
6. `run` returns `Ok(Some(fatal))` when the app quit on a fatal error (e.g. the initial
   timeline load failed with nothing to show) → turned into an `anyhow` error.
