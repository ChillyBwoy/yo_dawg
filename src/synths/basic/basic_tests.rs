use super::{voices::*, *};

#[test]
fn builtin_programs_render_distinct_finite_audio() {
    let mut sounds = Vec::new();
    for program in 0..names::NAMES.len() as u8 {
        let mut synth = BasicSynth::new(48_000);
        synth.event(AudioEvent::Program(0, program));
        synth.event(AudioEvent::On(0, 60, 100));
        let mut left = vec![0.0; 12_000];
        let mut right = vec![0.0; left.len()];
        synth.render(&mut left, &mut right);
        assert_eq!(left, right);
        assert!(left.iter().all(|x| x.is_finite() && x.abs() < 1.0));
        assert!(left.iter().any(|x| x.abs() > 0.001));
        for previous in &sounds {
            assert_ne!(&left, previous, "program {program} repeats another timbre");
        }
        sounds.push(left);
        synth.event(AudioEvent::Off(0, 60));
        let mut left = vec![0.0; 24_000];
        let mut right = vec![0.0; left.len()];
        synth.render(&mut left, &mut right);
        assert!(left.last().unwrap().abs() < 0.0001);
    }
}

#[test]
fn original_four_timbres_are_unchanged() {
    for (program, harmonic) in [0.2, 0.5, 0.8, 0.0].into_iter().enumerate() {
        let angle = 0.123 * std::f32::consts::TAU;
        let expected = angle.sin() + harmonic * (angle * 2.0).sin();
        assert_eq!(
            waveform::sample(program as u8, 0.123, 440.0, 48_000.0, 0.1),
            expected
        );
    }
}
