//! Screen-row geometry of the file pane. `cursor` / `top` are logical rows; with soft
//! wrap a row takes 1+ screen rows. With wrap off every height is 1.

use super::state::{AppState, Displayed};
use crate::ui::content_view::wrap_breaks;
use crate::ui::file_pane::gutter_cols;

/// Screen rows taken by `row` (1 without wrap, for hunk headers and rows without text).
pub fn row_height(s: &AppState, d: &Displayed, row: usize) -> usize {
    if !s.file.wrap {
        return 1;
    }
    match d.row_text(row) {
        Some(t) => wrap_breaks(&t, text_cols(s, d), s.opts.tab_width).len() + 1,
        None => 1,
    }
}

/// Text columns of the current view.
pub fn text_cols(s: &AppState, d: &Displayed) -> usize {
    s.file_cols().saturating_sub(gutter_cols(s, d)).max(1)
}

/// Visible screen rows of `row` when it is the top row (minus `top_skip`).
fn visible_height(s: &AppState, d: &Displayed, row: usize) -> usize {
    let h = row_height(s, d, row);
    if row == s.file.top {
        h.saturating_sub(s.file.top_skip).max(1)
    } else {
        h
    }
}

/// Set `top`; `top_skip` only survives while `top` doesn't change.
pub fn set_top(s: &mut AppState, top: usize) {
    if top != s.file.top {
        s.file.top = top;
        s.file.top_skip = 0;
    }
}

/// Screen rows from the top of `top` to the end of `row` (inclusive), capped at `cap`.
pub fn screen_span(s: &AppState, d: &Displayed, top: usize, row: usize, cap: usize) -> usize {
    let mut acc = 0;
    for r in top..=row {
        acc += if r == top { visible_height(s, d, r) } else { row_height(s, d, r) };
        if acc >= cap {
            return cap;
        }
    }
    acc
}

/// Largest `top` for which rows `top..` still fill the screen (0 if the content is shorter).
pub fn max_top(s: &AppState, d: &Displayed) -> usize {
    let n = d.row_count().unwrap_or(0);
    let rows = s.file_rows();
    if !s.file.wrap {
        return n.saturating_sub(rows);
    }
    let mut acc = 0;
    for t in (0..n).rev() {
        acc += row_height(s, d, t);
        if acc >= rows {
            return t;
        }
    }
    0
}

/// Largest `top` with at most `offset` screen rows above `cursor` (anchor restore).
pub fn top_for_offset(s: &AppState, d: &Displayed, cursor: usize, offset: usize) -> usize {
    if !s.file.wrap {
        return cursor.saturating_sub(offset);
    }
    let (mut t, mut acc) = (cursor, 0);
    while t > 0 {
        let h = row_height(s, d, t - 1);
        if acc + h > offset {
            break;
        }
        acc += h;
        t -= 1;
    }
    t
}

/// Screen rows above the cursor, from `top` (anchor capture).
pub fn offset_of(s: &AppState, d: &Displayed) -> usize {
    let (top, cursor) = (s.file.top, s.file.cursor);
    if cursor <= top {
        return 0;
    }
    if !s.file.wrap {
        return cursor - top;
    }
    screen_span(s, d, top, cursor - 1, s.file_rows())
}

/// Logical rows to move so the move covers `screen` screen rows (≥ 1).
pub fn rows_for_screen(s: &AppState, d: &Displayed, from: usize, screen: usize, forward: bool) -> usize {
    if !s.file.wrap {
        return screen.max(1);
    }
    let n = d.row_count().unwrap_or(0);
    let (mut k, mut acc) = (0, 0);
    loop {
        let r = if forward { from + k } else { from.wrapping_sub(k + 1) };
        if r >= n {
            break;
        }
        let h = row_height(s, d, r);
        if acc + h > screen && k > 0 {
            break;
        }
        acc += h;
        k += 1;
    }
    k.max(1)
}

/// Logical row drawn at screen row `y` of the body; None past the content.
pub fn row_at_screen(s: &AppState, d: &Displayed, y: usize) -> Option<usize> {
    let n = d.row_count()?;
    if !s.file.wrap {
        return (s.file.top + y < n).then_some(s.file.top + y);
    }
    let mut acc = 0;
    for r in s.file.top..n {
        acc += visible_height(s, d, r);
        if y < acc {
            return Some(r);
        }
    }
    None
}

/// `top` adjusted so the whole cursor row is visible (its first screen row when taller
/// than the body).
pub fn scroll_to_cursor(s: &mut AppState) {
    let (rows, cursor, top) = (s.file_rows(), s.file.cursor, s.file.top);
    if cursor < top {
        return set_top(s, cursor);
    }
    if !s.file.wrap {
        if cursor >= top + rows {
            set_top(s, cursor + 1 - rows);
        }
        return;
    }
    let d = s.displayed();
    // walk back from the cursor: the screen fits rows `first..=cursor`
    let (mut t, mut acc) = (cursor, 0);
    loop {
        if t == top {
            acc += visible_height(s, &d, t);
            if acc <= rows || t == cursor {
                return; // already visible (or the cursor row is the tall top row)
            }
            break;
        }
        acc += row_height(s, &d, t);
        if acc > rows {
            break;
        }
        t -= 1;
    }
    let first = if acc > rows && t < cursor { t + 1 } else { t };
    set_top(s, first.max(top));
}

/// Make screen row `segment` of `row` (the cursor row) visible, scrolling inside it when
/// it is taller than the body.
pub fn reveal(s: &mut AppState, d: &Displayed, row: usize, segment: usize) {
    let rows = s.file_rows();
    let h = row_height(s, d, row);
    if h <= rows {
        return scroll_to_cursor(s);
    }
    if s.file.top == row && (s.file.top_skip..s.file.top_skip + rows).contains(&segment) {
        return;
    }
    s.file.top = row;
    s.file.top_skip = segment.saturating_sub(rows / 3).min(h - rows);
}
