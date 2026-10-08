# `CLAUDE.md`

## Purpose
Guidance for Claude Code sessions in this repository: workflow rules, commands, and the
big-picture architecture that takes several files to understand.

## Contents
* **Project**: one-paragraph summary, git ≥ 2.36 requirement.
* **Mandatory workflow rules**: mockup-first loop (mockup → user agreement → code →
  evaluate → mockup …); the mockup always describes the current code; `TODO.md` as the
  bug backlog, closed through the same loop.
* **Commands**: build, release build, all tests, one module's unit tests, one integration
  test, clippy; running in a TTY / via tmux; `-h` vs `--help`; how the test fixtures and the
  `views.rs` harness work.
* **Architecture**: layering (`model` → `git` → `worker` → `app` → `ui`, plus `ops`) and the
  cross-file concepts: `blob`/`prev_blob`, `BlobSource`/`ContentKey`, no tickets +
  `after_batch`, `Displayed`, line anchoring, pseudo-entries.
* **Verified pitfalls**: `--follow` drops merges → `--diff-merges=first-parent`; input
  thread starts after terminal setup; cwd / `GIT_PREFIX`; panic hook ignores workers.

## Rules
Kept short and non-duplicative: file-level detail belongs in the mockup specs, not here.
Update it when commands, workflow rules, or the cross-cutting architecture change.
