fn poly_blep(phase: f32, step: f32) -> f32 {
    if phase < step {
        let t = phase / step;
        t + t - t * t - 1.0
    } else if phase > 1.0 - step {
        let t = (phase - 1.0) / step;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

pub(super) fn sample(phase: f32, step: f32, square: bool) -> f32 {
    if square {
        let raw = if phase < 0.5 { 1.0 } else { -1.0 };
        raw + poly_blep(phase, step) - poly_blep((phase + 0.5).fract(), step)
    } else {
        2.0 * phase - 1.0 - poly_blep(phase, step)
    }
}
