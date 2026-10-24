use super::theme::MUTED;
use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub(super) fn draw(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Paragraph::new(vec![
        Line::from(app.status.as_str()),
        Line::from(Span::styled("Space play · Enter draw · Shift+arrows move · Ctrl+arrows length/velocity · Ctrl+S save · ? help", Style::default().fg(MUTED))),
    ]).block(Block::default().borders(Borders::TOP)), area);
}
