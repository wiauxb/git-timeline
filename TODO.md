# TODO

Likely bugs found while resyncing `mockup/` with the code. None fixed yet.
Each fix should come with a test and the matching `mockup/` spec update.

## Worth fixing

- [ ] **Blame can spin forever** — `src/ui/blame_view.rs`, `src/app/update.rs`
  When `git blame` returns a different number of lines than the displayed text (file
  edited on disk between read and blame), the view treats it as "not loaded". The
  mismatched result stays cached and is never re-requested, so the gutter shows
  `blame…` forever. Expected: discard and re-request once.

- [ ] **A failed diff looks like "no changes"** — `src/worker/mod.rs` (`Job::failure`)
  If `ComputeDiff` panics, the failure result is an empty `FileDiff`, which is cached.
  The diff view then shows "no changes" instead of an error, and the pair is never retried.

- [ ] **Commit details list no files for merges** — `src/git/show.rs`
  `git diff-tree` without `-m` / `--first-parent` usually prints nothing for a merge
  commit, so the `d` popup shows "Files changed (0)".

- [ ] **Prefetch can delay the selected revision** — `src/app/update.rs` (`want_content`)
  A neighbour prefetched at Low priority is recorded in `in_flight`. If it then becomes the
  selection, the High-priority request is skipped as a duplicate, and its content waits
  behind the Low queue.

- [ ] **Restore dialog can hide its buttons** — `src/ui/dialog.rs` (`frame`)
  Popup height is computed from the logical line count, but the paragraph wraps: a long
  commit subject or path pushes the `[y] Yes  [n] No` line out of the popup.

## Minor

- [ ] **Blame error row hides the cursor** — `src/ui/blame_view.rs`
  The `blame failed: …` row shifts the text down one row, but cursor/page math still uses
  the full body height, so the cursor row can end up off-screen at the bottom.

- [ ] **Export to a directory asks "Overwrite?" first** — `src/ops/export.rs` (`write`)
  The "exists" check runs before the "is a directory" check, so the user confirms an
  overwrite and only then gets the error.

- [ ] **`F1`–`F4` missing from the help** — `src/app/keymap.rs` (`BINDINGS`)
  They switch views but are not listed in the help screen.

- [ ] **Tests fail instead of skipping without git ≥ 2.36** — `tests/common/mod.rs`
  `git_available()` exists but is never called.

- [ ] **`-L` not validated at startup in a repo with no commits** — `src/git/repo.rs`
  The history check is skipped when HEAD is unborn and no range is given; git's error
  still surfaces, as a fatal error from inside the TUI.

- [ ] **Fallback rows assume a regular file** — `src/git/log.rs` (`flush_entry`)
  Commits with no raw record get `mode = Regular` even for symlinks/submodules. Display is
  correct (the reader re-resolves the mode), but restore uses `entry.mode`. Practically
  unreachable since `--diff-merges=first-parent`.

- [ ] **Merge that changed the file type is labelled "Merge"** — `src/git/log.rs`
  The `parents > 1` check wins over the `T` status. Cosmetic.

## To verify

- [ ] **Does `-L` mode drop merges like `--follow` does?** — `src/git/log.rs` (`build_args`)
  `-L` gets no `--diff-merges=first-parent`. Needs a test with a merge touching the range.

- [ ] **`streaming_many_commits` doesn't prove multi-chunk streaming** — `tests/timeline.rs`
  It only asserts ≥ 1 chunk (byte-by-byte parsing is covered by unit tests in `src/git/log.rs`).
