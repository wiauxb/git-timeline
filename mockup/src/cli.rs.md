# `src/cli.rs`

## Purpose
Declarative command-line interface (clap derive) and its conversion into `LogFilters`
and `UiOptions`.

## Synopsis

```
git timeline [OPTIONS] <PATH> [REVISION_RANGE]
```

## Contents

```rust
#[derive(clap::Parser, Debug, Clone)]
#[command(name = "git timeline", bin_name = "git timeline", version, about)]
pub struct Cli {
    pub path: PathBuf,                    // relative to the current directory
    pub revision_range: Option<String>,   // anything `git log` accepts
    #[arg(short = 'L', value_name = "START,END", value_parser = parse_line_range,
          conflicts_with = "no_follow")]
    pub line_range: Option<String>,
    #[arg(long)] pub author: Vec<String>, // repeatable
    #[arg(long)] pub since: Option<String>,
    #[arg(long)] pub until: Option<String>,
    #[arg(long)] pub no_follow: bool,
    #[arg(long)] pub no_highlight: bool,
    #[arg(long)] pub theme: Option<String>,
    #[arg(long, value_enum, default_value = "content")] pub view: InitialView,
    #[arg(long, default_value_t = 4, value_parser = value_parser!(u8).range(1..=16))]
    pub tab_width: u8,
}

pub enum InitialView { Content, Diff, Blame }
pub struct UiOptions { pub initial_view: InitialView, pub tab_width: u8 }

impl Cli {
    pub fn log_filters(&self) -> LogFilters;   // range, authors, since, until
    pub fn theme_name(&self) -> String;        // --theme, else $GIT_TIMELINE_THEME, else "base16-ocean.dark"
    pub fn ui_options(&self) -> UiOptions;
}
fn parse_line_range(s: &str) -> Result<String, String>;
```

## Validation rules
* `-L` must contain `,` and no `:` (the `:path` part is ours); it is otherwise passed
  verbatim to git, which validates it during discovery.
* `-L` conflicts with `--no-follow` (git's `-L` tracks lines across renames itself).
* `REVISION_RANGE` is validated during discovery. When given, no pseudo-entries.
* `--author`/`--since`/`--until` are forwarded as `--author=<v>` etc.

## Tests
Unit test `parses`: full option parse; `-L`+`--no-follow` conflict; `-L` with `:` or
without `,` rejected.
