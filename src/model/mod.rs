//! Domain types shared by every layer. Pure: no IO, no git process, no ratatui.
//!
//! Line numbers are 0-based internally and rendered 1-based by `ui/`.

mod blame;
mod content;
pub mod diff;
pub mod line_map;
mod path;
mod revision;

pub use blame::*;
pub use content::*;
pub use diff::*;
pub use line_map::{LineAnchor, MappedLine};
pub use path::RepoPath;
pub use revision::*;
