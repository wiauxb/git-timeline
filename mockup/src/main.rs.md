# `src/main.rs`

## Purpose
Thin binary entry point: parse the CLI, call `git_timeline::run`, map the result to an
exit code. No modules, no logic.

## Contents

```rust
use git_timeline::cli::Cli;

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();               // usage errors: clap prints and exits 2
    match git_timeline::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("git-timeline: {e}"); ExitCode::FAILURE }
    }
}
```

## Exit codes
* `0` normal quit
* `1` error (printed after the terminal is restored, prefixed `git-timeline: `)
* `2` CLI usage error (clap)

The lifecycle steps are described in `lib.rs.md`.
