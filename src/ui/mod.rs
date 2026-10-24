//! Screen composition. Individual widgets and dialogs own their rendering.

mod dialogs;
mod piano_roll;
mod status;
mod theme;
mod tracks;
mod transport;
mod velocity;

use crate::app::App;
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
    let middle = Layout::horizontal([Constraint::Length(24), Constraint::Min(20)]).split(rows[1]);
    let resized = app.roll != middle[1];
    app.track_area = middle[0];
    app.roll = middle[1];
    app.velocity_area = rows[2];
    if resized {
        app.ensure_visible();
    }

    transport::draw(f, app, rows[0]);
    tracks::draw(f, app, middle[0]);
    piano_roll::draw(f, app, middle[1]);
    velocity::draw(f, app, rows[2]);
    status::draw(f, app, rows[3]);
    dialogs::draw(f, app);
}
