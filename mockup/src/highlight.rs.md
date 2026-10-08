# `src/highlight.rs`

## Purpose
Turn a `TextContent` into per-line styled spans using **syntect**. Runs on worker
threads; the result is cached on the main thread and reused by all views (content,
diff, blame all show highlighted lines).

## Contents

```rust
pub struct Highlighter {                // cheap to clone (Arc inside)
    inner: Option<Arc<Inner>>,          // None when --no-highlight / NO_COLOR
}
struct Inner {
    syntaxes: syntect::parsing::SyntaxSet,     // SyntaxSet::load_defaults_newlines()
    theme: syntect::highlighting::Theme,       // from ThemeSet::load_defaults()
}

/// Styled version of one blob; line i ↔ TextContent line i.
pub struct HighlightedText {
    pub syntax_name: String,
    pub lines: Vec<Vec<StyledSpan>>,
}
pub struct StyledSpan { pub range: std::ops::Range<usize>, pub style: ratatui::style::Style }
// `range` is a byte range inside the line (no string copy).

pub enum HighlightOutcome { Highlighted(Arc<HighlightedText>), Plain /* skipped */ }

impl Highlighter {
    pub fn new(theme_name: String, disabled: bool) -> Self;
    pub fn enabled(&self) -> bool;
    /// Decide the syntax from the path *of that revision* (renames can change the
    /// extension), falling back to the first line (shebang, modelines).
    pub fn syntax_for(&self, path: &RepoPath, first_line: &str) -> Option<String>;
    pub fn highlight(&self, content: &TextContent, syntax: &str) -> HighlightOutcome;
}
```

## Specs
* **Limits:** skip (return `Plain`) when the text is > 1 MiB or > 20 000 lines, or any line
  is > 10 000 bytes (minified files make syntect very slow).
* **Cache key** (in `app/cache.rs`): `(ContentKey, syntax_name)`.
* syntect requires a full pass from line 0 (stateful parser); there is no lazy
  per-viewport highlighting in v1. Typical files take a few ms; the content is shown
  **plain first**, then re-drawn highlighted when the job completes (no blocking).
* Colors: syntect RGB → `ratatui::style::Color::Rgb`. Only foreground + bold/italic/
  underline are used; the theme background is ignored (the terminal's background is kept).
* Unknown theme name → fall back to default and set a status-bar warning once.
* `NO_COLOR` set (non-empty) ⇒ highlighter disabled, regardless of flags.
