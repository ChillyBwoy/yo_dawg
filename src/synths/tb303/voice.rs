use super::{filter::Filter, oscillator};

pub(super) struct Settings {
    pub square: bool,
    pub cutoff: f32,
    pub resonance: f32,
    pub env_mod: f32,
    pub decay: f32,
    pub accent: f32,
    pub tuning: f32,
    pub slide: bool,
}

pub(super) struct Voice {
    held: [u64; 128],
    velocities: [f32; 128],
    order: u64,
    pub key: Option<u8>,
    phase: f32,
    frequency: f32,
    envelope: f32,
    amplitude: f32,
    accented: bool,
    gliding: bool,
    filter: Filter,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            held: [0; 128],
            velocities: [0.0; 128],
            order: 0,
            key: None,
            phase: 0.0,
            frequency: 440.0,
            envelope: 0.0,
            amplitude: 0.0,
            accented: false,
            gliding: false,
            filter: Filter::default(),
        }
    }
}

impl Voice {
    pub fn on(&mut self, key: u8, velocity: f32, slide: bool) {
        let legato = self.key.is_some() && slide;
        self.order += 1;
        self.held[key as usize] = self.order;
        self.velocities[key as usize] = velocity;
        self.key = Some(key);
        self.gliding = legato;
        self.accented = velocity >= 100.0 / 127.0;
        if !legato {
            self.frequency = frequency(key);
            self.envelope = 1.0;
        }
    }
    pub fn off(&mut self, key: u8) {
        self.held[key as usize] = 0;
        if self.key != Some(key) {
            return;
        }
        self.key = self
            .held
            .iter()
            .enumerate()
            .filter(|(_, order)| **order > 0)
            .max_by_key(|(_, order)| **order)
            .map(|(key, _)| key as u8);
        if let Some(key) = self.key {
            self.accented = self.velocities[key as usize] >= 100.0 / 127.0;
        }
    }
    pub fn sample(&mut self, settings: &Settings, rate: f32) -> f32 {
        let target = self.key.map(frequency).unwrap_or(self.frequency);
        if self.gliding {
            self.frequency += (target - self.frequency) * (1.0 - (-1.0 / (0.06 * rate)).exp());
        } else {
            self.frequency = target;
        }
        let gate = if self.key.is_some() { 1.0 } else { 0.0 };
        let time = if gate > self.amplitude { 0.003 } else { 0.03 };
        self.amplitude += (gate - self.amplitude) * (1.0 - (-1.0 / (time * rate)).exp());
        self.envelope *= (-1.0 / (settings.decay * rate)).exp();
        let accent = if self.accented { settings.accent } else { 0.0 };
        let cutoff = (settings.cutoff
            * 2.0f32.powf(self.envelope * (settings.env_mod * 5.0 + accent)))
        .min(18_000.0);
        let step = (self.frequency * 2.0f32.powf(settings.tuning / 12.0) / (rate * 2.0)).min(0.45);
        let mut output = 0.0;
        for _ in 0..2 {
            let raw = oscillator::sample(self.phase, step, settings.square);
            self.phase = (self.phase + step).fract();
            output += self
                .filter
                .sample(raw, cutoff, settings.resonance, rate * 2.0);
        }
        output * 0.15 * self.amplitude * (1.0 + accent * 0.5)
    }
}

fn frequency(key: u8) -> f32 {
    440.0 * 2.0f32.powf((key as f32 - 69.0) / 12.0)
}
