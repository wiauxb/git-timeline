//! Copy text to the clipboard: native clipboard, or OSC 52 (works over SSH).

use std::io::Write;

use base64::Engine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMethod {
    Clipboard,
    Osc52,
}

pub fn copy(text: &str) -> Result<CopyMethod, String> {
    let remote = std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_TTY").is_some();
    if !remote && copy_native(text).is_ok() {
        return Ok(CopyMethod::Clipboard);
    }
    copy_osc52(text).map(|_| CopyMethod::Osc52).map_err(|e| e.to_string())
}

#[cfg(feature = "clipboard")]
fn copy_native(text: &str) -> Result<(), String> {
    use std::sync::Mutex;
    // On X11 the selection is owned by the process: keep the instance alive until exit.
    static KEEP: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);
    let mut guard = KEEP.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
    }
    guard.as_mut().unwrap().set_text(text.to_string()).map_err(|e| e.to_string())
}

#[cfg(not(feature = "clipboard"))]
fn copy_native(_text: &str) -> Result<(), String> {
    Err("built without clipboard support".into())
}

fn copy_osc52(text: &str) -> std::io::Result<()> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(text);
    let seq = format!("\x1b]52;c;{b64}\x07");
    let seq = if std::env::var_os("TMUX").is_some() {
        format!("\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b"))
    } else {
        seq
    };
    let mut out = std::io::stdout();
    out.write_all(seq.as_bytes())?;
    out.flush()
}
