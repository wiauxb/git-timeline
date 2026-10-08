//! Every non-syntax style. ANSI named colors so the user's palette applies.

use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub border: Style,
    pub border_focused: Style,
    pub title: Style,
    pub selected_row: Style,
    pub selected_row_unfocused: Style,
    pub cursor_line: Style,
    pub cursor_line_unfocused: Style,
    pub hash: Style,
    pub date: Style,
    pub author: Style,
    pub subject: Style,
    pub dim: Style,
    pub pseudo_entry: Style,
    pub mark: Style,
    pub change_added: Style,
    pub change_deleted: Style,
    pub change_renamed: Style,
    pub change_other: Style,
    pub diff_added: Style,
    pub diff_removed: Style,
    pub diff_hunk_header: Style,
    pub diff_added_bg: Style,
    pub diff_removed_bg: Style,
    pub line_number: Style,
    pub blame_gutter: Style,
    pub blame_age: [Color; 6],
    pub status_badge: Style,
    pub status_tab: Style,
    pub status_tab_active: Style,
    pub status_info: Style,
    pub status_warn: Style,
    pub status_error: Style,
    pub danger: Style,
    pub link: Style,
    /// Search hits, both panes.
    pub search_match: Style,
    pub mono: bool,
}

pub fn theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            monochrome()
        } else {
            colored()
        }
    })
}

fn colored() -> Theme {
    let s = Style::default;
    Theme {
        border: s().fg(Color::DarkGray),
        border_focused: s().fg(Color::Cyan),
        title: s().add_modifier(Modifier::BOLD),
        selected_row: s().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD),
        selected_row_unfocused: s().bg(Color::DarkGray).fg(Color::White),
        cursor_line: s().bg(Color::Indexed(236)),
        cursor_line_unfocused: s().add_modifier(Modifier::UNDERLINED),
        hash: s().fg(Color::Yellow),
        date: s().fg(Color::Green),
        author: s().fg(Color::Cyan),
        subject: s(),
        dim: s().fg(Color::DarkGray),
        pseudo_entry: s().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        mark: s().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        change_added: s().fg(Color::Green).add_modifier(Modifier::BOLD),
        change_deleted: s().fg(Color::Red).add_modifier(Modifier::BOLD),
        change_renamed: s().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        change_other: s().fg(Color::Blue).add_modifier(Modifier::BOLD),
        diff_added: s().fg(Color::Green),
        diff_removed: s().fg(Color::Red),
        diff_hunk_header: s().fg(Color::Cyan),
        diff_added_bg: s().bg(Color::Indexed(22)),
        diff_removed_bg: s().bg(Color::Indexed(52)),
        line_number: s().fg(Color::DarkGray),
        blame_gutter: s().fg(Color::Gray),
        blame_age: [
            Color::LightRed,
            Color::LightYellow,
            Color::Yellow,
            Color::Green,
            Color::Cyan,
            Color::Blue,
        ],
        status_badge: s().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD),
        status_tab: s().fg(Color::Gray),
        status_tab_active: s().fg(Color::Black).bg(Color::Gray).add_modifier(Modifier::BOLD),
        status_info: s().fg(Color::Green),
        status_warn: s().fg(Color::Yellow),
        status_error: s().fg(Color::White).bg(Color::Red),
        danger: s().fg(Color::Red).add_modifier(Modifier::BOLD),
        link: s().add_modifier(Modifier::UNDERLINED),
        search_match: s().fg(Color::Black).bg(Color::Yellow),
        mono: false,
    }
}

fn monochrome() -> Theme {
    let s = Style::default;
    let bold = s().add_modifier(Modifier::BOLD);
    let dim = s().add_modifier(Modifier::DIM);
    let rev = s().add_modifier(Modifier::REVERSED);
    Theme {
        border: dim,
        border_focused: bold,
        title: bold,
        selected_row: rev,
        selected_row_unfocused: rev.add_modifier(Modifier::DIM),
        cursor_line: rev,
        cursor_line_unfocused: s().add_modifier(Modifier::UNDERLINED),
        hash: s(),
        date: dim,
        author: s(),
        subject: s(),
        dim,
        pseudo_entry: bold,
        mark: bold,
        change_added: bold,
        change_deleted: bold,
        change_renamed: bold,
        change_other: bold,
        diff_added: bold,
        diff_removed: dim,
        diff_hunk_header: bold,
        diff_added_bg: s(),
        diff_removed_bg: s(),
        line_number: dim,
        blame_gutter: dim,
        blame_age: [Color::Reset; 6],
        status_badge: rev,
        status_tab: s(),
        status_tab_active: rev,
        status_info: s(),
        status_warn: bold,
        status_error: rev,
        danger: bold,
        link: s().add_modifier(Modifier::UNDERLINED),
        search_match: rev,
        mono: true,
    }
}
