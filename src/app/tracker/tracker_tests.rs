use super::*;
use crate::{
    audio::{Audio, Options},
    engine::{Command, Engine},
    model::{PPQ, Project, Track},
};
use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

fn project() -> Project {
    let mut p = Project::default();
    p.tracks.push(Track::new(1));
    p.tracks[0].notes = vec![
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
        Note {
            start: 0,
            duration: PPQ,
            key: 67,
            velocity: 100,
        },
        Note {
            start: 120,
            duration: 360,
            key: 72,
            velocity: 85,
        },
    ];
    p
}

fn app(p: Project) -> App {
    let (errors, _) = crossbeam_channel::bounded(1);
    let audio = Audio::open(
        &Options {
            disabled: true,
            ..Default::default()
        },
        errors,
    )
    .unwrap();
    let engine = Engine::start(p.clone(), audio.sink.clone());
    let mut app = App::new(
        p,
        engine,
        audio.sink,
        "unused.json".into(),
        audio.instruments,
    );
    app.roll = Rect::new(24, 3, 96, 24);
    app
}

fn key(app: &mut App, key: K, modifiers: M) {
    app.key_event(KeyEvent::new(key, modifiers));
}
fn tracker(app: &mut App) {
    app.set_view(ViewMode::Tracker);
    app.prepare_tracker(Rect::new(0, 3, 120, 24));
}
fn choose_note(app: &mut App, index: usize) {
    let projection = Projection::new(&app.project, app.grid);
    app.tracker.lane = projection
        .columns
        .iter()
        .find(|c| c.track == app.track && c.notes.contains(&index))
        .unwrap()
        .lane;
    app.cursor = app.project.tracks[app.track].notes[index].start / app.grid * app.grid;
}

#[test]
fn projection_preserves_chords_off_grid_starts_and_every_note() {
    let p = project();
    let original = serde_json::to_value(&p).unwrap();
    for grid in [30, 120, 480] {
        let projection = Projection::new(&p, grid);
        let mut seen = Vec::new();
        for column in &projection.columns {
            for &index in &column.notes {
                seen.push((column.track, index));
                let tick = p.tracks[column.track].notes[index].start / grid * grid;
                assert_eq!(column.cell(&p, tick, grid), projection::Cell::Start(index));
            }
        }
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 4);
        let minimum_columns = if grid >= 120 { 3 } else { 2 };
        assert!(projection.columns.iter().filter(|c| c.track == 0).count() >= minimum_columns);
        assert!(
            projection
                .columns
                .iter()
                .any(|c| c.track == 1 && c.notes.is_empty())
        );
        assert_eq!(serde_json::to_value(&p).unwrap(), original);
    }
}

#[test]
fn simultaneous_chord_notes_have_independent_columns() {
    let mut p = Project::default();
    p.tracks[0].notes = [60, 64, 67]
        .into_iter()
        .map(|key| Note {
            start: 0,
            duration: PPQ,
            key,
            velocity: 100,
        })
        .collect();
    let projection = Projection::new(&p, 120);
    assert_eq!(projection.columns.len(), 3);
    for (index, column) in projection.columns.iter().enumerate() {
        assert_eq!(column.cell(&p, 0, 120), projection::Cell::Start(index));
        assert_eq!(column.cell(&p, 120, 120), projection::Cell::Hold(index));
        assert_eq!(column.cell(&p, 480, 120), projection::Cell::Off);
    }
}

#[test]
fn switching_views_changes_neither_project_history_selection_nor_playback() {
    let mut app = app(project());
    app.selected.insert(0);
    let original = serde_json::to_value(&app.project).unwrap();
    app.engine.send(Command::Play(true));
    app.engine.send(Command::Live(vec![0x90, 60, 100]));
    let deadline = Instant::now() + Duration::from_secs(1);
    while !app.playing() || !app.live_notes().contains(60) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    for _ in 0..4 {
        key(&mut app, K::F(4), M::NONE);
    }
    assert_eq!(app.view, ViewMode::PianoRoll);
    assert_eq!(serde_json::to_value(&app.project).unwrap(), original);
    assert_eq!(app.selected.len(), 1);
    assert!(!app.dirty && app.undo.is_empty());
    assert!(app.playing() && app.live_notes().contains(60));
}

