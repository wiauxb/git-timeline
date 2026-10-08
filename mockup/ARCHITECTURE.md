# git-timeline — Architecture overview

> This file is the index of the mockup. It is *not* a file of the final project.
> Every other `.md` in `mockup/` mirrors one file of the future crate
> (`mockup/src/git/log.rs.md` ⇢ `src/git/log.rs`).

## 1. Goal

`git timeline <path>` opens a terminal UI that lets you walk through every version of
one file in its git history. Selecting a revision shows that file **as it was** at that
revision, plus a few alternative views.

## 2. Decisions taken (from the Q&A)

| Topic | Decision |
|---|---|
| Language / stack | Rust, `ratatui` + `crossterm` |
| Layout | Two panes: **timeline (commits)** on the left, **file view** on the right |
| Views | `Content` (default), `Diff vs previous`, `Diff vs marked` (A↔B), `Blame` |
| Extras | Follow renames, syntax highlighting, actions (copy hash, export, restore) |
| Git access | Shell out to the `git` CLI (no libgit2 / gix) |
| Scroll when changing revision | Keep the **same logical line** (mapped through a diff) |
| Uncommitted changes | `Working tree` and `Staged` pseudo-entries above HEAD |
| CLI options | revision range, `-L <start>,<end>`, `--author`, `--since`, `--until` |

Decisions taken by the architect (can be revisited):

* **Diffs are computed in-process** with the `similar` crate, not with `git diff`.
  Reason: we already hold both blob contents (cached), it works identically for
  commits / index / working tree, and the same diff is reused for logical line mapping.
  `git` is used to *fetch* data, Rust code *compares* it.
* **"Diff vs previous" = the change introduced by that revision**, using the *old blob*
  reported by `git log --raw`, not "the previous row of the list". This stays correct
  when the list is filtered (`--author`, `-L`, range).
* **No async runtime.** Plain threads + `crossbeam-channel`. One input thread, N worker
  threads, the main thread owns all state and draws.
* **Caches live on the main thread**; workers are stateless (except a persistent
  `git cat-file --batch-command` process each).
* Minimum git version: **2.36** (`cat-file --batch-command`), checked at startup.

## 3. File tree

```
git-timeline/
├── Cargo.toml                    crate manifest, binary named `git-timeline`
├── README.md                     user documentation
├── src/
│   ├── main.rs                   thin binary: parse CLI, call lib::run, exit code
│   ├── lib.rs                    library root (all modules; lets tests/ import them)
│   ├── cli.rs                    clap definition of arguments/options
│   ├── error.rs                  error types + user-facing messages
│   ├── terminal.rs               raw mode / alt screen setup, restore, panic hook
│   ├── highlight.rs              syntect-based syntax highlighting
│   ├── git/
│   │   ├── mod.rs                `Git` command runner (spawn, capture, stream)
│   │   ├── repo.rs               `RepoContext`: toplevel, path resolution, version check
│   │   ├── log.rs                build the timeline from `git log --follow --raw -z`
│   │   ├── status.rs             detect staged / unstaged changes → pseudo-entries
│   │   ├── blob.rs               read file content (object db / index / working tree)
│   │   ├── blame.rs              `git blame --porcelain` parsing
│   │   └── show.rs               full commit message / details on demand
│   ├── model/
│   │   ├── mod.rs                re-exports
│   │   ├── path.rs               `RepoPath` (byte-exact repo-relative path)
│   │   ├── revision.rs           `Oid`, `RevisionId`, `TimelineEntry`, `ChangeKind`…
│   │   ├── content.rs            `FileContent`, `TextContent`, binary/size detection
│   │   ├── diff.rs               in-process diff → hunks
│   │   ├── blame.rs              `BlameData`, `BlameLine`
│   │   └── line_map.rs           logical line mapping between two versions
│   ├── app/
│   │   ├── mod.rs                `App`: event loop, effect execution
│   │   ├── state.rs              `AppState` (the single source of truth)
│   │   ├── action.rs             `Action` (user intents) and `Effect` (side effects)
│   │   ├── update.rs             reducer: (state, action | job result) → effects
│   │   ├── keymap.rs             key/mouse event → `Action`, context-aware
│   │   └── cache.rs              LRU caches for content / diff / blame / highlight
│   ├── worker/
│   │   ├── mod.rs                worker pool, `Job` / `JobResult`, staleness tickets
│   │   └── jobs.rs               implementation of each job kind
│   ├── ops/
│   │   ├── mod.rs                re-exports
│   │   ├── clipboard.rs          copy text (arboard, fallback OSC 52)
│   │   ├── export.rs             write a version to a file
│   │   └── restore.rs            write a version into the working tree
│   └── ui/
│       ├── mod.rs                root `draw()`, layout, overlay stacking
│       ├── theme.rs              colors and styles, NO_COLOR support
│       ├── timeline_pane.rs      left pane: list of revisions
│       ├── file_pane.rs          right pane frame + header, dispatch to view renderer
│       ├── content_view.rs       render full file content
│       ├── diff_view.rs          render unified diff (prev or marked)
│       ├── blame_view.rs         render blame gutter + content
│       ├── status_bar.rs         bottom line: mode, hints, messages
│       ├── help.rs               `?` overlay listing key bindings
│       └── dialog.rs             confirm / text-input / details / error popups
└── tests/
    ├── common/mod.rs             fixture repository builder
    ├── timeline.rs               integration tests: log, renames, -L, filters, pseudo-entries
    ├── views.rs                  integration tests: content, diff, blame, line mapping
    └── ui_snapshots.rs           ratatui `TestBackend` snapshot tests
```

