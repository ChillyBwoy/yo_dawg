use super::App;
use crate::{
    engine::{Event, TrackPosition},
    model::{Note, snap},
};

pub(super) struct PendingRecording {
    start: TrackPosition,
    velocity: u8,
}

#[cfg(test)]
mod recording_tests {
    use super::*;
    use crate::{
        audio::{Audio, Options},
        engine::Engine,
        model::{BAR, Project, Track},
    };

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
        let mut p = Project::default();
        p.tracks[0].length = Some(BAR);
        p.tracks.push(Track::new(1));
        p.tracks[1].length = Some(BAR * 5);
        let engine = Engine::start(p.clone(), audio.sink.clone());
        App::new(
            p,
            engine,
            audio.sink,
            "unused.json".into(),
            audio.instruments,
        )
    }

    #[test]
    fn recording_uses_track_boundaries_even_after_multiple_wraps() {
        let mut app = app();
        app.record_note(
            0,
            60,
            TrackPosition {
                tick: 120,
                cycle: 0,
            },
            TrackPosition {
                tick: 1800,
                cycle: 2,
            },
            100,
        );
        let note = &app.project.tracks[0].notes[0];
        assert_eq!(note.start + note.duration, BAR);
        app.record_note(
            1,
            64,
            TrackPosition {
                tick: BAR * 4 + 120,
                cycle: 0,
            },
            TrackPosition {
                tick: BAR * 4 + 360,
                cycle: 0,
            },
            90,
        );
        assert_eq!(app.project.tracks[1].notes[0].start, BAR * 4 + 120);
    }

    #[test]
    fn length_change_does_not_discard_a_pending_recorded_note() {
        use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
        let mut app = app();
        app.recording_notes.insert(
            (0, 60),
            PendingRecording {
                start: TrackPosition {
                    tick: 120,
                    cycle: 0,
                },
                velocity: 100,
            },
        );
        app.key_event(KeyEvent::new(K::Char('l'), M::NONE));
        app.key_event(KeyEvent::new(K::Char('2'), M::NONE));
        app.key_event(KeyEvent::new(K::Enter, M::NONE));
        assert_eq!(app.project.tracks[0].length, Some(BAR * 2));
        assert_eq!(app.project.tracks[0].notes.len(), 1);
    }
}

impl App {
    pub fn poll_engine(&mut self) {
        while let Ok(event) = self.engine.events.try_recv() {
            match event {
                Event::Message(text) => self.status = text,
                Event::Recorded {
                    key,
                    velocity,
                    tick,
                    cycle,
                    track,
                } => {
                    if !self.recording || track >= self.project.tracks.len() {
                        continue;
                    }
                    if velocity > 0 {
                        self.recording_notes.insert(
                            (track, key),
                            PendingRecording {
                                start: TrackPosition { tick, cycle },
                                velocity,
                            },
                        );
                    } else if let Some(pending) = self.recording_notes.remove(&(track, key)) {
                        self.record_note(
                            track,
                            key,
                            pending.start,
                            TrackPosition { tick, cycle },
                            pending.velocity,
                        );
                    }
                }
            }
        }
    }

    fn record_note(
        &mut self,
        track: usize,
        key: u8,
        start: TrackPosition,
        end: TrackPosition,
        velocity: u8,
    ) {
        self.checkpoint();
        let (region_start, region_end) = self.project.track_range(track);
        let wrapped = end.cycle != start.cycle || end.tick < start.tick;
        let start = snap(start.tick, self.grid).clamp(region_start, region_end - 1);
        let end = if wrapped {
            region_end
        } else {
            snap(end.tick, self.grid).min(region_end)
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
        let pending = std::mem::take(&mut self.recording_notes);
        for ((track, key), pending) in pending {
            let position = self.engine.track_position(track);
            self.record_note(track, key, pending.start, position, pending.velocity);
        }
    }
}
