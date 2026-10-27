use std::sync::atomic::{AtomicU64, Ordering};

const TRACKS: usize = 16;
const CHANNELS: usize = 16;

/// Authoritative input state, independent of the bounded UI event queue.
pub(super) struct LiveNotes {
    channels: [[AtomicU64; 2]; TRACKS * CHANNELS],
}

#[derive(Clone, Copy, Default)]
pub struct NoteSnapshot([u64; 2]);

impl NoteSnapshot {
    pub fn contains(self, key: u8) -> bool {
        key < 128 && self.0[key as usize / 64] & (1 << (key % 64)) != 0
    }
}

impl Default for LiveNotes {
    fn default() -> Self {
        Self {
            channels: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU64::new(0))),
        }
    }
}

impl LiveNotes {
    pub fn set(&self, track: usize, channel: u8, key: u8, pressed: bool) {
        if track >= TRACKS || channel >= 16 || key >= 128 {
            return;
        }
        let word = &self.channels[track * CHANNELS + channel as usize][key as usize / 64];
        let bit = 1 << (key % 64);
        if pressed {
            word.fetch_or(bit, Ordering::Relaxed);
        } else {
            word.fetch_and(!bit, Ordering::Relaxed);
        }
    }

    pub fn channel(&self, track: usize, channel: u8) -> NoteSnapshot {
        if track >= TRACKS || channel >= 16 {
            return NoteSnapshot::default();
        }
        NoteSnapshot(std::array::from_fn(|i| {
            self.channels[track * CHANNELS + channel as usize][i].load(Ordering::Relaxed)
        }))
    }

    pub fn snapshot(&self, track: usize) -> NoteSnapshot {
        if track >= TRACKS {
            return NoteSnapshot::default();
        }
        let mut words = [0; 2];
        for channel in &self.channels[track * CHANNELS..(track + 1) * CHANNELS] {
            for i in 0..2 {
                words[i] |= channel[i].load(Ordering::Relaxed);
            }
        }
        NoteSnapshot(words)
    }

    pub fn clear(&self) {
        for channel in &self.channels {
            for word in channel {
                word.store(0, Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_scoped_to_tracks_and_channels() {
        let notes = LiveNotes::default();
        for key in [0, 60, 64, 127] {
            notes.set(0, 0, key, true);
        }
        assert!(notes.snapshot(0).contains(0));
        assert!(notes.snapshot(0).contains(127));
        assert!(!notes.snapshot(1).contains(60));
        notes.set(0, 1, 60, true);
        notes.set(0, 0, 60, false);
        assert!(notes.snapshot(0).contains(60));
        notes.set(0, 1, 60, false);
        assert!(!notes.snapshot(0).contains(60));
        assert!(notes.snapshot(0).contains(64));
        notes.clear();
        assert!(!notes.snapshot(0).contains(64));
        assert!(!notes.snapshot(0).contains(127));
    }
}
