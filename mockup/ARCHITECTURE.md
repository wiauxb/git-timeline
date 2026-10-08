# git-timeline — Architecture overview

> **Rule:** this mockup is where the project is designed. Every architecture / code
> organisation change is first written here and agreed with the user, then implemented;
> evaluation findings come back here as new proposals (mockup → agree → code → evaluate →
> mockup …). The mockup always describes the current code.
>
> This file is the index of the mockup. Every other `.md` in `mockup/` mirrors one file of
> the crate (`mockup/src/git/log.rs.md` ⇢ `src/git/log.rs`).

## 1. Goal

`git timeline <path>` opens a terminal UI that lets you walk through every version of
one file in its git history. Selecting a revision shows that file **as it was** at that
revision, plus a few alternative views.

## 2. Decisions

| Topic | Decision |
|---|---|
| Language / stack | Rust, `ratatui` 0.30 (its re-exported `crossterm` backend) |
| Layout | Two panes: **timeline (revisions)** on the left, **file view** on the right |
| Views | `Content` (default), `Diff vs previous`, `Diff vs marked` (A↔B), `Blame` |
| Extras | Follow renames, syntax highlighting, actions (copy hash, export, restore) |
| Git access | Shell out to the `git` CLI (no libgit2 / gix) |
| Scroll when changing revision | Keep the **same logical line** (mapped through a diff) |
| Uncommitted changes | `Working tree` and `Staged` pseudo-entries above HEAD |
| CLI options | revision range, `-L <start>,<end>`, `--author`, `--since`, `--until` |
| Diffs | **Computed in-process** with `similar` (Myers): git *fetches*, Rust *compares*. Same diff reused for line mapping; works identically for commits / index / working tree |
| "Diff vs previous" | The change introduced by that revision: old blob from `git log --raw` (first parent for merges), not "the previous row". Correct even when the list is filtered |
| Merges | `git log --diff-merges=first-parent` so merges that produce a new version appear (see §8) |
| Concurrency | No async runtime: one input thread, 2-4 worker threads, main thread owns state and draws. `crossbeam-channel` |
| Caches | Live on the main thread; workers are stateless except one persistent `git cat-file --batch-command` each |
| Staleness | No per-job tickets: every result is cached, and `update::after_batch` re-derives what the current view still needs once per batch of events (gives debouncing of held keys for free). Only the timeline stream has a generation counter (reload) |
| Cross-thread errors | Carried as `String` in `JobResult` |
| Startup | Local state (`git status`) read **synchronously** before the TUI so the `Working tree` row is on top and selected before commits stream in |
| Search | `/` searches the **focused pane**: the timeline (hash prefix + subject) or the file view (displayed lines, every view). Regex, smart case, incremental. It jumps and highlights, never filters. `n`/`N` next/previous match, wrapping. One search per pane, kept across revision changes |
| Minimum git | **2.36** (`cat-file --batch-command`), checked at startup |

## 3. File tree

