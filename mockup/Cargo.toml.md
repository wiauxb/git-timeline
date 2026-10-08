# `Cargo.toml`

## Purpose
Crate manifest. Produces a single binary named **`git-timeline`** so that, once on
`PATH`, git dispatches `git timeline …` to it (git runs any `git-<name>` executable
found on `PATH` as the subcommand `<name>`).

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
ratatui           = "0.29"        # TUI widgets/layout
crossterm         = "0.28"        # terminal backend + input events
clap              = { version = "4", features = ["derive", "wrap_help"] }
similar           = "2"           # in-process diff (Myers, with deadline)
syntect           = { version = "5", default-features = false, features = ["default-fancy"] }
                                  # default-fancy = pure-Rust regex (no onig C dep)
crossbeam-channel = "0.5"         # MPMC channels for worker pool / event loop
lru               = "0.12"        # bounded caches
bstr              = "1"           # byte-string paths (non-UTF-8 safe)
chrono            = { version = "0.4", default-features = false, features = ["std", "clock"] }
                                  # parse %aI, render relative dates
unicode-width     = "0.2"         # correct column widths (CJK, emoji)
thiserror         = "2"
anyhow            = "1"           # only at the lib::run / main boundary
base64            = "0.22"        # OSC 52 clipboard payload
arboard           = { version = "3", optional = true, default-features = false, features = ["wayland-data-control"] }

[features]
default   = ["clipboard"]
clipboard = ["dep:arboard"]       # without it, copy uses OSC 52 only

[dev-dependencies]
tempfile = "3"                    # throwaway fixture repos
insta    = "1"                    # snapshot tests of rendered frames

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
```

## Specs / notes
* **Binary name is the contract.** Nothing else (no wrapper script) is needed for
  `git timeline` to work.
* `git timeline --help` is **intercepted by git**: it turns into `git help timeline`, which
  looks for a man page `git-timeline(1)`. Therefore:
  * `-h` must print clap help (works, git passes it through).
  * v1 ships no man page; README tells users to use `-h`. (Future: generate one with
    `clap_mangen` in a `xtask`, not in `build.rs`.)
* Library + binary in one crate: `src/lib.rs` (crate `git_timeline`, used by `tests/`)
  and `src/main.rs` (binary `git-timeline`).
* No `build.rs`. Syntax definitions come from syntect's embedded dump.
* Feature `clipboard` is on by default; packagers for headless systems can disable it.
