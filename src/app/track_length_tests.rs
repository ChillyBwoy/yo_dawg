use super::*;
use crate::{
    audio::{Audio, Options},
    model::{BAR, Project, Track},
};
use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};

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
    let mut p = Project::demo();
    p.tracks.push(Track::new(1));
    let engine = Engine::start(p.clone(), audio.sink.clone());
    App::new(
        p,
        engine,
        audio.sink,
        "unused.json".into(),
        audio.instruments,
    )
}

fn length(app: &mut App, key: char, value: &str) {
    app.key_event(KeyEvent::new(K::Char(key), M::NONE));
    for c in value.chars() {
        app.key_event(KeyEvent::new(K::Char(c), M::NONE));
    }
    app.key_event(KeyEvent::new(K::Enter, M::NONE));
}

#[test]
fn each_track_has_its_own_length_and_project_length_is_separate() {
    let mut app = app();
    let notes = app.project.tracks[0].notes.clone();
    length(&mut app, 'l', "1");
    assert_eq!(app.project.tracks[0].length, Some(BAR));
    assert_eq!(app.project.tracks[1].length, None);
    assert_eq!(app.project.loop_end, BAR * 4);
    assert_eq!(app.project.tracks[0].notes, notes);
    app.select_track(1);
    app.set_view(ViewMode::Tracker);
    length(&mut app, 'l', "3");
    assert_eq!(app.project.tracks[1].length, Some(BAR * 3));
    assert_eq!(app.project.tracks[0].length, Some(BAR));
    length(&mut app, 'L', "5");
    assert_eq!(app.project.loop_end, BAR * 5);
    assert_eq!(app.project.track_length(0), BAR);
    assert_eq!(app.project.track_length(1), BAR * 3);
    length(&mut app, 'l', "0");
    assert_eq!(app.project.track_length(1), BAR * 5);
    assert_eq!(app.project.tracks[1].length, None);
}

#[test]
fn length_edits_undo_and_support_tick_lengths() {
    let mut app = app();
    length(&mut app, 'l', "480t");
    assert_eq!(app.project.tracks[0].length, Some(PPQ));
    app.key_event(KeyEvent::new(K::Char('z'), M::CONTROL));
    assert_eq!(app.project.tracks[0].length, None);
    length(&mut app, 'l', "5000");
    assert_eq!(app.project.tracks[0].length, None);
    assert!(app.status.contains("Error"));
}
