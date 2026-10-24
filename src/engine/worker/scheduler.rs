use super::Worker;

#[derive(Clone, Copy)]
pub(super) struct Scheduled {
    tick: u32,
    track: usize,
    key: u8,
    velocity: u8,
}

impl Worker {
    pub(super) fn rebuild_schedule(&mut self) {
        self.schedule.clear();
        for (i, track) in self.project.tracks.iter().enumerate() {
            if !self.project.audible(i) {
                continue;
            }
            for n in &track.notes {
                self.schedule.push(Scheduled {
                    tick: n.start,
                    track: i,
                    key: n.key,
                    velocity: n.velocity,
                });
                self.schedule.push(Scheduled {
                    tick: n.start + n.duration,
                    track: i,
                    key: n.key,
                    velocity: 0,
                });
            }
        }
        self.schedule.sort_by_key(|e| (e.tick, e.velocity != 0));
    }

    pub(super) fn advance(&mut self, mut distance: f64) {
        while distance > 0.0 {
            let to_boundary = self.project.loop_end as f64 - self.tick;
            let wraps = distance >= to_boundary;
            let end = if wraps {
                self.project.loop_end as f64
            } else {
                self.tick + distance
            };
            let first = self
                .schedule
                .partition_point(|e| (e.tick as f64) < self.tick);
            let last = self.schedule.partition_point(|e| (e.tick as f64) < end);
            for i in first..last {
                let e = self.schedule[i];
                self.note(e.track, e.key, e.velocity, false);
            }
            self.tick = end;
            // Subtract only whole loop segments. Subtracting (end - old_tick)
            // on the final segment can leave a positive residue smaller than
            // one ULP of tick: tick + residue == tick, and this loop never ends.
            if !wraps {
                break;
            }
            distance -= to_boundary;
            self.panic();
            self.tick = self.project.loop_start as f64;
        }
    }
}
