# `src/git/blame.rs`

## Purpose
Run `git blame --porcelain` for the selected revision and parse it into `BlameData`.

## Contents

```rust
pub fn blame(git: &Git, entry: &TimelineEntry, staged_bytes: Option<&[u8]>) -> Result<BlameData>;
pub(crate) fn parse_porcelain(out: &[u8]) -> Result<BlameData>;
```

## Command per row kind
| Row | Command |
|---|---|
| `Commit(oid)` | `git blame --porcelain <oid> -- <entry.path>` |
| `WorkingTree` | `git blame --porcelain -- <path>` (uncommitted lines get the null oid) |
| `Index` | `git blame --porcelain --contents - -- <path>` with the **staged blob bytes on stdin** |
| `Deleted` / Absent / Binary / Submodule | not run; view shows "nothing to blame" |

* `entry.path` is the path **at that commit** (renames).
* Pathspec magic is not used for blame (it takes a plain path after `--`).
* Default git rename/copy detection inside blame is used (no `-M`/`-C` in v1).

## Porcelain parsing
```
<oid> <orig_line> <final_line> [<group_count>]
author …
author-time <unix>
author-tz +0200
summary …
[boundary]
filename <orig path>
\t<line content>
```
* Header fields appear only the first time an oid is seen ⇒ cache in `commits` map.
* `final_line` is 1-based → stored 0-based; lines are assembled by `final_line`.
* Content lines (after `\t`) are ignored (we already have the content).
* `group_start` = true when the oid differs from the previous final line's oid.

## Specs
* Blame can be slow (seconds on old, big files). It runs on a worker; the view shows a
  spinner + the plain content meanwhile. Switching revision while it runs ⇒ result is
  still cached when it arrives (cheap to switch back).
