# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`git-timeline`: a ratatui TUI to browse every version of one file through its git history.
The binary is named `git-timeline`, so once on `PATH`, git dispatches `git timeline <path>` to it.
Requires git ≥ 2.36 (`cat-file --batch-command`).

## Mandatory workflow rules

- **Design happens in `mockup/`, never directly in code.** `mockup/` holds one `.md` per
  repository file (`mockup/src/git/log.rs.md` ⇢ `src/git/log.rs`); `mockup/ARCHITECTURE.md`
  is the index (decisions, file tree, layering, key bindings, git command catalogue, verified
  git behaviors). All thinking about architecture and code organisation is done there. Work
  in a loop:
  1. **Mockup**: write the proposed change into the affected specs (new/removed/renamed
     files, signatures, behavior, decisions).
  2. **Agree**: present it to the user and **stop until they agree**. No code before that.
  3. **Code**: implement exactly the agreed mockup change, with tests.
  4. **Evaluate**: build, test, run. Anything learned goes back to step 1 as a mockup
     proposal, not as an ad-hoc code change.

  The mockup always describes the current code; never append "as implemented" deviation
  notes.
- **`TODO.md`** is the bug backlog. New suspected bugs go there (not only in chat). Fixing an
  item goes through the same loop: fix design in the mockup → agreement → code + test →
  remove the item.

## Commands

```sh
cargo build                                   # debug
cargo build --release                         # target/release/git-timeline
cargo test                                    # all tests (need git on PATH)
cargo test --lib model::line_map              # unit tests of one module
cargo test --test views anchor_follows_code   # one integration test
cargo clippy --all-targets                    # must stay warning-free
```

Running it needs a real TTY (it refuses otherwise):
`PATH=$PWD/target/release:$PATH git timeline <file>` from inside any git repo. To exercise
the UI non-interactively, drive it in tmux (`tmux new-session -d … ; tmux send-keys …;
tmux capture-pane -p`). Use `-h`, not `--help`: git intercepts `git timeline --help` to
look for a man page.

Integration tests build throwaway repos (`tests/common/mod.rs`, `Fixture`) with
`GIT_CONFIG_GLOBAL=/dev/null`, and pass that env to the code under test via
`Git::with_env` / `RepoContext::discover_with`. `tests/views.rs` drives the reducer
end-to-end without threads or a terminal: its `Harness` runs jobs synchronously with
`worker::run_inline` and renders with ratatui's `TestBackend`.

## Architecture (big picture)

Library crate `git_timeline` (all modules, so `tests/` can use them) + thin `main.rs`.

Layering, strictly one-way:
- `model/`: pure types and algorithms (no IO): timeline entries, content, in-process diff
  (`similar`), blame data, logical line mapping.
- `git/`: the **only** code that spawns `git` (always `git -C <toplevel>`, stderr captured,
  `GIT_OPTIONAL_LOCKS=0`, `-z` output). Returns `model/` types.
- `worker/`: thread pool running `Job`s (git reads, diffs, highlighting) off the main
  thread; each worker owns a persistent `git cat-file --batch-command` process.
- `app/`: main thread owns `AppState` and all caches. `keymap` maps events → `Action`,
  `update::update` is an IO-free reducer returning `Effect`s, `App::run` executes them.
- `ui/`: pure render of `&AppState`; `ui::draw` returns `LayoutInfo`, fed back through
  `Action::Layout` (the UI never mutates state).
- `ops/`: clipboard / export / restore side effects (atomic writes; never touch index/HEAD).

Key concepts that span files:
- **A timeline row carries both versions.** `TimelineEntry` has `blob` (the version at
  that revision) and `prev_blob` (the version just before it, i.e. the old blob from
  `git log --raw`). So "diff vs previous" is exact even when the list is filtered by
  author, `-L` or range. `BlobSource` says where bytes come from; `ContentKey` is the cache
  key (content-addressed; working-tree keys include mtime+len).
- **No job tickets.** Every job result is cached. `update::after_batch` runs once per batch
  of events and re-derives what the current view still needs (`want_content`,
  `want_diff`, …, deduped by `in_flight`). That gives staleness handling and key-repeat
  debouncing. Only the timeline stream has a generation counter (for reload).
- **`AppState::displayed()`** resolves the current view from the caches into a `Displayed`
  enum. Both the renderer and the reducer use it (row counts, cursor ↔ source-line mapping).
- **Logical line anchoring**: when the selection or view changes, a `LineAnchor` (content
  key + line) is captured. It is resolved once the new content is loaded, through a diff
  computed between the two versions directly (`model::line_map`).
- **Pseudo-entries**: `Working tree` / `Staged` rows come from `git/status.rs`, are read
  synchronously at startup (so they are on top and selected first), and are hidden when a
  revision range is given.

## Verified pitfalls (don't regress)

- `git log --follow` silently **drops merge commits**; `--diff-merges=first-parent` is
  required (not used in `-L` mode, which follows renames itself and emits no `R` records).
- The input thread must start **after** terminal setup (`App::run`), otherwise it swallows
  the terminal's reply to ratatui's cursor-position query.
- git runs external subcommands in the caller's cwd without `GIT_PREFIX`; paths are resolved
  against cwd in `git/repo.rs`.
- The panic hook ignores `worker-*` threads: worker panics are caught and reported in the
  UI and must not tear down the terminal.
