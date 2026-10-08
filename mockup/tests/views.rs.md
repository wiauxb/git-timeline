# `tests/views.rs`

## Purpose
Drive the reducer and views end-to-end without threads or a terminal: jobs are executed
synchronously with `worker::run_inline`, rendering uses ratatui's `TestBackend`.

## Harness

```rust
struct Harness { s: AppState, ctx: Arc<RepoContext>, effects: Vec<Effect>, size: (u16, u16) }
impl Harness {
    /// Context from the fixture, AppState (highlighting off), fixed LayoutInfo
    /// (file body 20 rows × 78 cols); runs LoadLocalState + LoadTimeline{gen 0}; settles.
    fn new(f: &Fixture, args: &[&str]) -> Self;
    fn feed(&mut self, job: Job);              // run_inline → update(JobDone) for each result
    fn run_effects(&mut self, fx: Vec<Effect>);// Submit ⇒ feed; others collected in `effects`
    fn settle(&mut self);                      // loop after_batch until nothing is requested
    fn act(&mut self, a: Action);              // update + effects + settle
    fn render(&self) -> String;                // ui::draw on a `size` TestBackend (120×30) → text
    fn buffer(&self) -> Buffer;                // same, the raw buffer (styles)
    fn resize(&mut self, w: u16, h: u16);      // draw at w×h and feed the Layout back
    fn type_str(&mut self, text: &str);        // InputChar per char
    fn hit_rows(&self) -> Vec<usize>;          // rows of the file search's matches
    fn status(&self) -> String;                // status message text
}
fn numbered(range) -> String;                  // "line i\n" lines
```

## Tests

| Test | Asserts |
|---|---|
| `anchor_follows_code` | 10 lines prepended at HEAD: cursor on row 45 ("line 35") → `SelectNext` → cursor 35; back → 45 |
| `anchor_same_blob_keeps_position` | across the R100 rename (same blob) cursor and top are unchanged |
| `diff_prev_matches_git_numstat` | for every commit of `renamed_history`, `+/−` counts equal `git show --numstat -M` |
| `diff_marked_is_ordered_old_to_new` | mark newest, select oldest, Compare → `+4 −0` |
| `blame_attributes_lines` | each line attributed to the commit that added it; `BlameJump` on line 2 selects that commit |
| `blame_working_tree_and_index` | worktree line has null oid; Staged row blame has 3 lines, staged line null oid |
| `restore_flow` | oldest version (old name) → `ConfirmRestore` → `Confirm` emits `Effect::Restore` to the **current** path with old bytes; after the op, file content restored, `WorkingTree` row appears, index untouched |
| `restore_refused_for_working_tree_and_deleted` | `StartRestore` opens no overlay on the WorkingTree row or a deleted-file row |
| `restore_sets_exec_bit` (unix) | `ops::restore` sets / clears exec bits from `FileMode` |
| `export_flow_and_default_name` | prompt prefilled `a@<short>.txt`; `Confirm` → `Effect::Export` into `invocation_dir`, `overwrite: false` |
| `content_reader_matches_git_show` | `BlobReader::read` of each entry matches `git show <oid>:<path>`; `Reload` rebuilds 4 entries |
| `binary_symlink_too_large` | NUL ⇒ `Binary`; size over a lowered limit ⇒ `TooLarge`; symlink ⇒ `Symlink { target }` |
| `renders_every_view` | TestBackend: Working tree row, subjects, content; diff (`+ five`, `@@`); blame (`not committed yet`); Compare without mark (`press m`); details popup; help (`Keys`); zoom hides the timeline |
| `search_timeline_hash_and_subject` | `/` in the timeline: a subject word selects that commit live; a hash prefix selects it; `n` wraps with "search wrapped to top"; `Cancel` restores selection and previous query |
| `search_file_all_views` | file search in Content jumps to the matching line; in DiffPrev it lands on the `+` row, skips hunk headers and searches removed rows on the old side; in Blame it ignores the gutter (author name not found); selecting another revision keeps the query and recomputes hits without moving the anchored cursor |
| `search_steps_through_one_line` | one 600-char line with "needle" at 3 places: wrap off ⇒ `n` ×3 keeps the cursor on that line and moves `hscroll` to each match, counter `1/3` `2/3` `3/3`, 4th `n` wraps; wrap on with a 5-row body ⇒ each `n` sets `top_skip` so the match's segment is on screen, and the current match has `search_current`; `j` then resets `top_skip` and the counter shows `-/3` |
| `search_horizontal_reveal` | a match at column 200 sets `hscroll = 192` and is visible in the render |
| `search_invalid_regex_and_no_match` | `(` shows `invalid regex` and Enter keeps the prompt; unknown word → Enter closes with Warn "pattern not found" |
| `search_renders_highlights` | TestBackend: prompt in the status bar, `k/M` counter, matched cells carry the `search_match` style |
| `hunk_keys_moved` | keymap: `}` / `{` → hunks in diff views, `n` / `N` / `/` → search; `n` without a search → "no search — press / to search" |
| `wrap_renders_continuations` | a 200-char line in a 78-col body: wrap off ⇒ one row, clipped; `w` ⇒ text split at spaces over 3 rows, `↪` on rows 2–3, the next line's number below; in DiffPrev the `+` row keeps the tint on its continuation rows |
| `wrap_viewport_math` | 3-row lines: `j` moves one logical line; `CursorDown` past the screen keeps the whole cursor line visible; `PageDown` covers one screen of screen rows; `ClickFileRow` on a continuation row selects its line; `ScrollRight` does nothing (and works again once wrap is off) |
| `wrap_anchor_keeps_screen_offset` | HEAD: cursor 3 screen rows (3 short lines) below `top`; `SelectNext` to the version where those lines are long ⇒ `top` is 1 line above the cursor: same **screen** offset, not the same number of lines |
| `stale_results_are_cached_but_harmless` | a content result for a far entry is cached and does not move the cursor |
