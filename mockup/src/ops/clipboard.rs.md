# `src/ops/clipboard.rs`

## Purpose
Copy a string (the full commit hash) to the clipboard, including over SSH.

## Contents

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMethod { Clipboard, Osc52 }

pub fn copy(text: &str) -> Result<CopyMethod, String>;

#[cfg(feature = "clipboard")]     fn copy_native(text: &str) -> Result<(), String>; // arboard
#[cfg(not(feature = "clipboard"))] fn copy_native(text: &str) -> Result<(), String>; // always Err
fn copy_osc52(text: &str) -> std::io::Result<()>;
```

## Strategy
1. Remote session (`SSH_CONNECTION` or `SSH_TTY` set) ⇒ skip the native clipboard (it would
   be the remote machine's) and use OSC 52.
2. Otherwise try native (arboard); any failure ⇒ fall back to OSC 52.
3. OSC 52: `ESC ] 52 ; c ; <base64> BEL` written straight to stdout and flushed. Inside
   tmux (`TMUX` set) it is wrapped as `ESC P tmux; <seq with ESC doubled> ESC \`.

## Notes
* On X11 the selection is owned by the process, so the `arboard::Clipboard` is created once
  and kept alive in a `static Mutex<Option<Clipboard>>` until exit.
* OSC 52 success can't be detected; `update.rs` reports `copied <hash> (via terminal OSC 52)`
  so the user knows which path was used.
