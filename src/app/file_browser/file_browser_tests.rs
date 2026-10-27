use super::*;
use crate::{
    audio::{Audio, Options},
    engine::Engine,
    model::{BAR, Project},
    persistence::project,
};
use crossterm::event::{KeyCode as K, KeyModifiers as M};
use ratatui::{Terminal, backend::TestBackend};
use std::{
    fs,
    time::{Duration, Instant},
};
use tempfile::{TempDir, tempdir};

fn fixtures() -> TempDir {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("folder")).unwrap();
    project::save(&Project::demo(), &dir.path().join("score.yodawg.json")).unwrap();
    fs::write(dir.path().join("notes.txt"), "not a project").unwrap();
    fs::write(dir.path().join("clip.MIDI"), b"placeholder").unwrap();
    dir
}

fn browser(dir: &Path, action: FileAction) -> FileBrowser {
    FileBrowser::new(action, dir, &dir.join("score.yodawg.json")).unwrap()
}

fn key(browser: &mut FileBrowser, code: K) -> Outcome {
    browser.handle_key(KeyEvent::new(code, M::NONE)).unwrap()
}

fn select(browser: &mut FileBrowser, name: &str) {
    let index = browser
        .explorer
        .files()
        .iter()
        .position(|f| f.name == name)
        .unwrap();
    browser.explorer.set_selected_idx(index);
}

fn app(dir: &Path) -> App {
    let (errors, _) = crossbeam_channel::bounded(1);
    let audio = Audio::open(
        &Options {
            disabled: true,
            ..Default::default()
        },
        errors,
    )
    .unwrap();
    let p = Project::demo();
    let engine = Engine::start(p.clone(), audio.sink.clone());
    App::new(
        p,
        engine,
        audio.sink,
        dir.join("score.yodawg.json"),
        audio.instruments,
    )
}

#[test]
fn filters_keep_directories_and_matching_extensions() {
    let dir = fixtures();
    let json = browser(dir.path(), FileAction::OpenProject);
    assert!(json.explorer.files().iter().any(|f| f.name == "folder/"));
    assert!(
        json.explorer
            .files()
            .iter()
            .any(|f| f.name == "score.yodawg.json")
    );
    assert!(
        !json
            .explorer
            .files()
            .iter()
            .any(|f| f.name == "clip.MIDI" || f.name == "notes.txt")
    );
    let midi = browser(dir.path(), FileAction::ImportMidi);
    assert!(midi.explorer.files().iter().any(|f| f.name == "clip.MIDI"));
    assert!(midi.explorer.files().iter().any(|f| f.name == "../"));
    assert!(
        !midi
            .explorer
            .files()
            .iter()
            .any(|f| f.name.ends_with(".json"))
    );
}

#[test]
fn directory_navigation_never_changes_process_working_directory() {
    let dir = fixtures();
    let cwd = std::env::current_dir().unwrap();
    let mut b = browser(dir.path(), FileAction::OpenProject);
    select(&mut b, "folder/");
    key(&mut b, K::Enter);
    assert_eq!(b.explorer.cwd(), &dir.path().join("folder"));
    key(&mut b, K::Backspace);
    assert_eq!(b.explorer.cwd(), dir.path());
    assert_eq!(std::env::current_dir().unwrap(), cwd);
    select(&mut b, "folder/");
    fs::remove_dir(dir.path().join("folder")).unwrap();
    assert!(b.handle_key(KeyEvent::new(K::Right, M::NONE)).is_err());
    assert_eq!(b.explorer.cwd(), dir.path());
    assert!(b.explorer.files().get(b.explorer.selected_idx()).is_some());
}

#[test]
fn arrow_and_page_navigation_uses_explorer_selection() {
    let dir = fixtures();
    for i in 0..30 {
        fs::write(dir.path().join(format!("item{i:02}.json")), "{}").unwrap();
    }
    let mut b = browser(dir.path(), FileAction::OpenProject);
    key(&mut b, K::Home);
    key(&mut b, K::Down);
    assert_eq!(b.explorer.selected_idx(), 1);
    key(&mut b, K::PageDown);
    assert_eq!(b.explorer.selected_idx(), 13);
    key(&mut b, K::PageUp);
    assert_eq!(b.explorer.selected_idx(), 1);
    key(&mut b, K::Up);
    assert_eq!(b.explorer.selected_idx(), 0);
    key(&mut b, K::End);
    assert_eq!(b.explorer.selected_idx(), b.explorer.files().len() - 1);
}

#[test]
fn save_adds_extensions_and_requires_explicit_overwrite_confirmation() {
    let dir = fixtures();
    let path = dir.path().join("score.yodawg.json");
    let original = fs::read(&path).unwrap();
    let mut b = browser(dir.path(), FileAction::SaveProject);
    assert!(matches!(key(&mut b, K::Enter), Outcome::Continue));
    assert_eq!(b.overwrite.as_deref(), Some(path.as_path()));
    assert!(matches!(key(&mut b, K::Enter), Outcome::Continue));
    key(&mut b, K::Char('n'));
    assert!(b.overwrite.is_none());
    assert_eq!(fs::read(&path).unwrap(), original);
    key(&mut b, K::Enter);
    assert!(matches!(key(&mut b, K::Char('y')), Outcome::Selected(p) if p == path));
    b.input = "new song".into();
    assert!(
        matches!(key(&mut b, K::Enter), Outcome::Selected(p) if p == dir.path().join("new song.yodawg.json"))
    );
    b.input = "wrong.txt".into();
    assert!(b.handle_key(KeyEvent::new(K::Enter, M::NONE)).is_err());
    b.action = FileAction::ExportMidi;
    b.input = "new song".into();
    assert!(
        matches!(key(&mut b, K::Enter), Outcome::Selected(p) if p == dir.path().join("new song.mid"))
    );
}

