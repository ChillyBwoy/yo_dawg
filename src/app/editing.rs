use super::App;
use crate::model::{BAR, Note, snap};
use std::collections::HashSet;

impl App {
    pub(super) fn note_at(&self, tick: u32, key: u8) -> Option<usize> {
        self.project.tracks[self.track]
            .notes
            .iter()
            .rposition(|n| n.key == key && n.start <= tick && tick < n.start + n.duration)
    }

    pub(super) fn targets(&self) -> Vec<usize> {
        if self.selected.is_empty() {
            self.note_at(self.cursor, self.key).into_iter().collect()
        } else {
            self.selected.iter().copied().collect()
        }
    }

    pub(super) fn toggle_note(&mut self) {
        if let Some(i) = self.note_at(self.cursor, self.key) {
            self.selected.clear();
            self.selected.insert(i);
            self.delete();
        } else {
            self.checkpoint();
            let notes = &mut self.project.tracks[self.track].notes;
            notes.push(Note {
                start: self.cursor,
                duration: self.grid.min(BAR * 4096 - self.cursor),
                key: self.key,
                velocity: 100,
            });
            self.selected.clear();
            self.selected.insert(notes.len() - 1);
            self.sync();
        }
    }

    pub(super) fn delete(&mut self) {
        let indices: HashSet<_> = self.targets().into_iter().collect();
        if indices.is_empty() {
            return;
        }
        self.checkpoint();
        let mut i = 0;
        self.project.tracks[self.track].notes.retain(|_| {
            let keep = !indices.contains(&i);
            i += 1;
            keep
        });
        self.selected.clear();
        self.sync();
    }

    pub(super) fn copy(&mut self) {
        self.clipboard = self
            .targets()
            .iter()
            .map(|i| self.project.tracks[self.track].notes[*i].clone())
            .collect();
        let first = self.clipboard.iter().map(|n| n.start).min().unwrap_or(0);
        for n in &mut self.clipboard {
            n.start -= first;
        }
        self.status = format!("Copied {} notes", self.clipboard.len());
    }

    pub(super) fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        self.checkpoint();
        self.selected.clear();
        let notes = &mut self.project.tracks[self.track].notes;
        for mut n in self.clipboard.clone() {
            n.start += self.cursor;
            if n.start + n.duration <= BAR * 4096 {
                self.selected.insert(notes.len());
                notes.push(n);
            }
        }
        self.sync();
    }

    pub(super) fn quantize(&mut self) {
        let indices = self.targets();
        if indices.is_empty() {
            return;
        }
        self.checkpoint();
        for i in indices {
            let n = &mut self.project.tracks[self.track].notes[i];
            n.start = snap(n.start, self.grid).min(BAR * 4096 - n.duration);
        }
        self.sync();
    }

    pub(super) fn transform(&mut self, dt: i32, dk: i16, resize: bool, velocity: i16) {
        let targets = self.targets();
        if targets.is_empty() {
            return;
        }
        // Clamp one shared delta so chords keep their spacing at the edges.
        let notes = &self.project.tracks[self.track].notes;
        let min_start = targets.iter().map(|i| notes[*i].start).min().unwrap() as i32;
        let max_end = targets
            .iter()
            .map(|i| notes[*i].start + notes[*i].duration)
            .max()
            .unwrap() as i32;
        let min_duration = targets.iter().map(|i| notes[*i].duration).min().unwrap() as i32;
        let min_key = targets.iter().map(|i| notes[*i].key).min().unwrap() as i16;
        let max_key = targets.iter().map(|i| notes[*i].key).max().unwrap() as i16;
        let dt = dt.clamp(
            if resize { 1 - min_duration } else { -min_start },
            (BAR * 4096) as i32 - max_end,
        );
        let dk = dk.clamp(-min_key, 127 - max_key);
        self.checkpoint();
        for i in targets {
            let n = &mut self.project.tracks[self.track].notes[i];
            if resize {
                n.duration = (n.duration as i32 + dt) as u32;
            } else {
                n.start = (n.start as i32 + dt) as u32;
                n.key = (n.key as i16 + dk) as u8;
            }
            n.velocity = (n.velocity as i16 + velocity).clamp(1, 127) as u8;
        }
        self.sync();
    }
}
