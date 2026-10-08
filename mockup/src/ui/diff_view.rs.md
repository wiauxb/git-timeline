# `src/ui/diff_view.rs`

## Purpose
Render a unified diff between two versions, for both `DiffPrev` (change introduced by
the selected revision) and `DiffMarked` (marked ↔ selected, older → newer). Also provides
the summary text for non-text pairs.

## Contents

```rust
pub struct DiffInputs<'a> {
    pub diff: &'a FileDiff,
    pub rows: &'a [DiffRow],             // FileDiff::rows(): HunkHeader / Line
    pub old: Option<&'a TextContent>, pub new: Option<&'a TextContent>,   // None = absent
    pub old_hl: Option<&'a HighlightedText>, pub new_hl: Option<&'a HighlightedText>,
    pub old_mode: FileMode, pub new_mode: FileMode,
}

pub fn render(f: &mut Frame, area: Rect, s: &AppState, d: &DiffInputs);
pub fn summary(old: &FileContent, new: &FileContent) -> String;   // "binary, 1.2 KiB  →  absent"
pub fn human(n: u64) -> String;                                   // B / KiB / MiB, 1 decimal
fn is_last_without_newline(t: &TextContent, line: usize) -> bool;
fn mode_str(m: FileMode) -> &'static str;                         // "100644" …
```

## Layout
```
@@ -148,7 +148,9 @@
148 148 │  unset zcompdump_revision
151     │- if command mkdir "${ZSH_COMPDUMP}.lock" 2>/dev/null; then
    151 │+ # Test that first so the lock directory …
152 154 │    zrecompile -q -p "$ZSH_COMPDUMP"
```
* Gutter (line numbers on): old and new 1-based numbers, each `digits(max line count)`
  wide, then ` │`; numbers off ⇒ only the 2-col marker. Text width = width − gutter.
* Marker `" "` / `"-"` / `"+"` followed by a space; removed/added rows get the
  `diff_removed_bg` / `diff_added_bg` tint across the full row (via `styled_line`'s bg fill).
* Text and syntax highlighting come from the **corresponding side** (`old` for removed
  rows, `new` for context/added rows). Unhighlighted `+`/`-` rows take the marker color.
* Hunk header row: `@@ -a,b +c,d @@` (1-based starts, no function context).
* Last line of a side without trailing newline gets a dim ` ⏎̸ no newline at end of file`.
* Cursor row styled with `content_view::cursor_style`.

## Empty diff (no rows)
Centered dim message at 1/3 height:
* `mode changed 100644 → 100755` when both sides exist and modes differ,
* `file absent on both sides` when both are absent,
* else `no changes`.

## `summary`
`<old desc>  →  <new desc>` with desc ∈ `absent`, `text, N lines`, `binary, <size>`,
`too large, <size>`, `symlink → <target>`, `submodule at <short>`.