## 4. Layering

```
          ┌───────────┐
          │ main/lib  │  cli → repo discovery → terminal → App::run
          └─────┬─────┘
                │
        ┌───────▼────────┐  AppEvent   ┌───────────────┐
        │   app (main    │◄────────────┤ input thread  │ crossterm events
        │   thread)      │◄────────────┤ worker pool   │ JobResult
        │ state/update/  │────────────►│ (worker/)     │ Job
        │ keymap/cache   │             └──────┬────────┘
        └───────┬────────┘                    │ uses
                │ reads state                 ▼
        ┌───────▼────────┐             ┌───────────────┐
        │      ui/       │             │ git/ + model/ │
        │ (pure render)  │             │ + highlight   │
        └────────────────┘             └───────────────┘
```

Rules:
* `model/` is pure (no IO, no git, no ratatui). Fully unit-testable.
* `git/` does IO via the `git` binary and returns `model/` types. No UI knowledge.
* `worker/` turns `Job`s into `git/` + `model/` + `highlight` calls.
* `app/` owns state; it never does blocking IO itself (except tiny fs writes in `ops/`).
* `ui/` is a pure function of `&AppState` → frame. It never mutates state.

## 5. Main data flow

1. `main` parses CLI → `RepoContext::discover` resolves the toplevel and the
   repo-relative path, checks git version, and checks that the path has any history
   or local existence (else exits **before** entering the TUI).
2. `App::run` enters the alternate screen, posts `Job::LoadTimeline` and
   `Job::LoadLocalState`.
3. The timeline arrives **streamed** in chunks (`JobResult::TimelineChunk`), so the list
   is usable within milliseconds even on 50k-commit histories.
4. Selecting an entry → `update` checks caches; missing pieces are requested
   (`Job::LoadContent`, `Job::Highlight`, `Job::ComputeDiff`, `Job::LoadBlame`).
   Each job carries a **ticket** (generation counter); results for a selection that is no
   longer current are cached but don't trigger redraw logic.
5. When the new content is available, the **logical line anchor** is translated through
   `line_map` from the previous version to the new one, and the view scrolls there.
6. Neighbours (±2 entries) are prefetched at low priority.

## 6. Key bindings (summary — authoritative list in `app/keymap.rs.md`)

