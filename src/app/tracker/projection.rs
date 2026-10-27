//! A lossless display projection. Note indices refer to the unchanged project.

use crate::model::Project;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Start(usize),
    Hold(usize),
    Off,
    Empty,
}

impl Cell {
    pub fn note_index(self) -> Option<usize> {
        match self {
            Self::Start(i) | Self::Hold(i) => Some(i),
            _ => None,
        }
    }
}

pub struct Column {
    pub track: usize,
    pub lane: usize,
    pub notes: Vec<usize>,
}

impl Column {
    pub fn cell(&self, project: &Project, tick: u32, grid: u32) -> Cell {
        let notes = &project.tracks[self.track].notes;
        for &i in &self.notes {
            let n = &notes[i];
            if n.start >= tick && n.start < tick + grid {
                return Cell::Start(i);
            }
        }
        for &i in &self.notes {
            let n = &notes[i];
            let end = n.start + n.duration;
            if n.start < tick && end >= tick && end < tick + grid {
                return Cell::Off;
            }
            if n.start < tick && end > tick {
                return Cell::Hold(i);
            }
        }
        Cell::Empty
    }
}

pub struct Projection {
    pub columns: Vec<Column>,
}

impl Projection {
    pub fn new(project: &Project, grid: u32) -> Self {
        let mut columns = Vec::new();
        for (track, t) in project.tracks.iter().enumerate() {
            let first = columns.len();
            let mut ends = Vec::new();
            let mut indices: Vec<_> = (0..t.notes.len()).collect();
            indices.sort_by_key(|&i| (t.notes[i].start, t.notes[i].key, i));
            for i in indices {
                let n = &t.notes[i];
                let row = n.start / grid * grid;
                let lane = ends
                    .iter()
                    .position(|&end| end <= row)
                    .unwrap_or(ends.len());
                if lane == ends.len() {
                    ends.push(0);
                    columns.push(Column {
                        track,
                        lane,
                        notes: Vec::new(),
                    });
                }
                // Even short notes sharing a display row need separate columns.
                ends[lane] = (n.start + n.duration).max(row + grid);
                columns[first + lane].notes.push(i);
            }
            if ends.is_empty() {
                columns.push(Column {
                    track,
                    lane: 0,
                    notes: Vec::new(),
                });
            }
        }
        Self { columns }
    }

    pub fn column_index(&self, track: usize, lane: usize) -> usize {
        self.columns
            .iter()
            .position(|c| c.track == track && c.lane == lane)
            .or_else(|| self.columns.iter().position(|c| c.track == track))
            .unwrap_or(0)
    }
}
