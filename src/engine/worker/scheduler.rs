use super::Worker;

#[derive(Clone, Copy)]
struct Scheduled {
    tick: u32,
    key: u8,
    velocity: u8,
}

pub(super) struct TrackSchedule {
    events: Vec<Scheduled>,
    pub active: [u32; 128],
}

impl Worker {
    pub(super) fn rebuild_schedule(&mut self) {
        self.schedule.clear();
        for (index, track) in self.project.tracks.iter().enumerate() {
            let mut events = Vec::new();
            let (start, end) = self.project.track_range(index);
            if self.project.audible(index) {
                for note in &track.notes {
                    let on = note.start.max(start);
                    let off = (note.start + note.duration).min(end);
                    if on >= off {
                        continue;
                    }
                    events.push(Scheduled {
                        tick: on,
                        key: note.key,
                        velocity: note.velocity,
                    });
                    events.push(Scheduled {
                        tick: off,
                        key: note.key,
                        velocity: 0,
                    });
                }
            }
            events.sort_by_key(|event| (event.tick, event.velocity != 0));
            self.schedule.push(TrackSchedule {
                events,
                active: [0; 128],
            });
        }
    }

    pub(super) fn advance(&mut self, distance: f64) {
        if distance <= 0.0 {
            return;
        }
        for track in 0..self.schedule.len() {
            self.advance_track(track, self.track_phase(track), distance);
        }

        let to_boundary = self.project.loop_end as f64 - self.tick;
        if distance < to_boundary {
            self.tick += distance;
        } else {
            let span = (self.project.loop_end - self.project.loop_start) as f64;
            let remainder = distance - to_boundary;
            self.loop_count = self
                .loop_count
                .saturating_add(1 + (remainder / span).floor() as u64);
            self.tick = self.project.loop_start as f64 + remainder % span;
        }
        self.absolute_tick += distance;
        self.publish_track_positions();
    }

    fn advance_track(&mut self, track: usize, mut position: f64, mut distance: f64) {
        let (start, boundary) = self.project.track_range(track);
        while distance > 0.0 {
            let to_boundary = boundary as f64 - position;
            let wraps = distance >= to_boundary;
            let end = if wraps {
                boundary as f64
            } else {
                position + distance
            };
            let events = &self.schedule[track].events;
            let first = events.partition_point(|event| (event.tick as f64) < position);
            let last = events.partition_point(|event| (event.tick as f64) < end);
            for index in first..last {
                let event = self.schedule[track].events[index];
                let active = &mut self.schedule[track].active[event.key as usize];
                if event.velocity > 0 {
                    *active = active.saturating_add(1);
                    self.note(track, event.key, event.velocity, false);
                } else if *active > 0 {
                    *active -= 1;
                    if *active == 0 {
                        self.release_sequence_key(track, event.key);
                    }
                }
            }
            // Do not subtract a rounded final segment: a sub-ULP residue can
            // otherwise keep this loop running forever without advancing time.
            if !wraps {
                break;
            }
            distance -= to_boundary;
            self.stop_track_sequence(track);
            position = start as f64;
        }
    }

    fn stop_track_sequence(&mut self, track: usize) {
        for key in 0..128 {
            if self.schedule[track].active[key] > 0 {
                self.schedule[track].active[key] = 0;
                self.release_sequence_key(track, key as u8);
            }
        }
    }

    pub(super) fn sequence_key_active(&self, track: usize, key: u8) -> bool {
        self.schedule
            .get(track)
            .is_some_and(|schedule| schedule.active[key as usize] > 0)
    }
}
