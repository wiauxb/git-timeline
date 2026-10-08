# `src/ui/help.rs`

## Purpose
`?` overlay listing all key bindings, generated from `app::keymap::BINDINGS` (single
source of truth, so help can't drift from the keymap).

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState, scroll: usize);
```

## Specs
* Sections in order: Global, Timeline pane, File pane, Diff views, Blame view
  (`Context::title()`), blank line between sections; each binding is
  `  {keys:<26}` (`hash` style) + description.
* Footer (dim): `git-timeline <CARGO_PKG_VERSION> · git <x.y.z>` (from `ctx.git_version`).
* Popup: `centered(area, 82, min(lines + 2, height − 2))`, `Clear` underneath, bordered
  with title ` Keys `, `border_focused` style.
* Scroll offset clamped to `lines − inner height`; keys handled by keymap/update
  (`j`/`k`, PgUp/PgDn/Space scroll; `Esc`, `q`, `?`, `Enter`, `d` close).
