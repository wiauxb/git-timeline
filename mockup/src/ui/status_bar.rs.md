# `src/ui/status_bar.rs`

## Purpose
One-row bar at the bottom: focus/mode indicator, view tabs, context-sensitive key hints,
and transient messages.

## Contents

```rust
pub fn render(f: &mut Frame, area: Rect, s: &AppState);
```

## Layout (left → right)
1. Focus badge: ` TIMELINE ` / ` FILE ` (reversed).
2. View tabs: `1 Content  2 Diff  3 Cmp  4 Blame`, current one highlighted; `3 Cmp`
   dimmed when no mark.
3. Right side:
   * If `s.status` is set and not expired: the message (colored by level). Info
     messages expire after 3 s, warnings 5 s, errors stay until the next action.
   * Else: hints for the focused context, truncated to fit:
     timeline → `m mark · y copy · e export · R restore · ? help`;
     file → `n/N hunk · # numbers · w wrap · z zoom · ? help`;
     blame → `⏎ jump to commit`.
   * Far right, when relevant: `line 120/214` and the cursor line's blame summary.
