use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackPosition {
    pub tick: u32,
    pub cycle: u32,
}

/// A packed position/cycle snapshot avoids mismatched reads at a loop boundary.
pub(super) struct TrackPositions([AtomicU64; 16]);

impl Default for TrackPositions {
    fn default() -> Self {
        Self(std::array::from_fn(|_| AtomicU64::new(0)))
    }
}

impl TrackPositions {
    pub fn set(&self, track: usize, position: TrackPosition) {
        if let Some(slot) = self.0.get(track) {
            slot.store(
                ((position.cycle as u64) << 32) | position.tick as u64,
                Ordering::Relaxed,
            );
        }
    }
    pub fn get(&self, track: usize) -> TrackPosition {
        let value = self
            .0
            .get(track)
            .map(|p| p.load(Ordering::Relaxed))
            .unwrap_or(0);
        TrackPosition {
            tick: value as u32,
            cycle: (value >> 32) as u32,
        }
    }
}
