use crate::{
    app::{App, tracker::pitch_name},
    ui::theme::{ACCENT, MUTED},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

pub(in crate::ui) fn draw(f: &mut Frame, app: &App, area: Rect) {
    let track = &app.project.tracks[app.track];
    let detail = if let Some(index) = app.tracker_note_index() {
        let n = &track.notes[index];
        format!(
            "{} · start {} (+{}) · length {} ticks · velocity {}",
            pitch_name(n.key),
            n.start,
            n.start % app.grid,
            n.duration,
            n.velocity
        )
    } else {
        "Empty cell · Enter inserts the entry pitch; e edits exact note values".into()
    };
    let held = app.live_notes();
    let midi: Vec<_> = (0..128)
        .filter(|&key| held.contains(key))
        .map(pitch_name)
        .collect();
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                format!(
                    "{} · {} · {:?} · ch {} · entry {} · tick {}",
                    track.name,
                    app.instrument_label(),
                    track.route,
                    track.channel + 1,
                    pitch_name(app.key),
                    app.cursor
                ),
                Style::default().fg(ACCENT),
            )),
            Line::from(detail),
            Line::from(Span::styled(
                if midi.is_empty() {
                    "e: pitch velocity length start · Alt+↑/↓: entry pitch · Alt+PgUp/PgDn: octave"
                        .into()
                } else {
                    format!("MIDI: {}", midi.join(" "))
                },
                Style::default().fg(MUTED),
            )),
        ])
        .block(Block::bordered().title(" Note details · * off-grid onset (unchanged) ")),
        area,
    );
}
