use super::AudioEvent;

#[derive(Clone, Copy, Default)]
struct Voice {
    track: u8,
    key: u8,
    phase: f32,
    level: f32,
    release: bool,
}

pub(super) struct BasicSynth {
    voices: [Voice; 128],
    programs: [u8; 16],
    volumes: [f32; 16],
    sample_rate: f32,
}

impl BasicSynth {
    pub fn new(rate: u32) -> Self {
        Self {
            voices: [Voice::default(); 128],
            programs: [0; 16],
            volumes: [100.0 / 127.0; 16],
            sample_rate: rate as f32,
        }
    }

    pub fn event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::On(c, k, v) => {
                let index = self
                    .voices
                    .iter()
                    .position(|x| x.level <= 0.0001)
                    .unwrap_or_else(|| {
                        self.voices
                            .iter()
                            .enumerate()
                            .min_by(|(_, a), (_, b)| a.level.total_cmp(&b.level))
                            .unwrap()
                            .0
                    });
                self.voices[index] = Voice {
                    track: c,
                    key: k,
                    phase: 0.0,
                    level: v as f32 / 127.0,
                    release: false,
                };
            }
            AudioEvent::Off(c, k) => {
                for v in &mut self.voices {
                    if v.track == c && v.key == k {
                        v.release = true;
                    }
                }
            }
            AudioEvent::Program(c, p) => self.programs[c as usize] = p,
            AudioEvent::Volume(c, v) => self.volumes[c as usize] = v as f32 / 127.0,
            AudioEvent::Panic => self.voices.fill(Voice::default()),
        }
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let mut value = 0.0;
            for v in &mut self.voices {
                if v.level <= 0.0001 {
                    continue;
                }
                let angle = v.phase * std::f32::consts::TAU;
                let harmonic = match self.programs[v.track as usize] % 4 {
                    0 => 0.2,
                    1 => 0.5,
                    2 => 0.8,
                    _ => 0.0,
                };
                value += (angle.sin() + harmonic * (angle * 2.0).sin())
                    * v.level
                    * self.volumes[v.track as usize]
                    * 0.08;
                v.phase = (v.phase
                    + 440.0 * 2.0f32.powf((v.key as f32 - 69.0) / 12.0) / self.sample_rate)
                    .fract();
                if v.release {
                    v.level *= (-1.0 / (0.025 * self.sample_rate)).exp();
                }
            }
            *l = value;
            *r = value;
        }
    }
}