| Key | Action |
|---|---|
| `j`/`k`, `↓`/`↑` | move in the focused pane |
| `Tab` | switch focus timeline ⇄ file |
| `[` / `]` | previous / next revision while keeping file focus |
| `1` `2` `3` `4` | view: content / diff vs previous / diff vs marked / blame |
| `m` | mark / unmark the selected revision as compare base |
| `n` / `N` | next / previous hunk (diff views) |
| `Enter` (blame view) | jump to the revision that last touched the line |
| `z` | zoom the file pane (hide timeline) |
| `d` | commit details popup (full message, author, committer, parents) |
| `y` | copy full hash |
| `e` | export this version to a file |
| `R` | restore this version into the working tree (with confirmation) |
| `r` | reload (re-run status + log) |
| `?` | help overlay |
| `q`, `Ctrl-C` | quit |

## 7. Git command catalogue

| Purpose | Command (always prefixed by `git -C <toplevel> --no-pager -c color.ui=never`) | File |
|---|---|---|
| version | `git --version` | `git/repo.rs` |
| toplevel | `git rev-parse --show-toplevel` (run in cwd) | `git/repo.rs` |
| timeline | `git log --follow --raw --no-abbrev -z --format=<F> [range] [filters] -- <path>` | `git/log.rs` |
| timeline `-L` | `git log -L<a>,<b>:<path> --no-patch --raw --no-abbrev -z --format=<F> [range] [filters]` | `git/log.rs` |
| local state | `git status --porcelain=v2 -z --no-renames --untracked-files=all -- <path>` | `git/status.rs` |
| index blob | `git ls-files -s -z -- <path>` | `git/status.rs` |
| HEAD blob | `git ls-tree -z HEAD -- <path>` | `git/status.rs` |
| blob content | `git cat-file --batch-command` (persistent: `info <oid>` / `contents <oid>`) | `git/blob.rs` |
| merge fallback | `git rev-parse --verify -q <commit>:<path>` | `git/blob.rs` |
| blame | `git blame --porcelain [--contents -] [<oid>] -- <path>` | `git/blame.rs` |
| details | `git show -s --format=%B%x00%D <oid>` | `git/show.rs` |
| files of a commit | `git diff-tree --no-commit-id -r --name-status -z -M --root <oid>` | `git/show.rs` |

## 8. Out of scope for v1 (explicitly)

* Side-by-side diff, word-level diff highlighting.
* Search inside the file / filtering the commit list from inside the UI.
* Directories (only single files).
* Config file; all configuration is via CLI flags / env.
* Applying `.gitattributes` filters (CRLF, smudge) to working-tree content (see
  `git/blob.rs.md` → "Known limitation").

## 9. As implemented — differences from this mockup

The crate in the repository root follows this mockup. Where reality differed, the code
wins; the differences are:

* **Merges need `--diff-merges=first-parent`.** Verified on git 2.55: `git log --follow`
  silently omits merge commits, even when the merge produced a new version of the file
  (the newest row would then not match `HEAD`). With the flag, merges come with one raw
  record against their first parent, so `prev_blob` stays exact. `-L` mode is unchanged.
* **No tickets.** Instead of tagging jobs with a generation ticket, every result is
  cached and `update::after_batch` re-derives what the current view still needs once per
  batch of events. That gives staleness handling and debouncing (holding `j`) for free.
  Only the timeline stream keeps a generation counter (for reload).
* **Errors cross threads as `String`**, not `Arc<Error>`.
* **`BlameKey::Local { rev, content }`** replaces `Index` / `WorkingTree` keys: local blames
  are keyed by the exact bytes being blamed.
* **Soft wrap (`w`) is not implemented**; horizontal scrolling only.
* **Local state is read synchronously at startup** (a few ms) so the `Working tree` row is
  on top and selected before the first commits stream in.
* **The input thread starts in `App::run`**, after the terminal is set up: started earlier
  it swallowed the terminal's answer to ratatui's cursor-position query.
* **`GIT_TIMELINE_LOG` debug logging is not implemented.**
* `worker::run_inline` (doc-hidden) runs a job synchronously; `tests/views.rs` uses it to
  drive the reducer end-to-end without threads or a terminal. `tests/ui_snapshots.rs` was
  folded into `tests/views.rs` (`renders_every_view`, `TestBackend`).
