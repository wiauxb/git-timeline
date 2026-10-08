use std::path::PathBuf;

use clap::Parser;

use crate::git::log::LogFilters;

/// Browse every version of a file through its git history.
#[derive(Parser, Debug, Clone)]
#[command(name = "git timeline", bin_name = "git timeline", version, about)]
pub struct Cli {
    /// File to explore (relative to the current directory).
    pub path: PathBuf,

    /// Revision or range, as accepted by `git log` (e.g. `main`, `v1.0..HEAD`).
    pub revision_range: Option<String>,

    /// Only revisions touching lines START,END (same syntax as `git log -L`).
    #[arg(short = 'L', value_name = "START,END", value_parser = parse_line_range,
          conflicts_with = "no_follow")]
    pub line_range: Option<String>,

    /// Only commits by this author (repeatable).
    #[arg(long, value_name = "PATTERN")]
    pub author: Vec<String>,

    /// Only commits more recent than this date.
    #[arg(long, value_name = "DATE")]
    pub since: Option<String>,

    /// Only commits older than this date.
    #[arg(long, value_name = "DATE")]
    pub until: Option<String>,

    /// Do not follow renames.
    #[arg(long)]
    pub no_follow: bool,

    /// Disable syntax highlighting.
    #[arg(long)]
    pub no_highlight: bool,

    /// syntect theme name [default: $GIT_TIMELINE_THEME or base16-ocean.dark].
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,

    /// Initial view.
    #[arg(long, value_enum, default_value = "content")]
    pub view: InitialView,

    /// Tab width used when rendering file content.
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u8).range(1..=16))]
    pub tab_width: u8,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialView {
    Content,
    Diff,
    Blame,
}

#[derive(Clone, Copy, Debug)]
pub struct UiOptions {
    pub initial_view: InitialView,
    pub tab_width: u8,
}

impl Cli {
    pub fn log_filters(&self) -> LogFilters {
        LogFilters {
            range: self.revision_range.clone(),
            authors: self.author.clone(),
            since: self.since.clone(),
            until: self.until.clone(),
        }
    }

    pub fn theme_name(&self) -> String {
        self.theme
            .clone()
            .or_else(|| std::env::var("GIT_TIMELINE_THEME").ok().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| "base16-ocean.dark".to_string())
    }

    pub fn ui_options(&self) -> UiOptions {
        UiOptions { initial_view: self.view, tab_width: self.tab_width }
    }
}

fn parse_line_range(s: &str) -> Result<String, String> {
    if !s.contains(',') {
        return Err("expected START,END".into());
    }
    if s.contains(':') {
        return Err("give only START,END; the path is the PATH argument".into());
    }
    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let c = Cli::try_parse_from(["git-timeline", "-L", "1,5", "--author=a", "f.rs", "main"]).unwrap();
        assert_eq!(c.line_range.as_deref(), Some("1,5"));
        assert_eq!(c.revision_range.as_deref(), Some("main"));
        assert!(Cli::try_parse_from(["x", "-L", "1,5", "--no-follow", "f"]).is_err());
        assert!(Cli::try_parse_from(["x", "-L", "1,5:f", "f"]).is_err());
        assert!(Cli::try_parse_from(["x", "-L", "15", "f"]).is_err());
    }
}