#[test]
fn typed_paths_and_selected_files_can_be_opened() {
    let dir = fixtures();
    let path = dir.path().join("score.yodawg.json");
    let mut b = browser(dir.path(), FileAction::OpenProject);
    assert!(matches!(key(&mut b, K::Enter), Outcome::Selected(p) if p == path));
    b.handle_key(KeyEvent::new(K::Char('l'), M::CONTROL))
        .unwrap();
    for c in path.to_string_lossy().chars() {
        key(&mut b, K::Char(c));
    }
    assert!(matches!(key(&mut b, K::Enter), Outcome::Selected(p) if p == path));
    assert!(matches!(key(&mut b, K::Esc), Outcome::Cancel));
}

#[test]
fn app_save_export_import_and_open_use_the_browser() {
    let dir = fixtures();
    let mut app = app(dir.path());
    app.project.bpm = 133;
    app.dirty = true;
    app.open_file_browser(FileAction::SaveProject);
    app.key_event(KeyEvent::new(K::Enter, M::NONE));
    assert!(app.file_browser.as_ref().unwrap().overwrite.is_some());
    app.key_event(KeyEvent::new(K::Char('y'), M::NONE));
    assert!(app.file_browser.is_none());
    assert!(!app.dirty);
    assert_eq!(project::load(&app.path).unwrap().bpm, 133);
    app.open_file_browser(FileAction::ExportMidi);
    app.key_event(KeyEvent::new(K::Enter, M::NONE));
    assert!(app.file_browser.is_none());
    assert!(dir.path().join("score.mid").is_file());
    app.open_file_browser(FileAction::ImportMidi);
    app.key_event(KeyEvent::new(K::Enter, M::NONE));
    assert!(app.file_browser.is_none());
    assert!(app.dirty);
    assert_eq!(app.project.tracks[0].notes.len(), 8);
    app.dirty = false;
    app.open_file_browser(FileAction::OpenProject);
    app.key_event(KeyEvent::new(K::Enter, M::NONE));
    assert!(app.file_browser.is_none());
    assert!(!app.dirty);
    assert_eq!(app.project.bpm, 133);
}

#[test]
fn cancel_preserves_scroll_project_and_playback_and_remembers_directory() {
    let dir = fixtures();
    let mut app = app(dir.path());
    app.offset = BAR * 8;
    app.top_key = 100;
    app.engine.send(crate::engine::Command::Play(true));
    let deadline = Instant::now() + Duration::from_secs(1);
    while !app.playing() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    app.open_file_browser(FileAction::OpenProject);
    select(app.file_browser.as_mut().unwrap(), "folder/");
    app.key_event(KeyEvent::new(K::Enter, M::NONE));
    app.key_event(KeyEvent::new(K::Char(' '), M::NONE));
    assert!(app.playing());
    app.key_event(KeyEvent::new(K::Esc, M::NONE));
    assert_eq!((app.offset, app.top_key), (BAR * 8, 100));
    assert_eq!(app.project.tracks[0].notes.len(), 8);
    assert!(!app.dirty);
    app.open_file_browser(FileAction::OpenProject);
    assert_eq!(
        app.file_browser.as_ref().unwrap().explorer.cwd(),
        &dir.path().join("folder")
    );
}

#[test]
fn invalid_file_keeps_browser_open_and_current_project_intact() {
    let dir = fixtures();
    fs::write(dir.path().join("broken.json"), "not JSON").unwrap();
    let mut app = app(dir.path());
    app.open_file_browser(FileAction::OpenProject);
    select(app.file_browser.as_mut().unwrap(), "broken.json");
    app.key_event(KeyEvent::new(K::Enter, M::NONE));
    assert!(app.file_browser.as_ref().unwrap().error.is_some());
    assert_eq!(app.project.tracks[0].notes.len(), 8);
}

#[test]
fn browser_renders_with_updated_ratatui() {
    let dir = fixtures();
    let mut app = app(dir.path());
    let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    app.offset = BAR * 8;
    app.top_key = 100;
    app.open_file_browser(FileAction::SaveProject);
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("Save project"));
    assert!(text.contains("score.yodawg.json"));
    assert_eq!((app.offset, app.top_key), (BAR * 8, 100));
}

#[test]
fn midi_import_has_an_unambiguous_function_key_shortcut() {
    let dir = fixtures();
    let mut app = app(dir.path());
    app.key_event(KeyEvent::new(K::F(3), M::NONE));
    assert_eq!(
        app.file_browser.as_ref().unwrap().action,
        FileAction::ImportMidi
    );
}
