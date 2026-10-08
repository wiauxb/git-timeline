# `src/app/keymap.rs`

## Purpose
Translate raw crossterm events into `Action`s, **depending on context** (overlay open,
focused pane, current view). The single authoritative key-binding table; `ui/help.rs`
renders from the same table so help can't drift.

## Contents

```rust
pub fn map(s: &AppState, ev: crossterm::event::Event) -> Option<Action>;

pub struct Binding { pub keys: &'static [&'static str], pub action: &'static str,
                     pub context: Context }
pub enum Context { Global, Timeline, File, DiffViews, BlameView, TextInput, Confirm }
pub static BINDINGS: &[Binding];        // used by help overlay
```

## Resolution order
1. Overlay open ⇒ overlay bindings only (`TextInput`, `Confirm`, `Help`/`Details`/`Error`
   which close on `Esc`/`q`/`Enter`; `Help` also scrolls with `j`/`k`).
2. Focused pane bindings.
3. Global bindings.

## Bindings

### Global
| Keys | Action |
|---|---|
| `q`, `Ctrl-C` | Quit |
| `?` | ShowHelp |
| `Tab`, `Shift-Tab` | ToggleFocus |
| `1` / `2` / `3` / `4` | SetView(Content / DiffPrev / DiffMarked / Blame) |
| `m` | ToggleMark |
| `z` | ToggleZoom |
| `d` | ShowDetails |
| `y` | CopyHash |
| `e` | StartExport |
| `R` | StartRestore (capital: destructive) |
| `r` | Reload |
| `[` / `]` | SelectNext (older) / SelectPrev (newer) — works from both panes |

### Timeline pane
| Keys | Action |
|---|---|
| `j`, `↓` / `k`, `↑` | SelectNext / SelectPrev |
| `g`, `Home` / `G`, `End` | SelectFirst / SelectLast |
| `PgDn`, `Ctrl-F` / `PgUp`, `Ctrl-B` | SelectPageDown / SelectPageUp |
| `Enter`, `l`, `→` | FocusFile |

### File pane
| Keys | Action |
|---|---|
| `j`, `↓` / `k`, `↑` | CursorDown(1) / CursorUp(1) |
| `Ctrl-D` / `Ctrl-U` | HalfPageDown / HalfPageUp |
| `PgDn`, `Space` / `PgUp` | PageDown / PageUp |
| `g` / `G` | Top / Bottom |
| `h`, `←` / `l`, `→` | ScrollLeft / ScrollRight (when not wrapping) |
| `Esc` | FocusTimeline |
| `#` | ToggleLineNumbers |
| `w` | ToggleWrap |
| `n` / `N` | NextHunk / PrevHunk (diff views) |
| `Enter` | BlameJump (blame view) |

### Mouse
* Wheel over a pane ⇒ 3× Cursor/Select up/down in that pane (without changing focus).
* Left click in timeline ⇒ `SelectIndex(row + list_offset)` + focus timeline.
* Left click in file pane ⇒ focus file + move cursor to that row.
* Pane hit-testing uses the rects stored by `ui::draw` in `AppState::layout`.

## Notes
* No vim count prefix (`10j`) in v1: digits `1-4` are global view switches, and
  mixing both would be ambiguous.
* Keys are matched on `KeyEventKind::Press` (and `Repeat`) only — Windows emits releases.
