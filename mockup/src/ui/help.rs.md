# `src/ui/help.rs`

## Purpose
`?` overlay listing all key bindings, generated from `app::keymap::BINDINGS` (single
source of truth).

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, scroll: usize);
```

## Specs
* Centered popup, 70 × min(term-4, content) cells; sections per `Context`
  (Global, Timeline, File, Diff views, Blame view).
* Scrollable with `j`/`k` if it doesn't fit; closed with `?`, `Esc`, `q`.
* Footer: `git-timeline <version> · git <version>`.
