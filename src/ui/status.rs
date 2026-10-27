use super::theme::MUTED;
use crate::app::{App, ViewMode};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub(super) fn draw(f: &mut Frame, app: &App, area: Rect) {
    let hint = if app.view == ViewMode::Tracker {
        "F4 piano roll · ↑/↓ rows · ←/→ columns · e edit note · Shift+Enter add voice · Ctrl+S save · ? help"
    } else {
        "F4 tracker · Space play · Enter draw · Shift+arrows move · Ctrl+arrows length/velocity · ? help"
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(app.status.as_str()),
            Line::from(Span::styled(hint, Style::default().fg(MUTED))),
        ])
        .block(Block::default().borders(Borders::TOP)),
        area,
    );
}