```
git-timeline/
├── CLAUDE.md                     guidance for Claude Code sessions (rules, commands, architecture)
├── Cargo.toml                    crate manifest: lib `git_timeline` + bin `git-timeline`
├── README.md                     user documentation
├── TODO.md                       backlog of likely bugs and open verifications
├── src/
│   ├── main.rs                   thin binary: parse CLI, call lib::run, exit code
│   ├── lib.rs                    library root (all modules; lets tests/ use them) + run()
│   ├── cli.rs                    clap definition of arguments/options
│   ├── error.rs                  error enum + user-facing messages
│   ├── terminal.rs               raw mode / alt screen / mouse / paste; restore; panic hook
│   ├── highlight.rs              syntect-based syntax highlighting
│   ├── git/
│   │   ├── mod.rs                `Git` command runner (output, probe, stdin, stream)
│   │   ├── repo.rs               `RepoContext`: toplevel, path resolution, validation
│   │   ├── log.rs                timeline from `git log --follow --raw -z`, streamed
│   │   ├── status.rs             staged / unstaged changes → pseudo-entries
│   │   ├── blob.rs               file content (object db via cat-file batch / working tree)
│   │   ├── blame.rs              `git blame --porcelain` parsing
│   │   └── show.rs               full commit message + changed files on demand
│   ├── model/
│   │   ├── mod.rs                re-exports
│   │   ├── path.rs               `RepoPath` (byte-exact repo-relative path)
│   │   ├── revision.rs           `Oid`, `RevisionId`, `TimelineEntry`, `ChangeKind`…
│   │   ├── search.rs             `Query`: smart-case regex, hash-prefix matching
│   │   ├── content.rs            `FileContent`, `TextContent`, binary detection
│   │   ├── diff.rs               in-process diff → hunks → display rows
│   │   ├── blame.rs              `BlameData`, `BlameLine`, `BlameKey`
│   │   └── line_map.rs           logical line mapping between two versions
│   ├── app/
│   │   ├── mod.rs                `App`: event loop, effect execution, input thread
│   │   ├── state.rs              `AppState` + `Displayed` (what the file pane shows)
│   │   ├── action.rs             `Action` (intents), `Effect` (side effects), `OpResult`
│   │   ├── update.rs             reducer + `after_batch` (requests, re-anchoring, prefetch)
│   │   ├── keymap.rs             key/mouse event → `Action`; `BINDINGS` table for help
│   │   ├── search.rs             `/` search: per-pane query, prompt flow, hit lists, n/N
│   │   └── cache.rs              LRU caches for content / highlight / diff / blame / details
│   ├── worker/
│   │   ├── mod.rs                worker pool, `Job` / `JobResult`, `run_inline` (tests)
│   │   └── jobs.rs               implementation of each job kind
│   ├── ops/
│   │   ├── mod.rs                `atomic_write` shared by export/restore
│   │   ├── clipboard.rs          copy text (arboard, fallback OSC 52)
│   │   ├── export.rs             write a version to a file
│   │   └── restore.rs            write a version into the working tree
│   └── ui/
│       ├── mod.rs                root `draw()` → `LayoutInfo`, helpers (age, fit, centered)
│       ├── theme.rs              ANSI-color styles, NO_COLOR monochrome
│       ├── timeline_pane.rs      left pane: list of revisions
│       ├── file_pane.rs          right pane frame, header, placeholders, dispatch
│       ├── content_view.rs       full content + shared line renderer (`styled_line`)
│       ├── diff_view.rs          unified diff (prev or marked) + non-text summary
│       ├── blame_view.rs         blame gutter + content
│       ├── status_bar.rs         bottom line: focus, view tabs, hints/messages, position
│       ├── help.rs               `?` overlay generated from `BINDINGS`
│       └── dialog.rs             restore / export / overwrite / details / error popups
└── tests/
    ├── common/mod.rs             fixture repository builder + scenarios
    ├── timeline.rs               history building: log, renames, -L, filters, merges, pseudo-entries, errors
    └── views.rs                  reducer + views driven end-to-end (inline jobs, TestBackend)
```

## 4. Layering

