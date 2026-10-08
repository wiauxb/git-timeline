# `Cargo.toml`

## Purpose
Crate manifest. One crate with a **library** (`src/lib.rs`, crate `git_timeline`, used by
`tests/`) and a **binary** named **`git-timeline`** (`src/main.rs`). Once on `PATH`, git
dispatches `git timeline …` to it.

## Contents

```toml
[package]
name = "git-timeline"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
license = "MIT OR Apache-2.0"
description = "Browse every version of a file through its git history"

[[bin]]
name = "git-timeline"
path = "src/main.rs"

[dependencies]
ratatui           = "0.30"   # TUI; its `ratatui::crossterm` re-export is the backend
clap              = { version = "4", features = ["derive", "wrap_help"] }
similar           = "2"      # in-process diff (Myers + deadline)
syntect           = { version = "5", default-features = false, features = ["default-fancy"] }
                             # pure-Rust regex, no onig C dep
crossbeam-channel = "0.5"    # worker queues (select_biased!), event channel
lru               = "0.18"   # bounded caches
bstr              = "1"      # byte-string paths
chrono            = { version = "0.4", default-features = false, features = ["std", "clock"] }
unicode-width     = "0.2"
regex             = "1"      # `/` search (already in the tree via syntect)
thiserror         = "2"
anyhow            = "1"      # only at the lib::run / main boundary
base64            = "0.22"   # OSC 52 payload
arboard           = { version = "3", optional = true, default-features = false, features = ["wayland-data-control"] }

[features]
default   = ["clipboard"]
clipboard = ["dep:arboard"]  # without it, copy uses OSC 52 only

[dev-dependencies]
tempfile = "3"               # throwaway fixture repos

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
```

## Notes
* **No direct `crossterm` dependency**: everything uses `ratatui::crossterm` so the
  backend version always matches ratatui's.
* **Binary name is the contract**: no wrapper script needed for `git timeline`.
* No `build.rs`; syntax definitions come from syntect's embedded dump. No man page
  (`--help` goes to git's man lookup; users use `-h`).
* `.gitignore` ignores `/target`; `Cargo.lock` is committed (binary crate).
