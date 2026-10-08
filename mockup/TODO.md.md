# `TODO.md`

## Purpose
Backlog of known/likely bugs and open verifications, as a checklist. Not a design
document: specs stay in the other mockup files.

## Contents
* **Worth fixing** — bugs with user-visible impact.
* **Minor** — cosmetic or nearly unreachable issues.
* **To verify** — behaviors not yet confirmed by a test.

Each item: checkbox, bold one-line title, the file(s)/function involved, and a short
description of the failure.

## Rules
* Fixing an item follows the mockup-first loop: write the fix design into the affected
  mockup specs → user agreement → code + test → remove the item from `TODO.md` and from the
  list below.
* New suspected bugs found during work are added here rather than left in chat.

## Current items
Worth fixing: blame spinning forever on line-count mismatch; failed diff shown as "no
changes"; commit details empty for merges; prefetch delaying the selected revision;
restore dialog hiding its buttons when text wraps.
Minor: blame error row hiding the cursor; export-to-directory prompting "Overwrite?"
first; `F1`–`F4` missing from help; tests not skipping without git ≥ 2.36; `-L` not
validated on unborn HEAD; fallback rows assuming a regular file; type-changing merges
labelled "Merge".
To verify: whether `-L` drops merges; multi-chunk streaming test.
