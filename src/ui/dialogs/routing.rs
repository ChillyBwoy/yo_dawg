use super::centered;
use crate::app::App;
use ratatui::{
    Frame,
    widgets::{Block, Clear, Paragraph, Wrap},
};
use std::sync::atomic::Ordering;

pub(super) fn draw(f: &mut Frame, app: &App) {
    let t = &app.project.tracks[app.track];
    let text = format!(
        "Selected track: {}\n\nF6  MIDI input: {}\nF7  Track MIDI output: {}\nr   Track route: {:?}\nc   MIDI channel: {}\np   Program: {}\nF8  External live MIDI Thru: {} (off avoids FM-1 local echo)\nF9  Audio input monitor: {}\n\nF5  Refresh / reconnect selected MIDI ports\nAudio devices: --devices; choose --input N / --output N at startup.\nBLE: connect in OS first, then F5. USB/BLE to the same device:\nselect ONE output port to avoid doubled notes.\n\nUSB Audio on FM-1 is not confirmed. Use 3.5mm → audio interface.\nInput/output sample-rate conversion is basic; use one interface\nfor stable low-latency monitoring. Esc / F2 closes this panel.",
        t.name,
        app.input.as_deref().unwrap_or("none"),
        t.midi_output.as_deref().unwrap_or("none"),
        t.route,
        t.channel + 1,
        app.instrument_label(),
        if app.thru { "ON" } else { "OFF" },
        if app.audio.monitor.load(Ordering::Relaxed) {
            "ON (requires --input N)"
        } else {
            "OFF"
        },
    );
    let rect = centered(f.area(), 85, 23);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(text)
            .block(Block::bordered().title(" MIDI / Audio routing "))
            .wrap(Wrap { trim: false }),
        rect,
    );
}
