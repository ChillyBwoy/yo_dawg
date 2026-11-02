use super::centered;
use crate::{
    app::{App, SynthPage},
    ui::theme::{ACCENT, MUTED},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};

pub(super) fn draw(f: &mut Frame, app: &mut App) {
    if app
        .instrument_picker
        .as_ref()
        .is_some_and(|p| matches!(p.page, SynthPage::Parameters(_)))
    {
        super::synth_panel::draw(f, app);
        return;
    }
    let rect = centered(f.area(), 72, 22);
    f.render_widget(Clear, rect);
    let parts = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(rect);
    let picker = app.instrument_picker.as_mut().unwrap();
    picker.page_size = parts[0].height.saturating_sub(2).max(1) as usize;
    let t = &app.project.tracks[app.track];
    let (title, items, help): (_, Vec<ListItem<'_>>, _) = match &picker.page {
        SynthPage::Synthesizers(synths) => (
            "Synthesizers",
            synths
                .iter()
                .map(|synth| {
                    ListItem::new(format!(
                        "{} {}",
                        if synth == &t.synth.kind { "●" } else { " " },
                        synth.name()
                    ))
                })
                .collect(),
            "↑/↓ select · Enter: instruments / controls · Esc: close\nF10 / Shift+P: current track's synth panel",
        ),
        SynthPage::Parameters(parameters) => (
            "Parameters",
            parameters
                .iter()
                .map(|p| ListItem::new(format!("{}: {}", p.name, p.display)))
                .collect(),
            "↑/↓ select · ←/→ adjust · Esc: synthesizers",
        ),
        SynthPage::Instruments => (
            "Basic instruments",
            (0..128)
                .map(|program| {
                    let label = app.instruments.label(program, app.track, t.route);
                    ListItem::new(format!(
                        "{} {}",
                        if program == t.program { "●" } else { " " },
                        label
                    ))
                })
                .collect(),
            "↑/↓ select · Enter apply · Esc: synthesizers\nF10 / Shift+P: synth panel · ● current instrument",
        ),
    };
    let count = items.len();
    let mut state = ListState::default().with_selected(Some(picker.selected));
    *state.offset_mut() = picker.offset;
    f.render_stateful_widget(
        List::new(items)
            .block(Block::bordered().title(format!(
                " {title} · {}/{} ",
                (picker.selected + 1).min(count),
                count
            )))
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
    f.render_widget(
        Paragraph::new(help).style(Style::default().fg(MUTED)),
        parts[1],
    );
}
