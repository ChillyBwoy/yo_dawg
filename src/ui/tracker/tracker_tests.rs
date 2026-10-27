use super::*;
use crate::{
    app::{ViewMode, tracker::projection::Projection},
    audio::{Audio, Options},
    engine::{Command, Engine},
    model::{Note, Project},
};
use ratatui::{Terminal, backend::TestBackend};
use std::time::{Duration, Instant};

fn app() -> App {
    let (errors, _) = crossbeam_channel::bounded(1);
    let audio = Audio::open(
        &Options {
            disabled: true,
            ..Default::default()
        },
        errors,
    )
    .unwrap();
    let mut project = Project::default();
    project.tracks[0].notes = vec![
        Note {
            start: 0,
            duration: 480,
            key: 67,
            velocity: 100,
        },
        Note {
            start: 17,
            duration: 31,
            key: 60,
            velocity: 90,
        },
        Note {
            start: 64,
            duration: 10,
            key: 64,
            velocity: 80,
        },
    ];
    let engine = Engine::start(project.clone(), audio.sink.clone());
    let mut app = App::new(
        project,
        engine,
        audio.sink,
        "unused.json".into(),
        audio.instruments,
    );
    app.set_view(ViewMode::Tracker);
    app
}

fn text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn tracker_shows_polyphony_raw_lengths_sustain_and_off_grid_markers() {
    let mut app = app();
    let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let display = text(&terminal);
    for expected in [
        "Tracker", "BAR:ROW", "C4", "E4", "G4", "000031*", "===", "OFF",
    ] {
        assert!(display.contains(expected), "Missing {expected}");
    }
    assert_eq!(app.project.tracks[0].notes[1].start, 17);
    assert!(!app.dirty);
}

#[test]
fn tracker_supports_small_terminals_and_many_columns() {
    let mut app = app();
    for (width, height) in [(70, 20), (120, 35), (240, 60)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        app.tracker.column_offset = Projection::new(&app.project, app.grid).columns.len() - 1;
        terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
        assert!(text(&terminal).contains("Tracker"));
    }
}

#[test]
fn held_midi_notes_are_highlighted_in_tracker_without_recording() {
    let mut app = app();
    app.engine.send(Command::Live(vec![0x90, 60, 100]));
    let deadline = Instant::now() + Duration::from_secs(1);
    while !app.live_notes().contains(60) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let x = app.tracker.area.x + TIME_WIDTH + COLUMN_WIDTH + 1;
    let y = app.tracker.area.y + 3;
    assert_eq!(terminal.backend().buffer()[(x, y)].bg, LIVE_NOTE);
    assert!(text(&terminal).contains("MIDI: C4"));
    assert!(!app.dirty);
}
