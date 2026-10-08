//! Raw terminal events → `Action`, depending on context. `BINDINGS` feeds the help overlay.

use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};

use super::action::Action;
use super::state::{AppState, Focus, Overlay, ViewMode};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Context {
    Global,
    Timeline,
    File,
    DiffViews,
    BlameView,
}

impl Context {
    pub fn title(self) -> &'static str {
        match self {
            Context::Global => "Global",
            Context::Timeline => "Timeline pane",
            Context::File => "File pane",
            Context::DiffViews => "Diff views",
            Context::BlameView => "Blame view",
        }
    }
}

pub struct Binding {
    pub keys: &'static str,
    pub action: &'static str,
    pub context: Context,
}

const fn b(keys: &'static str, action: &'static str, context: Context) -> Binding {
    Binding { keys, action, context }
}

pub static BINDINGS: &[Binding] = &[
    b("q  Ctrl-C", "quit", Context::Global),
    b("?", "this help", Context::Global),
    b("Tab", "switch focus timeline / file", Context::Global),
    b("1 2 3 4", "view: content / diff vs previous / diff vs marked / blame", Context::Global),
    b("[  ]", "older / newer revision (from any pane)", Context::Global),
    b("m", "mark / unmark revision as compare base", Context::Global),
    b("z", "zoom the file pane", Context::Global),
    b("w", "toggle soft wrap (file pane)", Context::Global),
    b("d", "commit details", Context::Global),
    b("y", "copy full commit hash", Context::Global),
    b("e", "export this version to a file", Context::Global),
    b("R", "restore this version into the working tree", Context::Global),
    b("r", "reload history and local changes", Context::Global),
    b("/", "search the focused pane (regex, smart case)", Context::Global),
    b("n N", "next / previous search match", Context::Global),
    b("j k  ↓ ↑", "older / newer revision", Context::Timeline),
    b("g G  Home End", "newest / oldest", Context::Timeline),
    b("PgDn PgUp  Ctrl-F Ctrl-B", "page down / up", Context::Timeline),
    b("Enter  l  →", "focus the file pane", Context::Timeline),
    b("Esc", "clear the search", Context::Timeline),
    b("j k  ↓ ↑", "move cursor", Context::File),
    b("Ctrl-D Ctrl-U", "half page down / up", Context::File),
    b("PgDn Space  PgUp", "page down / up", Context::File),
    b("g G", "top / bottom", Context::File),
    b("h l  ← →", "scroll horizontally", Context::File),
    b("#", "toggle line numbers", Context::File),
    b("Esc", "clear the search, else back to the timeline", Context::File),
    b("}  {", "next / previous hunk", Context::DiffViews),
    b("Enter", "jump to the revision that last changed the line", Context::BlameView),
];

pub fn map(s: &AppState, ev: Event) -> Option<Action> {
    match ev {
        Event::Key(k) if k.kind != KeyEventKind::Release => map_key(s, k),
        Event::Mouse(m) => map_mouse(s, m),
        Event::Paste(text) => match s.overlay {
            Some(Overlay::ExportPrompt { .. } | Overlay::Search(_)) => Some(Action::InputPaste(text)),
            _ => None,
        },
        // redraw; the new layout is computed by the draw itself
        Event::Resize(..) => Some(Action::Layout(s.layout)),
        _ => None,
    }
}

