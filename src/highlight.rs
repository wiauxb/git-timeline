//! syntect-based syntax highlighting, run on worker threads.

use std::ops::Range;
use std::sync::Arc;

use ratatui::style::{Color, Modifier, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::model::{RepoPath, TextContent};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_LINES: usize = 20_000;
const MAX_LINE_LEN: usize = 10_000;

#[derive(Clone)]
pub struct Highlighter {
    inner: Option<Arc<Inner>>,
    pub theme_warning: Option<String>,
}

struct Inner {
    syntaxes: SyntaxSet,
    theme: Theme,
}

/// Styled version of one text; line i ↔ TextContent line i.
/// Ranges are byte ranges into `TextContent::line_str(i)`.
#[derive(Debug)]
pub struct HighlightedText {
    pub syntax_name: String,
    pub lines: Vec<Vec<(Range<usize>, Style)>>,
}

#[derive(Clone, Debug)]
pub enum HighlightOutcome {
    Highlighted(Arc<HighlightedText>),
    /// Skipped: disabled, too large, unknown syntax.
    Plain { reason: &'static str },
}

impl Highlighter {
    pub fn new(theme_name: &str, disabled: bool) -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        if disabled || no_color {
            return Highlighter { inner: None, theme_warning: None };
        }
        let mut themes = ThemeSet::load_defaults().themes;
        let mut theme_warning = None;
        let theme = match themes.remove(theme_name) {
            Some(t) => t,
            None => {
                let mut names: Vec<_> = themes.keys().cloned().collect();
                names.sort();
                theme_warning =
                    Some(format!("unknown theme '{theme_name}' (available: {})", names.join(", ")));
                themes.remove("base16-ocean.dark").expect("default theme bundled")
            }
        };
        Highlighter {
            inner: Some(Arc::new(Inner { syntaxes: SyntaxSet::load_defaults_newlines(), theme })),
            theme_warning,
        }
    }

    pub fn disabled() -> Self {
        Highlighter { inner: None, theme_warning: None }
    }

    pub fn enabled(&self) -> bool {
        self.inner.is_some()
    }

    fn syntax_for<'a>(inner: &'a Inner, path: &RepoPath, first_line: &str) -> Option<&'a SyntaxReference> {
        let ss = &inner.syntaxes;
        let name = String::from_utf8_lossy(path.file_name());
        let ext = path.extension().map(|e| String::from_utf8_lossy(e).into_owned());
        ss.find_syntax_by_extension(&name)
            .or_else(|| ext.as_deref().and_then(|e| ss.find_syntax_by_extension(e)))
            .or_else(|| ss.find_syntax_by_first_line(first_line))
    }

    pub fn highlight(&self, text: &TextContent, path: &RepoPath) -> HighlightOutcome {
        let Some(inner) = &self.inner else {
            return HighlightOutcome::Plain { reason: "highlighting disabled" };
        };
        if text.bytes().len() > MAX_BYTES
            || text.line_count() > MAX_LINES
            || text.lines().any(|l| l.len() > MAX_LINE_LEN)
        {
            return HighlightOutcome::Plain { reason: "no highlighting (too large)" };
        }
        let first = if text.line_count() > 0 { text.line_str(0).into_owned() } else { String::new() };
        let Some(syntax) = Self::syntax_for(inner, path, &first) else {
            return HighlightOutcome::Plain { reason: "plain text" };
        };
        if syntax.name == "Plain Text" {
            return HighlightOutcome::Plain { reason: "plain text" };
        }

        let mut h = HighlightLines::new(syntax, &inner.theme);
        let mut lines = Vec::with_capacity(text.line_count());
        for i in 0..text.line_count() {
            let mut line = text.line_str(i).into_owned();
            let len = line.len();
            line.push('\n');
            let mut spans = Vec::new();
            match h.highlight_line(&line, &inner.syntaxes) {
                Ok(regions) => {
                    let mut pos = 0;
                    for (style, piece) in regions {
                        let end = (pos + piece.len()).min(len);
                        if end > pos {
                            spans.push((pos..end, to_style(style)));
                        }
                        pos += piece.len();
                    }
                }
                Err(_) => return HighlightOutcome::Plain { reason: "highlighting failed" },
            }
            lines.push(spans);
        }
        HighlightOutcome::Highlighted(Arc::new(HighlightedText { syntax_name: syntax.name.clone(), lines }))
    }
}

fn to_style(s: syntect::highlighting::Style) -> Style {
    let c = s.foreground;
    let mut st = Style::default().fg(Color::Rgb(c.r, c.g, c.b));
    if s.font_style.contains(FontStyle::BOLD) {
        st = st.add_modifier(Modifier::BOLD);
    }
    if s.font_style.contains(FontStyle::ITALIC) {
        st = st.add_modifier(Modifier::ITALIC);
    }
    if s.font_style.contains(FontStyle::UNDERLINE) {
        st = st.add_modifier(Modifier::UNDERLINED);
    }
    st
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_rust() {
        let h = Highlighter { inner: Highlighter::new("base16-ocean.dark", false).inner, theme_warning: None };
        if !h.enabled() {
            return; // NO_COLOR in the environment
        }
        let t = TextContent::from_bytes(b"fn main() {\n    let x = 1;\n}\n".to_vec());
        match h.highlight(&t, &RepoPath::from_git_bytes(b"src/main.rs")) {
            HighlightOutcome::Highlighted(ht) => {
                assert_eq!(ht.syntax_name, "Rust");
                assert_eq!(ht.lines.len(), 3);
                assert_eq!(ht.lines[0].last().unwrap().0.end, "fn main() {".len());
            }
            other => panic!("{other:?}"),
        }
    }
}
