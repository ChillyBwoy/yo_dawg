use crate::{
    app::App,
    audio::{Audio, Options},
    engine::Engine,
    model::{BAR, Project, Track},
};
use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
use ratatui::layout::Rect;

fn scrolled_app() -> App {
    let (errors, _) = crossbeam_channel::bounded(1);
    let audio = Audio::open(
        &Options {
            disabled: true,
            ..Default::default()
        },
        errors,
    )
    .unwrap();
    let mut project = Project::demo();
    project.tracks.push(Track::new(1));
    let engine = Engine::start(project.clone(), audio.sink.clone());
    let mut app = App::new(
        project,
        engine,
        audio.sink,
        "unused.json".into(),
        audio.instruments,
    );
    app.roll = Rect::new(24, 3, 96, 24);
    app.offset = BAR * 8;
    app.top_key = 100;
    app
}

#[test]
fn shortcuts_preserve_manual_scroll() {
    let mut app = scrolled_app();
    let shortcuts = [
        (K::Char(' '), M::NONE),
        (K::Tab, M::NONE),
        (K::Char('m'), M::NONE),
        (K::Char('s'), M::NONE),
        (K::Char('r'), M::NONE),
        (K::Char('['), M::NONE),
        (K::Char(']'), M::NONE),
        (K::Char('+'), M::NONE),
        (K::Char('-'), M::NONE),
        (K::Char('p'), M::NONE),
        (K::Char('l'), M::NONE),
        (K::Char('b'), M::NONE),
        (K::F(2), M::NONE),
        (K::Char('?'), M::NONE),
        (K::Char('z'), M::NONE),
        (K::Char('s'), M::CONTROL),
        (K::Char('a'), M::CONTROL),
        (K::Char('z'), M::CONTROL),
        (K::Right, M::CONTROL),
        (K::Up, M::CONTROL),
        (K::Right, M::SHIFT),
        (K::Up, M::SHIFT),
    ];
    for (key, modifiers) in shortcuts {
        app.prompt = None;
        app.file_browser = None;
        app.instrument_picker = None;
        app.help = false;
        app.settings = false;
        let viewport = (app.offset, app.top_key);
        app.key_event(KeyEvent::new(key, modifiers));
        assert_eq!(
            (app.offset, app.top_key),
            viewport,
            "shortcut {key:?} / {modifiers:?} reset scrolling"
        );
    }
}

#[test]
fn horizontal_navigation_preserves_vertical_scroll() {
    let mut app = scrolled_app();
    app.key_event(KeyEvent::new(K::Right, M::NONE));
    assert_eq!(app.cursor, app.grid);
    assert!(app.offset <= app.cursor);
    assert_eq!(app.top_key, 100);
    app.key_event(KeyEvent::new(K::Home, M::NONE));
    assert_eq!(app.offset, 0);
    assert_eq!(app.top_key, 100);
}

#[test]
fn vertical_navigation_preserves_horizontal_scroll() {
    let mut app = scrolled_app();
    for key in [K::Up, K::Down, K::PageUp, K::PageDown] {
        app.key_event(KeyEvent::new(key, M::NONE));
        assert_eq!(app.offset, BAR * 8);
        let rows = app.roll.height.saturating_sub(4) as u8;
        assert!(app.key <= app.top_key && app.key >= app.top_key.saturating_sub(rows - 1));
    }
}
