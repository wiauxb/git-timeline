use std::process::ExitCode;

use clap::Parser;
use git_timeline::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse(); // usage errors exit with 2
    match git_timeline::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("git-timeline: {e}");
            ExitCode::FAILURE
        }
    }
}
