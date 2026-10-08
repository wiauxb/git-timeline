# `README.md`

## Purpose
End-user documentation. Developer docs live in doc comments and in `mockup/`.

## Sections

0. **Warning banner** (GitHub `> [!WARNING]` alert, first thing after the title, written
   in the maintainer's first person): the project is vibecoded and has a low priority in
   their schedule, expect rough edges; issues (linked to the GitHub issues page) and
   contributions are nonetheless welcome, with a promise to respond within a reasonable
   time frame.
1. **Pitch** + an ASCII screenshot of the two-pane UI.
2. **Install** — `cargo install --path .`; `~/.cargo/bin` on `PATH` makes `git timeline`
   work; git ≥ 2.36.
3. **Usage** — synopsis, examples (`v1.0..main`, `-L 40,80`, `--author/--since`), option
   table (`-L`, `--author/--since/--until`, `--no-follow`, `--view`, `--no-highlight`,
   `--theme`, `--tab-width`); note that `--help` is intercepted by git, use `-h`.
4. **Views** — content, diff (vs parent, first parent for merges, correct when filtered),
   compare (mark with `m`, always older → newer), blame (`Enter` jumps). Logical-line
   following when changing revision.
5. **Pseudo-entries** — `Working tree` / `Staged`, hidden with a revision range.
6. **Keys** — full table + mouse (wheel, click).
7. **Restore** — only rewrites the working-tree file at its current path (exec bit
   included); never index / HEAD / refs.
8. **Environment** — `NO_COLOR`, `GIT_TIMELINE_THEME`.
9. **Limitations** — single files; raw working-tree bytes (no `.gitattributes` filters);
   git's `--follow` heuristics; > 50 MiB not shown, > 1 MiB / 20 000 lines unhighlighted.
10. **Development** — `cargo test` (needs git), pointer to `mockup/ARCHITECTURE.md`.
