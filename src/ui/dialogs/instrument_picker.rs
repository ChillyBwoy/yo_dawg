use super::centered;
use crate::{
    app::App,
    ui::theme::{ACCENT, MUTED},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};

pub(super) fn draw(f: &mut Frame, app: &mut App) {
    let rect = centered(f.area(), 72, 22);
    f.render_widget(Clear, rect);
    let parts = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(rect);
    let picker = app.instrument_picker.as_mut().unwrap();
    picker.page_size = parts[0].height.saturating_sub(2).max(1) as usize;
    let t = &app.project.tracks[app.track];
    let items: Vec<_> = (0..128)
        .map(|program| {
            let label = app.instruments.label(program, app.track, t.route);
            ListItem::new(format!(
                "{} {}",
                if program == t.program { "●" } else { " " },
                label
            ))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(picker.selected));
    *state.offset_mut() = picker.offset;
    f.render_stateful_widget(
        List::new(items)
            .block(Block::bordered().title(format!(" Instruments · {}/128 ", picker.selected + 1)))
            .highlight_symbol("› ")
            .highlight_style(
                Style::default()
                    .fg(ACCENT)
                    .bg(Color::Rgb(38, 75, 69))
                    .add_modifier(Modifier::BOLD),
            ),
        parts[0],
        &mut state,
    );
    picker.offset = state.offset();
    f.render_widget(Paragraph::new("↑/↓ select · PgUp/PgDn page · Home/End\nEnter apply · Esc cancel · ● current instrument")
        .style(Style::default().fg(MUTED)), parts[1]);
}
