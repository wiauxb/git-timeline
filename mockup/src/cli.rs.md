# `src/cli.rs`

## Purpose
Declarative command-line interface (clap derive) and conversion into the internal
`LogQuery` + `UiOptions`.

## Synopsis

```
git timeline [OPTIONS] <PATH> [REVISION_RANGE]
```

## Contents

```rust
#[derive(clap::Parser)]
#[command(name = "git timeline", bin_name = "git timeline", version, about)]
pub struct Cli {
    /// File to explore (relative to the current directory).
    pub path: std::path::PathBuf,

    /// Revision or range, as accepted by `git log` (e.g. `main`, `v1.0..HEAD`).
    pub revision_range: Option<String>,

    /// Only revisions touching lines <START>,<END> (same syntax as `git log -L`:
    /// numbers, `+N` offsets, or `/regex/`).
    #[arg(short = 'L', value_name = "START,END")]
    pub line_range: Option<String>,

    #[arg(long)] pub author: Vec<String>,     // repeatable, OR-ed by git
    #[arg(long)] pub since: Option<String>,
    #[arg(long)] pub until: Option<String>,

    /// Do not follow renames.
    #[arg(long)] pub no_follow: bool,

    /// Disable syntax highlighting.
    #[arg(long)] pub no_highlight: bool,

    /// syntect theme name (default: env GIT_TIMELINE_THEME or "base16-ocean.dark").
    #[arg(long)] pub theme: Option<String>,

    /// Initial view.
    #[arg(long, value_enum, default_value = "content")]
    pub view: InitialView,                       // content | diff | blame

    /// Tab width used when rendering file content.
    #[arg(long, default_value_t = 4)] pub tab_width: u8,
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum InitialView { Content, Diff, Blame }

pub struct UiOptions { pub initial_view: InitialView, pub tab_width: u8 }

impl Cli {
    pub fn log_filters(&self) -> crate::git::log::LogFilters;   // author/since/until/range
    pub fn theme_name(&self) -> String;
}
```

## Validation rules
* `-L` value is passed **verbatim** to git (`-L<value>:<path>`); we only check that it
  contains a `,` and no `:` (the `:path` part is ours). Git reports semantic errors and
  `RepoContext::discover` turns them into `Error::Git` before the TUI starts.
* `-L` implies following (git's `-L` tracks the lines across renames by itself);
  combining `-L` with `--no-follow` is a clap conflict.
* `revision_range` is validated by `git rev-parse --verify` (single rev) or
  `git rev-list --max-count=0 <range>` (range syntax) during discovery.
* **Ambiguity:** `PATH` comes first. There is no `--` separator need because
  `REVISION_RANGE` is optional and positional-second.
* When `REVISION_RANGE` is given, the `Working tree` / `Staged` pseudo-entries are **not**
  shown (they only make sense when the history ends at `HEAD`).
* `--author`, `--since`, `--until` are forwarded as `--author=<v>` etc. (the `=` form
  prevents values starting with `-` from being parsed as options).

## Tests
* parsing of every option; `-L` + `--no-follow` conflict; `-L` with `:` rejected.
