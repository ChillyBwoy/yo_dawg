use super::App;
use crate::{
    engine::Event,
    model::{Note, snap},
};

impl App {
    pub fn poll_engine(&mut self) {
        while let Ok(event) = self.engine.events.try_recv() {
            match event {
                Event::Message(text) => self.status = text,
                Event::Recorded {
                    key,
                    velocity,
                    tick,
                    track,
                } => {
                    if !self.recording || track >= self.project.tracks.len() {
                        continue;
                    }
                    if velocity > 0 {
                        self.recording_notes.insert((track, key), (tick, velocity));
                    } else if let Some((start, v)) = self.recording_notes.remove(&(track, key)) {
                        self.record_note(track, key, start, tick, v);
                    }
                }
            }
        }
    }

    fn record_note(&mut self, track: usize, key: u8, start: u32, end: u32, velocity: u8) {
        self.checkpoint();
        let wrapped = end < start;
        let start =
            snap(start, self.grid).clamp(self.project.loop_start, self.project.loop_end - 1);
        let end = if wrapped {
            self.project.loop_end
        } else {
            snap(end, self.grid).min(self.project.loop_end)
        };
        self.project.tracks[track].notes.push(Note {
            start,
            duration: end.saturating_sub(start).max(1),
            key,
            velocity,
        });
        self.sync();
    }

    pub(super) fn finish_recording(&mut self) {
        let tick = self.position();
        let pending = std::mem::take(&mut self.recording_notes);
        for ((track, key), (start, v)) in pending {
            self.record_note(track, key, start, tick, v);
        }
    }
}
