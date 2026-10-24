use super::theme::ACCENT;
use crate::{
    app::App,
    model::{BAR, PPQ},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Paragraph},
};

pub(super) fn draw(f: &mut Frame, app: &App, area: Rect) {
    let position = app.position();
    let text = format!(
        " {} {}  {} BPM  4/4  {:03}:{:02}  Loop {}–{} ({} bars)  Grid 1/{}  {}{}",
        if app.playing() {
            "▶ PLAY"
        } else {
            "■ STOP"
        },
        if app.recording { "● REC" } else { "" },
        app.project.bpm,
        position / BAR + 1,
        position % BAR / PPQ + 1,
        app.project.loop_start / BAR + 1,
        app.project.loop_end / BAR + 1,
        (app.project.loop_end - app.project.loop_start) / BAR,
        BAR / app.grid,
        app.path.display(),
        if app.dirty { " *" } else { "" },
    );
    f.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(ACCENT))
            .block(Block::bordered().title(" yo_dawg · TUI DAW ")),
        area,
    );
}
