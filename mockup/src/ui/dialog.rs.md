# `src/ui/dialog.rs`

## Purpose
Render all modal overlays except help: confirmation, text input, commit details, error.

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState, overlay: &Overlay);

fn confirm_restore(f: &mut Frame, area: Rect, s: &AppState, rev: &RevisionId, overwrites: bool);
fn export_prompt(f: &mut Frame, area: Rect, input: &TextInput, error: Option<&str>);
fn confirm_overwrite(f: &mut Frame, area: Rect, dest: &Path);
fn details(f: &mut Frame, area: Rect, s: &AppState, oid: &Oid);
fn error(f: &mut Frame, area: Rect, msg: &str);
```

## Dialogs

**ConfirmRestore**
```
┌ Restore version ───────────────────────────────┐
│ Write abc1234 "Fix crash on empty file"         │
│ to src/app.rs in the working tree?              │
│                                                 │
│ ⚠ src/app.rs has uncommitted changes.           │  (danger style, only if overwrites)
│   They will be lost.                            │
│                                                 │
│ The index is not modified.     [y] Yes  [n] No  │
└─────────────────────────────────────────────────┘
```
Default is **No**: only `y` confirms; `n`, `Esc`, `q` cancel. `Enter` does nothing
(prevents accidental confirmation).

**ExportPrompt** — single-line input prefilled with the default name, cursor at end;
`Enter` confirms, `Esc` cancels; bracketed paste inserts text. Shows `error` below in red.

**ConfirmOverwrite** — "`<dest>` exists. Overwrite? [y/n]".

**Details** (`d`) — scrollable popup:
```
commit abc1234…(full)            refs: main, tag: v1.2
Author:    Alice <alice@x> 2024-03-02 14:03 +0100
Committer: Bob <bob@x>     2024-03-03 09:12 +0100
Parents:   9f0e1d2 [1a2b3c4]

<full message body>

Files changed (12):
 M src/app.rs
 R src/old.rs → src/new.rs
 …
```
Body/refs/files appear when `LoadDetails` completes (spinner meanwhile).

**Error** — red-bordered box with the message, closed by any key.
