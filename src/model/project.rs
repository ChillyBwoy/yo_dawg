use super::{BAR, Note, PPQ, Track};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub bpm: u16,
    pub loop_start: u32,
    pub loop_end: u32,
    pub tracks: Vec<Track>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            version: 1,
            bpm: 120,
            loop_start: 0,
            loop_end: BAR * 4,
            tracks: vec![Track::new(0)],
        }
    }
}

impl Project {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            bail!("Unsupported project version {}", self.version);
        }
        if !(20..=300).contains(&self.bpm) {
            bail!("BPM must be 20..300");
        }
        if self.loop_end <= self.loop_start || self.loop_end > BAR * 4096 {
            bail!("Invalid loop boundaries (maximum 4096 bars)");
        }
        if self.tracks.is_empty() || self.tracks.len() > 16 {
            bail!("Expected 1..16 tracks");
        }
        for t in &self.tracks {
            if t.channel > 15 || t.program > 127 || t.volume > 127 {
                bail!("Invalid track parameters");
            }
            for n in &t.notes {
                if n.key > 127
                    || n.velocity == 0
                    || n.velocity > 127
                    || n.duration == 0
                    || n.start.checked_add(n.duration).is_none()
                    || n.start + n.duration > BAR * 4096
                {
                    bail!("Invalid note in {}", t.name);
                }
            }
        }
        Ok(())
    }
    pub fn audible(&self, index: usize) -> bool {
        !self.tracks[index].mute && (!self.tracks.iter().any(|t| t.solo) || self.tracks[index].solo)
    }
    pub fn demo() -> Self {
        let mut p = Self::default();
        for (i, key) in [60, 64, 67, 72, 67, 64, 62, 67].into_iter().enumerate() {
            p.tracks[0].notes.push(Note {
                start: i as u32 * PPQ,
                duration: PPQ * 3 / 4,
                key,
                velocity: 95,
            });
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::snap;
    #[test]
    fn validation() {
        let mut p = Project::demo();
        assert!(p.validate().is_ok());
        p.tracks[0].notes[0].duration = 0;
        assert!(p.validate().is_err());
    }
    #[test]
    fn solo_mute() {
        let mut p = Project::default();
        p.tracks.push(Track::new(1));
        p.tracks[1].solo = true;
        assert!(!p.audible(0));
        assert!(p.audible(1));
        p.tracks[1].mute = true;
        assert!(!p.audible(1));
    }
    #[test]
    fn quantization() {
        assert_eq!(snap(179, 120), 120);
        assert_eq!(snap(181, 120), 240);
    }
}
