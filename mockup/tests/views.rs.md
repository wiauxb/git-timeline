# `tests/views.rs`

## Purpose
Drive the reducer and views end-to-end without threads or a terminal: jobs are executed
synchronously with `worker::run_inline`, rendering uses ratatui's `TestBackend`.

## Harness

```rust
struct Harness { s: AppState, ctx: Arc<RepoContext>, effects: Vec<Effect> }
impl Harness {
    /// Context from the fixture, AppState (highlighting off), fixed LayoutInfo
    /// (file body 20 rows × 78 cols); runs LoadLocalState + LoadTimeline{gen 0}; settles.
    fn new(f: &Fixture, args: &[&str]) -> Self;
    fn feed(&mut self, job: Job);              // run_inline → update(JobDone) for each result
    fn run_effects(&mut self, fx: Vec<Effect>);// Submit ⇒ feed; others collected in `effects`
    fn settle(&mut self);                      // loop after_batch until nothing is requested
    fn act(&mut self, a: Action);              // update + effects + settle
    fn render(&self) -> String;                // ui::draw on a 120×30 TestBackend → text
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
| `search_horizontal_reveal` | a match at column 200 sets `hscroll = 192` and is visible in the render |
| `search_invalid_regex_and_no_match` | `(` shows `invalid regex` and Enter keeps the prompt; unknown word → Enter closes with Warn "pattern not found" |
| `search_renders_highlights` | TestBackend: prompt in the status bar, `k/M` counter, matched cells carry the `search_match` style |
| `hunk_keys_moved` | keymap: `}` / `{` → hunks in diff views, `n` / `N` / `/` → search; `n` without a search → "no search — press / to search" |
| `stale_results_are_cached_but_harmless` | a content result for a far entry is cached and does not move the cursor |
