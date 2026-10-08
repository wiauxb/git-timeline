# `src/highlight.rs`

## Purpose
Turn a `TextContent` into per-line styled ranges using **syntect**. Runs on worker
threads; results are cached on the main thread (keyed by content + file name) and used
by content, diff and blame views.

## Contents

```rust
#[derive(Clone)]
pub struct Highlighter {
    inner: Option<Arc<Inner>>,          // None: --no-highlight or NO_COLOR
    pub theme_warning: Option<String>,  // unknown theme → shown once in the status bar
}
struct Inner { syntaxes: SyntaxSet /* load_defaults_newlines */, theme: Theme }

pub struct HighlightedText {
    pub syntax_name: String,
    /// line i ↔ TextContent line i; byte ranges into `TextContent::line_str(i)`.
    pub lines: Vec<Vec<(Range<usize>, ratatui::style::Style)>>,
}

#[derive(Clone)]
pub enum HighlightOutcome {
    Highlighted(Arc<HighlightedText>),
    Plain { reason: &'static str },     // shown in the file header
}

impl Highlighter {
    pub fn new(theme_name: &str, disabled: bool) -> Self;
    pub fn disabled() -> Self;
    pub fn enabled(&self) -> bool;
    pub fn highlight(&self, text: &TextContent, path: &RepoPath) -> HighlightOutcome;
}
```

## Specs
* Syntax choice uses the path **of that revision**: full file name as extension
  (`Makefile`), then extension, then first line (shebang). "Plain Text" → `Plain`.
* Limits → `Plain { "no highlighting (too large)" }`: > 1 MiB, > 20 000 lines, or any line
  > 10 000 bytes. Other reasons: "highlighting disabled", "plain text",
  "highlighting failed".
* Each line is fed with a trailing `\n` (newlines syntax set); ranges are clipped to the
  line length.
* Colors: syntect RGB fg → `Color::Rgb`, bold/italic/underline; theme background ignored.
* Unknown theme → falls back to `base16-ocean.dark` and sets `theme_warning` (lists
  available names).
* `NO_COLOR` non-empty ⇒ disabled regardless of flags.
* Content is shown plain first and redrawn when the highlight job completes.

## Tests
`highlights_rust`: `src/main.rs` → syntax "Rust", 3 lines, last range ends at line end.
