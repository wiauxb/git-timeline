# `tests/ui_snapshots.rs`

## Purpose
Lock down rendering with `ratatui::backend::TestBackend` + `insta` snapshots: build an
`AppState` by hand (or by feeding actions), call `ui::draw`, snapshot the buffer as text.

## Contents

```rust
fn render(state: &AppState, w: u16, h: u16) -> String;   // buffer → text (styles dropped)
fn render_styled(state: &AppState, w: u16, h: u16) -> String; // with style markers, few tests
```

## Snapshots
* `two_panes_content` (100×30) — timeline with pseudo-entries + mark, content view.
* `zoomed`, `narrow_terminal_autozoom` (50 cols).
* `diff_prev`, `diff_marked`, `diff_added_file`, `diff_no_newline_at_eof`.
* `blame_groups` — gutter only on group starts.
* `placeholders` — loading, absent, binary, submodule, no mark.
* `help_overlay`, `confirm_restore_with_warning`, `export_prompt`, `details_popup`.
* `wide_chars_and_tabs` — CJK + tabs with `hscroll` 3 (no split wide chars).
* `no_color` — monochrome theme.

## Notes
* Relative dates are rendered against a fixed "now" injected into `AppState` for tests
  (`AppState::clock: fn() -> DateTime<Local>`), so snapshots are stable.
