# `tests/views.rs`

## Purpose
Integration tests of what the views are built from: content reading, diff, blame,
logical line mapping, and the reducer's behavior on selection changes — without a
terminal.

## Test cases

| Test | Asserts |
|---|---|
| `content_at_each_commit` | `BlobReader::read` of each entry's blob equals `git show <oid>:<path>` |
| `blob_dedup` | two commits with identical file content ⇒ same `ContentKey` |
| `binary_detection` | NUL in first 8000 bytes ⇒ `Binary`; NUL at byte 9000 ⇒ `Text` |
| `too_large` | blob above a test-lowered limit ⇒ `TooLarge` without reading it |
| `symlink_and_submodule` | `Symlink{target}`; gitlink ⇒ `Submodule{commit}` |
| `worktree_key_changes_on_edit` | rewrite file ⇒ new `ContentKey::WorkingTree` |
| `diff_prev_matches_git` | for every entry, `+`/`−` counts of `diff::compute(prev, blob)` equal `git diff --numstat <prev-blob> <blob>` |
| `diff_marked_order` | mark newer, select older ⇒ diff still old→new |
| `blame_commit` | blame of entry N attributes each line to the expected fixture commit |
| `blame_index` | staged change ⇒ lines with null oid |
| `blame_after_rename` | blame on the renamed path at a pre-rename commit uses the old path |
| `line_map_*` | (pure) unchanged, deleted block, replaced block, insert before, EOF, empty |
| `anchor_follows_code` | reducer: cursor on `fn foo` line 40 at HEAD; select HEAD~1 where 10 lines were inserted above ⇒ cursor line 50 after results are fed |
| `anchor_same_blob` | selecting a commit where the file did not change keeps cursor & top exactly |
| `stale_results` | results with an old ticket are cached but do not move the cursor |
| `restore_flow` | `StartRestore` → `ConfirmRestore` overlay; `Confirm` emits `Effect::Restore` targeting the current path; refused for `WorkingTree` / `Absent` |
| `restore_writes_mode` | `ops::restore` sets exec bit from `Executable`, file bytes equal blob |
| `export_default_name` | `main@abc1234.rs`, `Makefile@abc1234`, `.env@abc1234` (no ext confusion) |
