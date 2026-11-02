mod controls;
#[cfg(test)]
mod synth_panel_tests;

use super::centered;
use crate::{
    app::{App, SynthPage},
    ui::theme::{ACCENT, BG, MUTED},
    vst::SynthKind,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

const CARD_HEIGHT: u16 = 4;

pub(super) fn draw(frame: &mut Frame, app: &mut App) {
    let rect = centered(frame.area(), 96, 26);
    frame.render_widget(Clear, rect);
    let track = &app.project.tracks[app.track];
    let title = format!(
        " {} · Track {:02} · Synth controls ",
        track.synth.kind.name(),
        app.track + 1
    );
    let block = Block::bordered()
        .title(title)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let parts = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(inner);
    let picker = app.instrument_picker.as_mut().unwrap();
    let SynthPage::Parameters(parameters) = &picker.page else {
        return;
    };
    let description = match track.synth.kind {
        SynthKind::Tb303 => "MONO ACID · oscillator → resonant filter → amplifier",
        SynthKind::Basic => "POLYPHONIC · instrument / SoundFont controls",
        SynthKind::Vst3 { .. } => "VST3 · plugin parameters rendered in the terminal",
    };
    let waveform = parameters
        .iter()
        .find(|p| p.name == "Square wave")
        .map(|p| {
            if p.value >= 0.5 {
                "Waveform: SQUARE  ┌──┐  ┌──┐"
            } else {
                "Waveform: SAW     /| /| /| /|"
            }
        })
        .unwrap_or("No external windows · all controls stay in this TUI");
    frame.render_widget(
        Paragraph::new(vec![Line::from(description), Line::from(waveform)])
            .style(Style::default().fg(MUTED)),
        parts[0],
    );
    frame.render_widget(Paragraph::new("↑/↓ select · ←/→ adjust · Shift+←/→ fine · Enter toggle\n0 default · Ctrl+Z/Y undo/redo · Space play/stop\nEsc synthesizers · F10 / Shift+P close panel").style(Style::default().fg(MUTED)), parts[2]);
    if parameters.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No exposed parameters. Enable audio and check that the VST3 plugin is loaded.",
            )
            .style(Style::default().fg(MUTED)),
            parts[1],
        );
        return;
    }
    let area = parts[1];
    if area.height < CARD_HEIGHT || area.width < 8 {
        if let Some(parameter) = parameters.get(picker.selected) {
            frame.render_widget(
                Paragraph::new(format!("{}: {}", parameter.name, parameter.display)),
                area,
            );
        }
        return;
    }
    let columns = if area.width >= 64 { 2usize } else { 1 };
    let rows = (area.height / CARD_HEIGHT) as usize;
    let capacity = columns * rows;
    picker.page_size = capacity;
    picker.selected = picker.selected.min(parameters.len() - 1);
    picker.offset = picker.offset / columns * columns;
    if picker.selected < picker.offset {
        picker.offset = picker.selected / columns * columns;
    }
    if picker.selected >= picker.offset + capacity {
        picker.offset = (picker.selected / columns + 1 - rows) * columns;
    }
    let width = area.width / columns as u16;
    for (visible, parameter) in parameters
        .iter()
        .skip(picker.offset)
        .take(capacity)
        .enumerate()
    {
        let index = picker.offset + visible;
        let card = Rect::new(
            area.x + (visible % columns) as u16 * width,
            area.y + (visible / columns) as u16 * CARD_HEIGHT,
            width,
            CARD_HEIGHT,
        );
        controls::draw(frame, parameter, index, index == picker.selected, card);
    }
    if parameters.len() > capacity {
        let mut state = ScrollbarState::new(parameters.len()).position(picker.selected);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(Color::Yellow)),
            rect,
            &mut state,
        );
    }
}
