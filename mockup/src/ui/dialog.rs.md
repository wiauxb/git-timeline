# `src/ui/dialog.rs`

## Purpose
Render every modal overlay: dispatches `Overlay::Help` to `help.rs` and draws
confirmation, text-input, commit-details and error popups.

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState, overlay: &Overlay);

/// Centered popup of `width` × (lines + 2), `Clear` underneath, wrapped paragraph,
/// border `danger` or `border_focused`. Returns the popup rect.
fn frame<'a>(f: &mut Frame, area: Rect, title: &'a str, lines: Vec<Line<'a>>, width: u16, danger: bool) -> Rect;

fn confirm_restore(f: &mut Frame, area: Rect, s: &AppState, rev: &RevisionId, overwrites: bool);
fn export_prompt(f: &mut Frame, area: Rect, input: &TextInput, err: Option<&str>);
fn confirm_overwrite(f: &mut Frame, area: Rect, dest: &Path);
fn error(f: &mut Frame, area: Rect, msg: &str);
fn details(f: &mut Frame, area: Rect, s: &AppState, oid: &Oid, scroll: usize);
```

## Dialogs

**ConfirmRestore** (width 64; danger border when `overwrites`)
```
┌ Restore version ─────────────────────────────────────────────┐
│Write a6ac609 "v1: hello"                                     │
│to src/main.rs in the working tree?                           │
│                                                              │
│⚠ src/main.rs has uncommitted changes. They will be lost.     │   (only if overwrites)
│                                                              │
│The index is not modified.     [y] Yes  [n] No                │
└──────────────────────────────────────────────────────────────┘
```
For pseudo-entries the "what" is the label (`Staged`). Only `y`/`Y` confirms; `n`, `N`,
`q`, `Esc` cancel; `Enter` does nothing (keymap).

**ExportPrompt** (width 70)
```
Save this version as (relative to where you launched git timeline):
> app@a6ac609.rs
<error, danger style, if any>
Enter save · Esc cancel
```
The terminal cursor is placed at the insertion point (popup x + 1 + 2 + display width of
the chars before `input.cursor`, row 2 of the popup), unless that would hit the border.

**ConfirmOverwrite** (width 70, danger) — `<dest> already exists.` / `Overwrite?  [y] Yes  [n] No`.

**Error** (width 70, danger) — the message, blank line, dim `press any key`.

**Details** (width 100, height min(lines + 2, area − 2), scrollable, title
` Commit details · Esc close `)
```
commit 0da79193394f92169e0150cbd219746ec353faf9  (HEAD -> master, origin/master)
Author:    Robby Russell <robby@…>  2026-09-07 01:14 -0700
Committer: Robby Russell <robby@…>  2026-09-07 01:14 -0700
Parents:   992edcc

    <full message, indented 4>

Files changed (1):
 M oh-my-zsh.sh            ← bold when it is the explored file at this commit
```
Author/committer/parents come from the timeline's `CommitMeta` (shown immediately);
refs, body and files from the `details` cache (`<spinner> loading…` until then, error
text in danger style on failure). Files count shows `+` when truncated (200 max). Status
letter colored A/D/R-C/other.
