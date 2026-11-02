/// Saturating four-pole low-pass. This is an acid-inspired model, not a
/// component-level reconstruction of Roland's diode ladder.
#[derive(Default)]
pub(super) struct Filter {
    stages: [f32; 4],
}

impl Filter {
    pub fn sample(&mut self, input: f32, cutoff: f32, resonance: f32, rate: f32) -> f32 {
        let g = (1.0 - (-std::f32::consts::TAU * cutoff.min(rate * 0.12) / rate).exp()).min(0.55);
        let mut value = (input * 1.4 - resonance * 3.7 * self.stages[3]).tanh();
        for stage in &mut self.stages {
            *stage += g * (value - stage.tanh());
            value = stage.tanh();
        }
        value
    }
}
