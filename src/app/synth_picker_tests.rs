use super::*;
use crate::{
    audio::{Audio, Options},
    model::{Project, Track},
    vst::SynthKind,
};
use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
use ratatui::{Terminal, backend::TestBackend};

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
    let mut project = Project::demo();
    project.tracks.push(Track::new(1));
    let engine = Engine::start(project.clone(), audio.sink.clone());
    App::new(
        project,
        engine,
        audio.sink,
        "unused.json".into(),
        audio.instruments,
    )
}

fn key(app: &mut App, code: K) {
    app.key_event(KeyEvent::new(code, M::NONE));
}

#[test]
fn picker_assigns_a_synth_per_track_and_supports_back_and_undo() {
    let mut app = app();
    let notes = app.project.tracks[0].notes.clone();
    key(&mut app, K::Char('p'));
    assert!(matches!(
        app.instrument_picker.as_ref().unwrap().page,
        SynthPage::Synthesizers(_)
    ));
    key(&mut app, K::Down);
    key(&mut app, K::Enter);
    assert_eq!(app.project.tracks[0].synth.kind, SynthKind::Tb303);
    assert_eq!(app.project.tracks[1].synth.kind, SynthKind::Basic);
    assert_eq!(app.project.tracks[0].notes, notes);
    key(&mut app, K::Esc);
    assert!(matches!(
        app.instrument_picker.as_ref().unwrap().page,
        SynthPage::Synthesizers(_)
    ));
    key(&mut app, K::Esc);
    assert!(app.instrument_picker.is_none());
    app.key_event(KeyEvent::new(K::Char('z'), M::CONTROL));
    assert_eq!(app.project.tracks[0].synth.kind, SynthKind::Basic);
}

#[test]
fn picker_renders_synth_list_and_basic_empty_slots() {
    let mut app = app();
    let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
    key(&mut app, K::Char('p'));
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(text.contains("Synthesizers"));
    assert!(text.contains("Basic"));
    assert!(text.contains("TB-303"));
    key(&mut app, K::Enter);
    key(&mut app, K::End);
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(text.contains("Basic instruments"));
    assert!(text.contains("--"));
    key(&mut app, K::Enter);
    assert_eq!(app.project.tracks[0].program, 127);
}

#[test]
fn synth_panel_shortcut_stays_in_the_tui_without_changing_the_project() {
    let mut app = app();
    let project = serde_json::to_value(&app.project).unwrap();
    key(&mut app, K::F(10));
    assert!(matches!(
        app.instrument_picker.as_ref().unwrap().page,
        SynthPage::Parameters(_)
    ));
    key(&mut app, K::F(10));
    assert!(app.instrument_picker.is_none());
    key(&mut app, K::Char('p'));
    key(&mut app, K::F(10));
    assert!(app.instrument_picker.is_some());
    assert_eq!(serde_json::to_value(&app.project).unwrap(), project);
    assert!(!app.dirty);
}

#[test]
#[ignore = "requires bundled VST3 plugins"]
fn plugin_parameter_edits_and_opaque_state_are_saved_from_the_picker() {
    let _plugins = crate::vst::test_lock();
    let mut app = app();
    let (errors, _) = crossbeam_channel::bounded(32);
    app.audio.vst = Some(
        crate::vst::Client::start(
            48_000,
            app.audio.queue.clone(),
            app.audio.overflow.clone(),
            errors,
        )
        .unwrap(),
    );
    app.configure_synths();
    key(&mut app, K::Char('p'));
    key(&mut app, K::Down);
    key(&mut app, K::Enter);
    let picker = app.instrument_picker.as_mut().unwrap();
    let SynthPage::Parameters(parameters) = &picker.page else {
        panic!("expected parameters");
    };
    picker.selected = parameters.iter().position(|p| p.name == "Cutoff").unwrap();
    let parameter = parameters[picker.selected].clone();
    key(&mut app, K::Right);
    let value = app.project.tracks[0].synth.parameters[&parameter.id];
    assert!(value > parameter.value);
    app.key_event(KeyEvent::new(K::Right, M::SHIFT));
    let fine = app.project.tracks[0].synth.parameters[&parameter.id];
    assert!((fine - value - 0.001).abs() < 0.00001);
    key(&mut app, K::Char('0'));
    assert!(
        (app.project.tracks[0].synth.parameters[&parameter.id] - parameter.default).abs() < 0.00001
    );
    app.key_event(KeyEvent::new(K::Char('z'), M::CONTROL));
    assert!((app.project.tracks[0].synth.parameters[&parameter.id] - fine).abs() < 0.00001);
    key(&mut app, K::Esc);
    key(&mut app, K::Esc);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synth.yodawg.json");
    app.open_file_browser(FileAction::SaveProject);
    let browser = app.file_browser.as_mut().unwrap();
    browser.input = path.to_string_lossy().into();
    browser.selected_text = false;
    key(&mut app, K::Enter);
    assert!(app.file_browser.is_none(), "{}", app.status);
    let saved = crate::persistence::project::load(&path).unwrap();
    assert_eq!(saved.tracks[0].synth.kind, SynthKind::Tb303);
    assert!((saved.tracks[0].synth.parameters[&parameter.id] - fine).abs() < 0.001);
    assert!(!saved.tracks[0].synth.state.is_empty());
    assert_eq!(saved.tracks[1].synth.kind, SynthKind::Basic);
}

#[test]
#[ignore = "requires bundled VST3 plugins"]
fn basic_tui_program_control_updates_the_instrument_and_supports_undo() {
    let _plugins = crate::vst::test_lock();
    let mut app = app();
    let (errors, _) = crossbeam_channel::bounded(32);
    app.audio.vst = Some(
        crate::vst::Client::start(
            48_000,
            app.audio.queue.clone(),
            app.audio.overflow.clone(),
            errors,
        )
        .unwrap(),
    );
    app.configure_synths();
    key(&mut app, K::F(10));
    let picker = app.instrument_picker.as_mut().unwrap();
    let SynthPage::Parameters(parameters) = &picker.page else {
        panic!("expected parameter panel");
    };
    picker.selected = parameters.iter().position(|p| p.name == "Program").unwrap();
    key(&mut app, K::Right);
    assert_eq!(app.project.tracks[0].program, 1);
    assert!(app.instrument_label().contains("Bright_Sine"));
    key(&mut app, K::Char('0'));
    assert_eq!(app.project.tracks[0].program, 0);
    app.key_event(KeyEvent::new(K::Char('z'), M::CONTROL));
    assert_eq!(app.project.tracks[0].program, 1);
    assert_eq!(app.project.tracks[1].program, 0);
}
