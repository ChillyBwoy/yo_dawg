//! Application state and coordination between input, editing, playback, and UI.

mod dialogs;
mod editing;
mod file_browser;
mod history;
mod input;
mod recording;
mod routing;
pub mod tracker;
mod viewport;

use crate::{
    audio::{AudioSink, Instruments},
    engine::{Command, Engine, NoteSnapshot},
    model::{Note, PPQ, Project},
};
pub use dialogs::{Action, InstrumentPicker, Prompt};
pub use file_browser::{BrowserFocus, FileAction, FileBrowser};
use ratatui::layout::Rect;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::atomic::Ordering,
};
pub use tracker::{TrackerState, ViewMode};

#[derive(Clone)]
struct Drag {
    index: usize,
    tick: u32,
    key: u8,
    original: Note,
    resize: bool,
    group: Vec<(usize, Note)>,
}

pub struct App {
    pub project: Project,
    pub engine: Engine,
    pub audio: AudioSink,
    pub instruments: Instruments,
    pub view: ViewMode,
    pub tracker: TrackerState,
    pub track: usize,
    pub cursor: u32,
    pub key: u8,
    pub grid: u32,
    pub cell_width: u16,
    pub offset: u32,
    pub top_key: u8,
    pub selected: HashSet<usize>,
    pub clipboard: Vec<Note>,
    pub status: String,
    pub dirty: bool,
    pub path: PathBuf,
    pub prompt: Option<Prompt>,
    pub file_browser: Option<FileBrowser>,
    pub instrument_picker: Option<InstrumentPicker>,
    pub help: bool,
    pub settings: bool,
    pub quit_confirm: bool,
    pub quit: bool,
    pub recording: bool,
    pub thru: bool,
    pub input_ports: Vec<String>,
    pub output_ports: Vec<String>,
    pub input: Option<String>,
    pub roll: Rect,
    pub track_area: Rect,
    pub track_offset: usize,
    pub velocity_area: Rect,
    undo: Vec<Project>,
    redo: Vec<Project>,
    drag: Option<Drag>,
    recording_notes: HashMap<(usize, u8), (u32, u8)>,
    last_file_directory: Option<PathBuf>,
}

impl App {
    pub fn new(
        project: Project,
        engine: Engine,
        audio: AudioSink,
        path: PathBuf,
        instruments: Instruments,
    ) -> Self {
        let mut app = Self {
            project,
            engine,
            audio,
            instruments,
            view: ViewMode::PianoRoll,
            tracker: TrackerState::default(),
            path,
            track: 0,
            cursor: 0,
            key: 60,
            grid: PPQ / 4,
            cell_width: 3,
            offset: 0,
            top_key: 72,
            selected: HashSet::new(),
            clipboard: Vec::new(),
            status: "Space: play | Enter: draw | ?: help | F2: routing".into(),
            dirty: false,
            prompt: None,
            file_browser: None,
            instrument_picker: None,
            help: false,
            settings: false,
            quit_confirm: false,
            quit: false,
            recording: false,
            thru: false,
            input_ports: Vec::new(),
            output_ports: Vec::new(),
            input: None,
            roll: Rect::default(),
            track_area: Rect::default(),
            track_offset: 0,
            velocity_area: Rect::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            drag: None,
            recording_notes: HashMap::new(),
            last_file_directory: None,
        };
        app.refresh_ports();
        app
    }

    pub fn position(&self) -> u32 {
        self.engine.position.load(Ordering::Relaxed)
    }
    pub fn playing(&self) -> bool {
        self.engine.playing.load(Ordering::Relaxed)
    }
    pub fn live_notes(&self) -> NoteSnapshot {
        self.engine.live_notes(self.track)
    }
    pub fn instrument_label(&self) -> String {
        let t = &self.project.tracks[self.track];
        self.instruments.label(t.program, self.track, t.route)
    }
    pub fn sync(&self) {
        self.engine.send(Command::Project(self.project.clone()));
    }

    pub fn select_track(&mut self, index: usize) {
        self.finish_recording();
        self.track = index.min(self.project.tracks.len() - 1);
        self.selected.clear();
        self.engine.send(Command::Selected(self.track));
        if self.view == ViewMode::Tracker {
            self.tracker.lane = 0;
            self.ensure_tracker_column_visible();
        }
    }

    fn request_quit(&mut self) {
        if self.dirty {
            self.quit_confirm = true;
        } else {
            self.quit = true;
        }
    }
}
