//! Piano roll composition: track sidebar, note grid, and velocity details.

mod details;
mod grid;
#[cfg(test)]
mod piano_roll_tests;
mod tracks;

use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

pub(super) fn draw(f: &mut Frame, app: &mut App, area: Rect, details_area: Rect) {
    let columns = Layout::horizontal([Constraint::Length(24), Constraint::Min(20)]).split(area);
    let resized = app.roll != columns[1];
    app.track_area = columns[0];
    app.roll = columns[1];
    app.velocity_area = details_area;
    if resized {
        app.ensure_visible();
    }
    tracks::draw(f, app, columns[0]);
    grid::draw(f, app, columns[1]);
    details::draw(f, app, details_area);
}
