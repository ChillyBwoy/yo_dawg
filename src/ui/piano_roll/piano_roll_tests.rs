use super::*;
use crate::{
    audio::{Audio, Options},
    engine::{Command, Engine},
    model::Project,
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
    let project = Project::demo();
    let engine = Engine::start(project.clone(), audio.sink.clone());
    let mut app = App::new(
        project,
        engine,
        audio.sink,
        "unused.json".into(),
        audio.instruments,
    );
    app.key = 61; // Keep the cursor from covering the C4 note under test.
    app
}

fn wait_for_note(app: &App, key: u8, pressed: bool) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while app.live_notes().contains(key) != pressed {
        assert!(Instant::now() < deadline, "MIDI state did not update");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn incoming_midi_highlights_keyboard_lane_and_notes_then_releases() {
    let mut app = app();
    let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
    app.engine.send(Command::Live(vec![0x90, 60, 100]));
    wait_for_note(&app, 60, true);
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let y = app.roll.y + 3 + (app.top_key - 60) as u16;
    let key_x = app.roll.x + 1;
    let note_x = app.roll.x + 7;
    let lane_x = app.roll.x + 20;
    let buf = terminal.backend().buffer();
    assert_eq!(buf[(key_x, y)].bg, LIVE_NOTE);
    assert_eq!(buf[(note_x, y)].bg, LIVE_NOTE);
    assert_eq!(buf[(lane_x, y)].bg, LIVE_ROW);
    assert_eq!(app.project.tracks[0].notes.len(), 8);
    assert!(!app.dirty);

    app.engine.send(Command::Live(vec![0x80, 60, 0]));
    wait_for_note(&app, 60, false);
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let buf = terminal.backend().buffer();
    assert_ne!(buf[(key_x, y)].bg, LIVE_NOTE);
    assert_ne!(buf[(note_x, y)].bg, LIVE_NOTE);
    assert_ne!(buf[(lane_x, y)].bg, LIVE_ROW);
}

#[test]
fn stop_and_panic_clear_input_indicators() {
    let app = app();
    for command in [Command::Play(false), Command::Panic] {
        app.engine.send(Command::Live(vec![0x90, 60, 100]));
        wait_for_note(&app, 60, true);
        app.engine.send(command);
        wait_for_note(&app, 60, false);
    }
}
