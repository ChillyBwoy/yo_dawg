//! Screen composition. Individual widgets and dialogs own their rendering.

mod dialogs;
mod piano_roll;
mod status;
mod theme;
mod tracker;
mod transport;

use crate::app::{App, ViewMode};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph},
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(theme::BG).fg(Color::White)),
        area,
    );
    if area.width < 70 || area.height < 20 {
        f.render_widget(
            Paragraph::new(
                "yo_dawg needs at least 70×20 terminal cells. Resize the terminal; Ctrl+Q quits.",
            ),
            area,
        );
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(5),
        Constraint::Length(3),
    ])
    .split(area);
    transport::draw(f, app, rows[0]);
    match app.view {
        ViewMode::PianoRoll => {
            piano_roll::draw(f, app, rows[1], rows[2]);
        }
        ViewMode::Tracker => {
            app.track_area = Default::default();
            app.velocity_area = Default::default();
            app.prepare_tracker(rows[1]);
            tracker::draw(f, app, rows[1]);
            tracker::details(f, app, rows[2]);
        }
    }
    status::draw(f, app, rows[3]);
    dialogs::draw(f, app);
}
