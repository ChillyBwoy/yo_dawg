//! Built-in additive timbres shared by synthesis and the instrument picker.

pub(super) const NAMES: [&str; 12] = [
    "Soft_Sine",
    "Bright_Sine",
    "Rich_Sine",
    "Pure_Sine",
    "Triangle",
    "Organ",
    "Reed",
    "Strings",
    "Brass",
    "Bell",
    "Electric_Piano",
    "Bass",
];

pub(super) fn sample(program: u8, phase: f32, frequency: f32, rate: f32, age: f32) -> f32 {
    let angle = phase * std::f32::consts::TAU;
    let partial = |multiple: f32| {
        if frequency * multiple < rate * 0.5 {
            (angle * multiple).sin()
        } else {
            0.0
        }
    };
    match program {
        4 => partial(1.0) - partial(3.0) / 9.0 + partial(5.0) / 25.0 - partial(7.0) / 49.0,
        5 => (partial(1.0) + 0.6 * partial(2.0) + 0.35 * partial(4.0)) / 1.5,
        6 => (partial(1.0) + 0.65 * partial(3.0) + 0.3 * partial(5.0) + 0.15 * partial(7.0)) / 1.6,
        7 => {
            (age / 0.18).min(1.0)
                * (partial(1.0)
                    + 0.4 * partial(2.0)
                    + 0.25 * partial(3.0)
                    + 0.18 * partial(4.0)
                    + 0.12 * partial(5.0))
                / 1.5
        }
        8 => {
            (age / 0.035).min(1.0)
                * (partial(1.0) + 0.7 * partial(2.0) + 0.5 * partial(3.0) + 0.3 * partial(4.0))
                / 1.8
        }
        9 => {
            partial(1.0) * (-age * 1.8).exp()
                + 0.5 * partial(2.0) * (-age * 3.0).exp()
                + 0.3 * partial(7.0) * (-age * 5.0).exp()
        }
        10 => {
            partial(1.0) * (-age * 1.2).exp()
                + 0.45 * partial(3.0) * (-age * 4.0).exp()
                + 0.2 * partial(5.0) * (-age * 7.0).exp()
        }
        11 => {
            partial(1.0) * (-age * 0.8).exp()
                + 0.55 * partial(2.0) * (-age * 5.0).exp()
                + 0.25 * partial(3.0) * (-age * 9.0).exp()
        }
        _ => {
            let harmonic = match program % 4 {
                0 => 0.2,
                1 => 0.5,
                2 => 0.8,
                _ => 0.0,
            };
            angle.sin() + harmonic * (angle * 2.0).sin()
        }
    }
}
