//! Musical data and timing units. Independent of devices, storage, and UI.

mod note;
mod project;
mod track;

pub use note::Note;
pub use project::Project;
pub use track::{Route, Track};

pub const PPQ: u32 = 480;
pub const BAR: u32 = PPQ * 4;

pub fn snap(tick: u32, grid: u32) -> u32 {
    ((tick + grid / 2) / grid) * grid
}