fn map_key(s: &AppState, k: KeyEvent) -> Option<Action> {
    use KeyCode::*;
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    if ctrl && k.code == Char('c') {
        return Some(Action::Quit);
    }

    if let Some(o) = &s.overlay {
        return match o {
            Overlay::ExportPrompt { .. } | Overlay::Search(_) => match k.code {
                Enter => Some(Action::Confirm),
                Esc => Some(Action::Cancel),
                Backspace => Some(Action::InputBackspace),
                Delete => Some(Action::InputDelete),
                Left => Some(Action::InputLeft),
                Right => Some(Action::InputRight),
                Home => Some(Action::InputHome),
                End => Some(Action::InputEnd),
                Char('a') if ctrl => Some(Action::InputHome),
                Char('e') if ctrl => Some(Action::InputEnd),
                Char(c) if !ctrl => Some(Action::InputChar(c)),
                _ => None,
            },
            // Enter deliberately does nothing: only `y` confirms.
            Overlay::ConfirmRestore { .. } | Overlay::ConfirmOverwrite { .. } => match k.code {
                Char('y') | Char('Y') => Some(Action::Confirm),
                Char('n') | Char('N') | Char('q') | Esc => Some(Action::Cancel),
                _ => None,
            },
            Overlay::Help { .. } | Overlay::Details { .. } => match k.code {
                Char('j') | Down => Some(Action::CursorDown(1)),
                Char('k') | Up => Some(Action::CursorUp(1)),
                PageDown | Char(' ') => Some(Action::CursorDown(10)),
                PageUp => Some(Action::CursorUp(10)),
                Esc | Char('q') | Char('?') | Enter | Char('d') => Some(Action::Cancel),
                _ => None,
            },
            Overlay::Error { .. } => Some(Action::Cancel),
        };
    }

    // pane-specific
    let pane = match s.focus {
        Focus::Timeline => match k.code {
            Char('j') | Down => Some(Action::SelectNext),
            Char('k') | Up => Some(Action::SelectPrev),
            Char('g') | Home => Some(Action::SelectFirst),
            Char('G') | End => Some(Action::SelectLast),
            PageDown => Some(Action::SelectPageDown),
            PageUp => Some(Action::SelectPageUp),
            Char('f') if ctrl => Some(Action::SelectPageDown),
            Char('b') if ctrl => Some(Action::SelectPageUp),
            Enter | Char('l') | Right => Some(Action::FocusFile),
            Esc => Some(Action::Escape),
            _ => None,
        },
        Focus::File => match k.code {
            Char('j') | Down => Some(Action::CursorDown(1)),
            Char('k') | Up => Some(Action::CursorUp(1)),
            Char('d') if ctrl => Some(Action::HalfPageDown),
            Char('u') if ctrl => Some(Action::HalfPageUp),
            PageDown | Char(' ') => Some(Action::PageDown),
            Char('f') if ctrl => Some(Action::PageDown),
            Char('b') if ctrl => Some(Action::PageUp),
            PageUp => Some(Action::PageUp),
            Char('g') | Home => Some(Action::Top),
            Char('G') | End => Some(Action::Bottom),
            Char('h') | Left => Some(Action::ScrollLeft),
            Char('l') | Right => Some(Action::ScrollRight),
            Char('#') => Some(Action::ToggleLineNumbers),
            Esc => Some(Action::Escape),
            Char('}') if s.view.is_diff() => Some(Action::NextHunk),
            Char('{') if s.view.is_diff() => Some(Action::PrevHunk),
            Enter if s.view == ViewMode::Blame => Some(Action::BlameJump),
            _ => None,
        },
    };
    if pane.is_some() {
        return pane;
    }
    if ctrl {
        return None;
    }

    match k.code {
        Char('q') => Some(Action::Quit),
        Char('?') => Some(Action::ShowHelp),
        Tab | BackTab => Some(Action::ToggleFocus),
        Char('1') | F(1) => Some(Action::SetView(ViewMode::Content)),
        Char('2') | F(2) => Some(Action::SetView(ViewMode::DiffPrev)),
        Char('3') | F(3) => Some(Action::SetView(ViewMode::DiffMarked)),
        Char('4') | F(4) => Some(Action::SetView(ViewMode::Blame)),
        Char('[') => Some(Action::SelectNext),
        Char(']') => Some(Action::SelectPrev),
        Char('m') => Some(Action::ToggleMark),
        Char('z') => Some(Action::ToggleZoom),
        Char('w') => Some(Action::ToggleWrap),
        Char('d') => Some(Action::ShowDetails),
        Char('y') => Some(Action::CopyHash),
        Char('e') => Some(Action::StartExport),
        Char('R') => Some(Action::StartRestore),
        Char('r') => Some(Action::Reload),
        Char('/') => Some(Action::StartSearch),
        Char('n') => Some(Action::SearchNext),
        Char('N') => Some(Action::SearchPrev),
        _ => None,
    }
}

fn inside(r: Rect, col: u16, row: u16) -> bool {
    r.contains(Position { x: col, y: row })
}

fn map_mouse(s: &AppState, m: MouseEvent) -> Option<Action> {
    if s.overlay.is_some() {
        return None;
    }
    let (col, row) = (m.column, m.row);
    let tl = s.layout.timeline;
    let fb = s.layout.file_body;
    let in_tl = inside(tl, col, row);
    let in_file = inside(s.layout.file, col, row);
    match m.kind {
        MouseEventKind::ScrollDown if in_tl => Some(Action::WheelTimeline(3)),
        MouseEventKind::ScrollUp if in_tl => Some(Action::WheelTimeline(-3)),
        MouseEventKind::ScrollDown if in_file => Some(Action::WheelFile(3)),
        MouseEventKind::ScrollUp if in_file => Some(Action::WheelFile(-3)),
        MouseEventKind::Down(MouseButton::Left) if in_tl => {
            // inside the border
            let r = row.checked_sub(tl.y + 1)? as usize;
            if row >= tl.y + tl.height.saturating_sub(1) {
                return None;
            }
            Some(Action::SelectIndex(s.list_offset + r))
        }
        MouseEventKind::Down(MouseButton::Left) if inside(fb, col, row) => {
            Some(Action::ClickFileRow((row - fb.y) as usize))
        }
        MouseEventKind::Down(MouseButton::Left) if in_file => Some(Action::FocusFile),
        _ => None,
    }
}
