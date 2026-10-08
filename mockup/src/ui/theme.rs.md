# `src/ui/theme.rs`

## Purpose
Every non-syntax style in one place, so the look is consistent and `NO_COLOR` is honored
in one spot.

## Contents

```rust
pub struct Theme {
    pub border: Style, pub border_focused: Style, pub title: Style,
    pub selected_row: Style, pub selected_row_unfocused: Style,
    pub cursor_line: Style, pub cursor_line_unfocused: Style,
    pub hash: Style, pub date: Style, pub author: Style, pub subject: Style, pub dim: Style,
    pub pseudo_entry: Style, pub mark: Style,
    pub change_added: Style, pub change_deleted: Style, pub change_renamed: Style, pub change_other: Style,
    pub diff_added: Style, pub diff_removed: Style, pub diff_hunk_header: Style,
    pub diff_added_bg: Style, pub diff_removed_bg: Style,     // background-only styles
    pub line_number: Style,
    pub blame_gutter: Style, pub blame_age: [Color; 6],       // newest → oldest
    pub status_badge: Style, pub status_tab: Style, pub status_tab_active: Style,
    pub status_info: Style, pub status_warn: Style, pub status_error: Style,
    pub danger: Style, pub link: Style,
    pub search_match: Style,                                  // search hits (both panes)
    pub mono: bool,                                           // monochrome theme active
}

pub fn theme() -> &'static Theme;   // chosen once (OnceLock): colored() or monochrome()
fn colored() -> Theme;
fn monochrome() -> Theme;
```

## Specs
* **Colored** theme uses ANSI named colors (adapts to the terminal palette) plus a few
  256-color indexes: cursor line `Indexed(236)`, added-line bg `Indexed(22)`, removed-line
  bg `Indexed(52)`. Selected row: white on blue (focused) / white on dark gray
  (unfocused). Blame age: LightRed, LightYellow, Yellow, Green, Cyan, Blue.
* **Monochrome** when `NO_COLOR` is set and non-empty: only `BOLD`, `DIM`, `REVERSED`,
  `UNDERLINED`; diff backgrounds are empty styles, `blame_age` is all `Reset`, `mono = true`
  (renderers then skip age coloring).
* `search_match`: black on yellow (colored) / `REVERSED` (monochrome).
* `link` (underline) marks blame hashes that can be jumped to.