```
          ┌───────────┐
          │ main/lib  │  cli → repo discovery → local state → terminal → App::run
          └─────┬─────┘
                │
        ┌───────▼────────┐  AppEvent   ┌───────────────┐
        │   app (main    │◄────────────┤ input thread  │ crossterm events
        │   thread)      │◄────────────┤ worker pool   │ JobResult
        │ state/update/  │────────────►│ (worker/)     │ Job (High / Low prio)
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
* `app/` owns state; it never does blocking IO itself (except small fs writes in `ops/`
  and the one synchronous `git status` at startup).
* `ui/` is a pure function of `&AppState` → frame; it returns `LayoutInfo`, which the app
  feeds back as `Action::Layout`.

## 5. Main data flow

1. `main` parses the CLI → `lib::run` → `RepoContext::discover` resolves the toplevel and
   repo-relative path, checks git version, validates range / `-L`, and checks the path has
   history or exists locally (else exits **before** the TUI). Then refuses non-TTYs.
2. `App::new` reads local state synchronously (pseudo-entries, selects row 0), spawns the
   worker pool. The terminal is entered, then `App::run` starts the **input thread**
   (started earlier it would swallow the terminal's reply to ratatui's cursor query) and
   submits `Job::LoadTimeline { gen }`.
3. The timeline arrives **streamed** (`TimelineChunk`, ≤ 256 entries / 50 ms).
4. Loop: wait for an event (120 ms tick), drain all pending ones, run each through
   `keymap::map` → `update::update`; then `update::after_batch` computes what the current
   view needs (contents, highlights, diff, blame) and submits missing jobs (High), resolves
   the pending line anchor, and prefetches neighbours ±2 (Low). Redraw if dirty.
5. On revision/view change the reducer captures a `LineAnchor` (content key + line +
   screen offset); once the new version is loaded, `line_map` translates it through a diff
   between the two versions (requested if not cached) and the cursor lands on the same code.

## 6. Key bindings (authoritative list: `app/keymap.rs`)

| Key | Action |
|---|---|
| `j`/`k`, `↓`/`↑` | move in the focused pane |
| `Tab` | switch focus timeline ⇄ file |
| `[` / `]` | older / newer revision from either pane |
| `1` `2` `3` `4` (or `F1`-`F4`) | view: content / diff vs previous / diff vs marked / blame |
| `m` | mark / unmark the selected revision as compare base |
| `/` | search the focused pane (regex, smart case, live) |
| `n` / `N` | next / previous search match (wraps) |
| `Esc` | clear the focused pane's search (file pane without search: focus timeline) |
| `}` / `{` | next / previous hunk (diff views) |
| `Enter` (blame view) | jump to the revision that last touched the line |
| `#` | toggle line numbers |
| `z` | zoom the file pane |
| `d` | commit details popup |
| `y` | copy full hash |
| `e` | export this version to a file |
| `R` | restore this version into the working tree (confirm with `y`) |
| `r` | reload |
| `?` | help overlay |
| `q`, `Ctrl-C` | quit |

## 7. Git command catalogue

All commands: `git -C <dir> --no-pager -c color.ui=never -c core.quotepath=off -c log.showSignature=false …`
with `GIT_TERMINAL_PROMPT=0 GIT_OPTIONAL_LOCKS=0 LC_ALL=C`, pager env removed.

| Purpose | Command | File |
|---|---|---|
| version | `git --version` | `git/repo.rs` |
| toplevel | `git rev-parse --show-toplevel` (in cwd) | `git/repo.rs` |
| range check | `git rev-list --max-count=1 <range> --` / `git rev-parse --verify -q <rev>^{commit}` | `git/repo.rs` |
| unborn HEAD | `git rev-parse --verify -q HEAD` | `git/repo.rs` |
| timeline | `git log --follow --diff-merges=first-parent --raw --no-abbrev -z --format=<F> [filters] [range] -- :(literal)<path>` | `git/log.rs` |
| timeline `-L` | `git log -L<a>,<b>:<path> --no-patch --raw --no-abbrev -z --format=<F> [filters] [range]` | `git/log.rs` |
| local state | `git status --porcelain=v2 -z --no-renames --untracked-files=all -- <pathspec>` | `git/status.rs` |
| index blob | `git ls-files -s -z -- <pathspec>` | `git/status.rs` |
| HEAD blob | `git ls-tree -z HEAD -- <path>` | `git/status.rs` |
| blob content | `git cat-file --batch-command --buffer` (persistent: `info` / `contents` + `flush`) | `git/blob.rs` |
| merge fallback | `git ls-tree -z <commit> -- <path>` | `git/blob.rs` |
| blame | `git blame --porcelain [--contents -] [<oid>] -- <path>` | `git/blame.rs` |
| details | `git show -s --format=%B%x00%D <oid>` | `git/show.rs` |
| files of a commit | `git diff-tree --no-commit-id -r --name-status -z -M --root <oid>` | `git/show.rs` |

## 8. Verified git behaviors (git 2.55)

* git runs an external `git-<name>` **in the caller's cwd** and sets **no `GIT_PREFIX`**;
  the path argument is resolved against `current_dir()`.
* `git timeline --help` is turned into `git help timeline` (man page lookup); `-h` is
  passed through to the binary.
* `git log --follow` **drops merge commits**, even a merge producing a new version of the
  file (the newest row would then not match `HEAD`). `--diff-merges=first-parent` makes
  them appear with one raw record against the first parent.
* `git log -L` follows lines across renames by itself, **skips rename-only commits** and
  emits **no `R` records** (older commits just report the old path). It forbids a pathspec.
* `--raw --no-abbrev -z` output: header token `\x1e…` (ended by NUL), then
  `\n:<m1> <m2> <o1> <o2> <status>\0<path>\0[<path2>\0]`.
* `cat-file --batch-command --buffer`: `info <oid>\nflush\n` → `<oid> <type> <size>\n` or
  `<oid> missing\n`; `contents <oid>\nflush\n` → header line + `<size>` bytes + `\n`.
* Blob-to-blob diff works (`git diff <blob> <blob>`); not used, since diffs are in-process.

## 9. Out of scope for now

* Side-by-side diff, word-level diff highlighting.
* Soft wrap (`w`): horizontal scrolling only.
* Filtering the commit list from inside the UI (search jumps, never hides rows);
  searching full commit messages or authors.
* Directories (only single files).
* Config file; configuration is via CLI flags / env only.
* `GIT_TIMELINE_LOG` debug logging.
* Applying `.gitattributes` filters (CRLF, LFS, smudge) to working-tree content.
* Suspend (Ctrl-Z); man page generation.
