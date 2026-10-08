# git-timeline

> [!WARNING]
> This is a **vibecoded** project, and it has quite a **low priority** in my schedule: expect rough edges.
> Nonetheless, any [issue](https://github.com/wiauxb/git-timeline/issues) or contribution is welcome!
> I'll do my best to respond within a reasonable time frame ;)

Browse every version of a file through its git history, in the terminal.

```
┌ Timeline ─ src/main.rs (6) ──────────┐┌ 5b089ca · Alice · 3 days ago · v3: bump ─ [Content] ────┐
│●Working tree   modified              ││src/main.rs · 4 lines · Rust                             │
│●Staged         modified              ││1 │ fn main() {                                          │
│ 5b089ca   3d   v3: bump              ││2 │     let x = 3;                                       │
│◆cbf9825   1w R move to src/main.rs   ││3 │     println!("v3 {x}");                              │
│ aa0c487   2w   v2: add x             ││4 │ }                                                    │
│ a6ac609   1mo A v1: hello            ││                                                         │
└──────────────────────────────────────┘└─────────────────────────────────────────────────────────┘
 TIMELINE   1 Content  2 Diff  3 Cmp  4 Blame  │ m mark · y copy · e export · R restore · ? help
```

## Install

```sh
cargo install --path .
```

This installs a `git-timeline` binary in `~/.cargo/bin`. As long as that directory is on
your `PATH`, git runs it as the subcommand `git timeline`. Requires **git ≥ 2.36**.

## Usage

```
git timeline [OPTIONS] <PATH> [REVISION_RANGE]

git timeline src/main.rs
git timeline src/main.rs v1.0..main
git timeline -L 40,80 src/main.rs
git timeline --author alice --since "3 months ago" src/main.rs
```

| Option | |
|---|---|
| `-L START,END` | only revisions touching those lines (`git log -L` syntax) |
| `--author P`, `--since D`, `--until D` | forwarded to `git log` |
| `--no-follow` | stop at renames |
| `--view content\|diff\|blame` | initial view |
| `--no-highlight`, `--theme NAME` | syntax highlighting (theme also via `GIT_TIMELINE_THEME`) |
| `--tab-width N` | default 4 |

`git timeline --help` is intercepted by git (it looks for a man page); use `-h`.

## Views

* **1 Content**: the file as it was at the selected revision.
* **2 Diff**: the change made by the selected revision, against its parent (for merges,
  the first parent). This holds even when the list is filtered.
* **3 Compare**: press `m` on a revision to mark it, select another one, and see the
  diff between them (always older → newer).
* **4 Blame**: who last changed each line, as of that revision. `Enter` on a line jumps
  to that commit.

When you move between revisions, the cursor stays on the **same logical line**: it follows
the code through insertions and deletions.

## Pseudo-entries

When the file has uncommitted changes, `Working tree` and/or `Staged` rows appear above
the newest commit. They are hidden when a revision range is given.

## Keys

| Key | |
|---|---|
| `j`/`k`, arrows | move in the focused pane |
| `Tab` | switch focus timeline ⇄ file |
| `[` / `]` | older / newer revision, from either pane |
| `1` `2` `3` `4` | content / diff / compare / blame |
| `m` | mark the compare base |
| `n` / `N` | next / previous hunk |
| `g` / `G`, `Ctrl-D` / `Ctrl-U`, PgUp/PgDn | jump / page |
| `h` / `l` (file pane) | horizontal scroll |
| `#` | toggle line numbers |
| `z` | zoom the file pane |
| `d` | commit details (full message, changed files) |
| `y` | copy the full commit hash (system clipboard, or OSC 52 over SSH) |
| `e` | export this version to a file (default `name@abc1234.ext`, in the launch directory) |
| `R` | restore this version into the working tree, after confirmation |
| `r` | reload |
| `?` | help |
| `q` | quit |

The mouse works too: wheel to scroll, click to select.

**Restore** only rewrites the working-tree file at its current path (with the right
executable bit). It never touches the index, HEAD or any ref. Review the result with
`git diff`.

## Environment

* `NO_COLOR`: monochrome UI, no syntax highlighting.
* `GIT_TIMELINE_THEME`: syntect theme name (e.g. `InspiredGitHub` for light terminals).

## Limitations

* Single files only, not directories.
* Working-tree content is read raw: `.gitattributes` filters (CRLF normalisation, LFS)
  are not applied, so the `Working tree` row can look fully changed in such repos.
* Renames are followed with git's own `--follow` heuristics.
* Files over 50 MiB are not displayed. Files over 1 MiB / 20 000 lines are shown without
  highlighting.

## Development

```sh
cargo test     # unit + integration tests (need git on PATH)
```

The design is documented file by file in [`mockup/`](mockup/ARCHITECTURE.md).
