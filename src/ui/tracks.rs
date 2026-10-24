use super::theme::ACCENT;
use crate::app::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, List, ListItem, ListState},
};

pub(super) fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<_> = app
        .project
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            ListItem::new(format!(
                "{:02} {}{} {}",
                i + 1,
                if t.mute { "M" } else { "·" },
                if t.solo { "S" } else { "·" },
                t.name
            ))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.track));
    *state.offset_mut() = app.track_offset;
    f.render_stateful_widget(
        List::new(items)
            .block(Block::bordered().title(" Tracks · Tab / a "))
            .highlight_style(Style::default().bg(Color::Rgb(38, 75, 69)).fg(ACCENT))
            .highlight_symbol("›"),
        area,
        &mut state,
    );
    app.track_offset = state.offset();
}
