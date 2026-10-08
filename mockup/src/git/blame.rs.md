# `src/git/blame.rs`

## Purpose
Run `git blame --porcelain` for a timeline row and parse it into `BlameData`.

## Contents

```rust
/// `staged` = bytes of the staged blob, required for the Index row.
pub fn blame(git: &Git, entry: &TimelineEntry, staged: Option<&[u8]>) -> Result<BlameData>;
pub fn parse_porcelain(out: &[u8]) -> Result<BlameData>;
fn to_time(unix: i64, tz: &str) -> DateTime<FixedOffset>;   // tz "+0200" / "-0530"
```

## Command per row kind
| Row | Command |
|---|---|
| `Commit(oid)` | `git blame --porcelain <oid> -- <entry.path>` (path *at that commit*) |
| `WorkingTree` | `git blame --porcelain -- <path>` (uncommitted lines get the null oid) |
| `Index` | `git blame --porcelain --contents - -- <path>`, staged bytes on stdin |

The app never requests blame for absent / non-text versions (no `BlameKey`, or content not
text). Plain path after `--` (no pathspec magic). Default rename handling of git blame
(no `-M`/`-C`).

## Porcelain parsing
```
<oid> <orig_line> <final_line> [<group_count>]
author …            author-time <unix>     author-tz +0200
summary …           [boundary]             filename <orig path>
\t<line content>
```
* A header line opens a record; key/value lines follow; the `\t` content line closes it.
* Commit info is stored the first time an oid is seen; `filename` is remembered per oid.
* Line numbers are 1-based in the output, stored 0-based; lines are sorted by final line.
* `group_start` = first line or commit differs from the previous line's commit.
* Content text is ignored (we already have the content).

## Specs
Blame can take seconds on big old files: it runs on a worker; the view shows content with
an empty gutter and a spinner meanwhile. Results (and errors) are cached by `BlameKey`.

## Tests
Unit: parse with repeated oid, boundary flag, groups. Integration: per-commit attribution,
working tree / staged rows have null-oid lines.
