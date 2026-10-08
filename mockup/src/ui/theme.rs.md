# `src/ui/theme.rs`

## Purpose
Central place for every non-syntax style, so the look is consistent and `NO_COLOR`
is honored in one spot.

## Contents

```rust
pub struct Theme {
    pub border: Style, pub border_focused: Style,
    pub title: Style,
    pub selected_row: Style, pub selected_row_unfocused: Style,
    pub cursor_line: Style,
    pub hash: Style, pub date: Style, pub author: Style, pub subject: Style,
    pub pseudo_entry: Style,           // Working tree / Staged rows
    pub mark: Style,                   // ◆ marker
    pub change_added: Style, pub change_deleted: Style, pub change_renamed: Style,
    pub diff_added: Style, pub diff_removed: Style, pub diff_hunk_header: Style,
    pub diff_added_bg: Option<Color>, pub diff_removed_bg: Option<Color>,
    pub line_number: Style,
    pub blame_gutter: Style, pub blame_age: [Color; 6],   // newest → oldest
    pub status_info: Style, pub status_warn: Style, pub status_error: Style,
    pub dialog_border: Style, pub danger: Style,
}

pub fn theme() -> &'static Theme;     // chosen once: colored or monochrome
```

## Specs
* Uses the 16 **ANSI named colors** (not RGB) for UI chrome, so it adapts to the user's
  terminal palette (light or dark). Only syntax highlighting uses RGB.
* Diff lines: added/removed get a subtle background (`diff_*_bg`) on top of syntax
  foreground colors; in monochrome mode, `+`/`-` prefixes and bold/dim carry the meaning.
* `NO_COLOR` ⇒ monochrome theme: only `REVERSED`, `BOLD`, `DIM`, `UNDERLINED` modifiers.
