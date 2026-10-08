# `src/main.rs`

## Purpose
Thin binary entry point. Parses the CLI and calls `git_timeline::run` (in `lib.rs`);
maps the result to an exit code. Contains no business logic and declares no modules.

## Contents

```rust
use git_timeline::cli::Cli;

fn main() -> std::process::ExitCode   // Cli::parse(), git_timeline::run(cli), exit code
```

Steps 2-8 below are implemented by `git_timeline::run` in `lib.rs`; they are described
here because they define the process lifecycle.

## Behavior (in order)

1. (`main`) `let cli = Cli::parse();` — clap handles `-h`, `-V`, usage errors (exit 2).
2. Optional debug logging: if env `GIT_TIMELINE_LOG=<file>` is set, append plain-text
   logs there (never to stderr while the TUI is active).
3. `let ctx = git::repo::RepoContext::discover(&cli, std::env::current_dir()?)?;`
   * Fails *before* the TUI starts on: git missing, git < 2.36, not in a repository,
     path is a directory, path has no history and no local existence, bad revision range.
4. Refuse to start if stdout or stdin is not a TTY → `Error::NotATty`.
5. `let highlighter = highlight::Highlighter::new(cli.theme_name(), cli.no_highlight);`
6. `let mut term = terminal::Terminal::enter()?;` (installs panic hook that restores the
   terminal before printing the panic).
7. `app::App::new(ctx, cli.into(), highlighter).run(&mut term)` — blocks until quit.
8. `term.leave()` (also done in `Drop`).
9. Exit codes:
   * `0` normal quit
   * `1` runtime error (message printed to stderr *after* terminal restore, formatted by
     `error::Error`'s `Display`, prefixed by `git-timeline: `)
   * `2` CLI usage error (clap)

## Notes
* `git` runs external subcommands **in the caller's working directory** and does not set
  `GIT_PREFIX` for them (verified), so the path argument is resolved against
  `current_dir()`.
