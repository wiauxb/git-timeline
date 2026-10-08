# `src/ops/clipboard.rs`

## Purpose
Copy a string (the commit hash) to the user's clipboard, including over SSH.

## Contents

```rust
pub enum CopyMethod { Clipboard, Osc52 }

pub fn copy(text: &str) -> Result<CopyMethod, String>;
fn copy_native(text: &str) -> Result<(), String>;      // arboard (feature "clipboard")
fn copy_osc52(text: &str) -> std::io::Result<()>;      // ESC ] 52 ; c ; <base64> BEL
```

## Strategy
1. If `SSH_CONNECTION` / `SSH_TTY` is set ⇒ **OSC 52 first** (native clipboard would be
   the remote machine's, useless).
2. Else try native (arboard). On Linux it needs X11/Wayland; failure ⇒ fall back to OSC 52.
3. OSC 52 is written directly to stdout (between frames; doesn't disturb ratatui).
   Inside tmux, wrap as `ESC P tmux; ESC <seq> ESC \` when `TMUX` is set.

## Notes
* OSC 52 success can't be detected (terminal may ignore it); status bar says
  "Copied abc1234 (via terminal OSC 52)" so the user knows which path was used.
* The `arboard::Clipboard` instance is created per call: on X11, the clipboard content
  is owned by the process, so we keep the last instance alive in a `static` until exit
  (otherwise the copied text vanishes when it is dropped).
