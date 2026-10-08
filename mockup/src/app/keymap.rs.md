# `src/app/keymap.rs`

## Purpose
Translate raw crossterm events into `Action`s, depending on context (overlay open,
focused pane, current view). `BINDINGS` is the documented key table; `ui/help.rs`
renders it.

## Contents

```rust
pub enum Context { Global, Timeline, File, DiffViews, BlameView }
impl Context { pub fn title(self) -> &'static str; }

pub struct Binding { pub keys: &'static str, pub action: &'static str, pub context: Context }
pub static BINDINGS: &[Binding];

pub fn map(s: &AppState, ev: Event) -> Option<Action>;
fn map_key(s: &AppState, k: KeyEvent) -> Option<Action>;
fn map_mouse(s: &AppState, m: MouseEvent) -> Option<Action>;
```

## Event mapping
* Key events: `Press` and `Repeat` (Release ignored).
* `Paste(text)` → `InputPaste` only in `ExportPrompt`.
* `Resize` → `Layout(s.layout)` (forces a redraw; the draw computes the new layout).

## Resolution order (keys)
1. `Ctrl-C` → `Quit`, always.
2. Overlay open → overlay keys only:
   * **ExportPrompt**: `Enter` Confirm, `Esc` Cancel, `Backspace`, `Delete`, `←/→`,
     `Home/End`, `Ctrl-A/Ctrl-E` (home/end), other chars → `InputChar`.
   * **ConfirmRestore / ConfirmOverwrite**: `y`/`Y` Confirm; `n`/`N`/`q`/`Esc` Cancel.
     **`Enter` does nothing** (no accidental confirmation).
   * **Help / Details**: `j`/`↓`, `k`/`↑` scroll 1; `PgDn`/`Space`, `PgUp` scroll 10;
     `Esc`/`q`/`?`/`Enter`/`d` close.
   * **Error**: any key closes.
3. Focused-pane keys, then (without Ctrl) global keys.

## Bindings

### Global
| Keys | Action |
|---|---|
| `q`, `Ctrl-C` | Quit |
| `?` | ShowHelp |
| `Tab`, `Shift-Tab` | ToggleFocus |
| `1`–`4`, `F1`–`F4` | SetView(Content / DiffPrev / DiffMarked / Blame) |
| `[` / `]` | SelectNext (older) / SelectPrev (newer), from either pane |
| `m` | ToggleMark |
| `z` | ToggleZoom |
| `d` | ShowDetails |
| `y` | CopyHash |
| `e` | StartExport |
| `R` | StartRestore |
| `r` | Reload |

### Timeline pane
| Keys | Action |
|---|---|
| `j`/`↓`, `k`/`↑` | SelectNext / SelectPrev |
| `g`/`Home`, `G`/`End` | SelectFirst / SelectLast |
| `PgDn`/`Ctrl-F`, `PgUp`/`Ctrl-B` | SelectPageDown / SelectPageUp |
| `Enter`, `l`, `→` | FocusFile |

### File pane
| Keys | Action |
|---|---|
| `j`/`↓`, `k`/`↑` | CursorDown(1) / CursorUp(1) |
| `Ctrl-D` / `Ctrl-U` | HalfPageDown / HalfPageUp |
| `PgDn`, `Space`, `Ctrl-F` / `PgUp`, `Ctrl-B` | PageDown / PageUp |
| `g`/`Home`, `G`/`End` | Top / Bottom |
| `h`/`←`, `l`/`→` | ScrollLeft / ScrollRight |
| `#` | ToggleLineNumbers |
| `Esc` | FocusTimeline |
| `n` / `N` (diff views) | NextHunk / PrevHunk |
| `Enter` (blame view) | BlameJump |

### Mouse (ignored while an overlay is open)
* Wheel over timeline → `WheelTimeline(±3)`; over file pane → `WheelFile(±3)`.
* Left click in the timeline (inside the border) → `SelectIndex(list_offset + row)`.
* Left click in the file body → `ClickFileRow(row - body.y)`; elsewhere in the file
  pane → `FocusFile`.
* Hit-testing uses `AppState::layout` (`timeline`, `file`, `file_body` rects).

## Notes
* No vim count prefix: digits `1`–`4` are global view switches.
* A fast `Esc` + key arrives as Alt+key; modifiers other than Ctrl are ignored, so it maps
  like the plain key.