#[test]
fn tracker_navigation_moves_through_time_and_polyphonic_columns() {
    let mut app = app(project());
    tracker(&mut app);
    key(&mut app, K::Down, M::NONE);
    assert_eq!(app.cursor, 120);
    key(&mut app, K::Up, M::NONE);
    assert_eq!(app.cursor, 0);
    key(&mut app, K::Right, M::NONE);
    assert_eq!((app.track, app.tracker.lane), (0, 1));
    assert_eq!(app.key, 60);
    key(&mut app, K::PageDown, M::NONE);
    assert_eq!(app.cursor, app.tracker.visible_rows() * app.grid);
    key(&mut app, K::Home, M::NONE);
    assert_eq!(app.cursor, app.project.loop_start);
    key(&mut app, K::End, M::NONE);
    assert_eq!(app.cursor, (app.project.loop_end - 1) / app.grid * app.grid);
}

#[test]
fn common_edits_target_off_grid_notes_and_undo_works_across_views() {
    let mut app = app(project());
    tracker(&mut app);
    choose_note(&mut app, 0);
    key(&mut app, K::Right, M::CONTROL);
    assert_eq!(app.project.tracks[0].notes[0].duration, 151);
    assert_eq!(app.project.tracks[0].notes[0].start, 17);
    key(&mut app, K::Up, M::CONTROL);
    assert_eq!(app.project.tracks[0].notes[0].velocity, 95);
    key(&mut app, K::F(4), M::NONE);
    key(&mut app, K::Char('z'), M::CONTROL);
    key(&mut app, K::Char('z'), M::CONTROL);
    assert_eq!(app.project.tracks[0].notes[0], project().tracks[0].notes[0]);
}

#[test]
fn exact_note_editor_and_extra_chord_voice_share_the_project() {
    let mut app = app(project());
    tracker(&mut app);
    choose_note(&mut app, 0);
    key(&mut app, K::Char('e'), M::NONE);
    assert_eq!(app.prompt.as_ref().unwrap().text, "C4 90 31 17");
    for c in "C#4 99 137 19".chars() {
        key(&mut app, K::Char(c), M::NONE);
    }
    key(&mut app, K::Enter, M::NONE);
    assert_eq!(
        app.project.tracks[0].notes[0],
        Note {
            start: 19,
            duration: 137,
            key: 61,
            velocity: 99
        }
    );
    key(&mut app, K::Enter, M::SHIFT);
    for c in "D4 88 240 19".chars() {
        key(&mut app, K::Char(c), M::NONE);
    }
    key(&mut app, K::Enter, M::NONE);
    assert_eq!(app.project.tracks[0].notes.len(), 5);
    assert_eq!(app.project.tracks[0].notes[4].key, 62);
    assert_eq!(app.project.tracks[0].notes[0].key, 61);
    key(&mut app, K::F(4), M::NONE);
    assert_eq!(app.key, 62);
    assert!(app.selected.contains(&4));
}

#[test]
fn invalid_or_canceled_editor_does_not_change_notes() {
    let mut app = app(project());
    tracker(&mut app);
    let original = serde_json::to_value(&app.project).unwrap();
    key(&mut app, K::Char('e'), M::NONE);
    for c in "C4 255 0 0".chars() {
        key(&mut app, K::Char(c), M::NONE);
    }
    key(&mut app, K::Enter, M::NONE);
    assert_eq!(serde_json::to_value(&app.project).unwrap(), original);
    assert!(!app.dirty);
    key(&mut app, K::Char('e'), M::NONE);
    key(&mut app, K::Esc, M::NONE);
    assert_eq!(serde_json::to_value(&app.project).unwrap(), original);
}

#[test]
fn tracker_mouse_scroll_and_view_switches_preserve_independent_viewports() {
    let mut app = app(project());
    app.offset = BAR * 8;
    app.top_key = 100;
    tracker(&mut app);
    app.tracker.top_tick = BAR * 2;
    app.tracker.column_offset = 1;
    app.tracker_mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 40,
        row: 8,
        modifiers: M::NONE,
    });
    let tick = app.tracker.top_tick;
    assert_eq!(tick, BAR * 2 + app.grid * 3);
    key(&mut app, K::F(4), M::NONE);
    assert_eq!((app.offset, app.top_key), (BAR * 8, 100));
    key(&mut app, K::F(4), M::NONE);
    app.prepare_tracker(Rect::new(0, 3, 120, 24));
    assert_eq!((app.tracker.top_tick, app.tracker.column_offset), (tick, 1));
    key(&mut app, K::Char('p'), M::NONE);
    key(&mut app, K::F(4), M::NONE);
    assert_eq!(app.view, ViewMode::Tracker); // A modal owns its keys.
}
