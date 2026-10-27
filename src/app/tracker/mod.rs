//! Tracker-specific viewport and controls; all musical data stays in Project.

mod editing;
pub use editing::pitch_name;
mod input;
pub mod projection;
#[cfg(test)]
mod tracker_tests;

use super::App;
use crate::model::{BAR, Note};
use projection::Projection;
use ratatui::layout::Rect;

pub const TIME_WIDTH: u16 = 12;
pub const COLUMN_WIDTH: u16 = 24;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    PianoRoll,
    Tracker,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::PianoRoll => "Piano roll",
            Self::Tracker => "Tracker",
        }
    }
}

struct EditTarget {
    track: usize,
    index: Option<usize>,
}

#[derive(Default)]
pub struct TrackerState {
    pub area: Rect,
    pub top_tick: u32,
    pub column_offset: usize,
    pub lane: usize,
    visited: bool,
    pub needs_reveal: bool,
    edit_target: Option<EditTarget>,
}

impl TrackerState {
    pub fn visible_rows(&self) -> u32 {
        self.area.height.saturating_sub(4).max(1) as u32
    }
    pub fn visible_columns(&self) -> usize {
        (self.area.width.saturating_sub(TIME_WIDTH + 1) / COLUMN_WIDTH).max(1) as usize
    }
}

impl App {
    pub fn set_view(&mut self, view: ViewMode) {
        self.view = view;
        if view == ViewMode::Tracker && !self.tracker.visited {
            let projection = Projection::new(&self.project, self.grid);
            if let Some(column) = projection.columns.iter().find(|c| {
                c.track == self.track && c.notes.iter().any(|i| self.selected.contains(i))
            }) {
                self.tracker.lane = column.lane;
            }
            self.tracker.visited = true;
            self.tracker.needs_reveal = true;
        }
    }

    pub fn tracker_note_index(&self) -> Option<usize> {
        let projection = Projection::new(&self.project, self.grid);
        projection
            .columns
            .get(projection.column_index(self.track, self.tracker.lane))?
            .cell(
                &self.project,
                self.cursor / self.grid * self.grid,
                self.grid,
            )
            .note_index()
    }

    pub fn prepare_tracker(&mut self, area: Rect) {
        let resized = self.tracker.area != area;
        self.tracker.area = area;
        let projection = Projection::new(&self.project, self.grid);
        let index = projection.column_index(self.track, self.tracker.lane);
        self.tracker.lane = projection.columns[index].lane;
        self.tracker.column_offset = self
            .tracker
            .column_offset
            .min(projection.columns.len().saturating_sub(1));
        if resized || self.tracker.needs_reveal {
            self.ensure_tracker_time_visible();
            self.ensure_tracker_column_visible();
            self.tracker.needs_reveal = false;
        }
    }

    pub(super) fn ensure_tracker_time_visible(&mut self) {
        let row = self.cursor / self.grid;
        let top = self.tracker.top_tick / self.grid;
        let rows = self.tracker.visible_rows();
        if row < top {
            self.tracker.top_tick = row * self.grid;
        } else if row >= top + rows {
            self.tracker.top_tick = (row + 1 - rows) * self.grid;
        }
    }

    pub(super) fn ensure_tracker_column_visible(&mut self) {
        let projection = Projection::new(&self.project, self.grid);
        let index = projection.column_index(self.track, self.tracker.lane);
        let width = self.tracker.visible_columns();
        if index < self.tracker.column_offset {
            self.tracker.column_offset = index;
        } else if index >= self.tracker.column_offset + width {
            self.tracker.column_offset = index + 1 - width;
        }
    }

    fn focus_tracker_column(&mut self, index: usize) {
        let projection = Projection::new(&self.project, self.grid);
        let column = &projection.columns[index.min(projection.columns.len() - 1)];
        let (track, lane) = (column.track, column.lane);
        if self.track != track {
            self.select_track(track);
        }
        self.tracker.lane = lane;
        self.selected.clear();
        self.focus_tracker_note();
        self.ensure_tracker_column_visible();
    }

    fn focus_tracker_note(&mut self) {
        if let Some(i) = self.tracker_note_index() {
            self.key = self.project.tracks[self.track].notes[i].key;
        }
    }

    fn move_tracker_row(&mut self, rows: i64) {
        self.cursor = ((self.cursor / self.grid) as i64 + rows)
            .clamp(0, ((BAR * 4096 - 1) / self.grid) as i64) as u32
            * self.grid;
        self.selected.clear();
        self.focus_tracker_note();
        self.ensure_tracker_time_visible();
    }

    fn note_for_tracker_edit(&self, force_new: bool) -> (Option<usize>, Note) {
        let index = if force_new {
            None
        } else {
            self.tracker_note_index()
        };
        let note = index
            .map(|i| self.project.tracks[self.track].notes[i].clone())
            .unwrap_or(Note {
                start: self.cursor,
                duration: self.grid.min(BAR * 4096 - self.cursor),
                key: self.key,
                velocity: 100,
            });
        (index, note)
    }
}
