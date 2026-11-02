use super::*;

#[test]
fn voice_uses_last_note_priority_and_returns_to_held_notes() {
    let mut voice = Voice::default();
    voice.on(48, 0.7, true);
    voice.on(60, 1.0, true);
    assert_eq!(voice.key, Some(60));
    voice.off(60);
    assert_eq!(voice.key, Some(48));
    voice.off(48);
    assert_eq!(voice.key, None);
}

#[test]
fn filter_stays_finite_at_extreme_settings_and_releases() {
    let mut settings = AcidParams::default().settings();
    settings.resonance = 1.0;
    settings.env_mod = 1.0;
    settings.cutoff = 12_000.0;
    for rate in [44_100.0, 48_000.0, 96_000.0] {
        for square in [false, true] {
            settings.square = square;
            let mut voice = Voice::default();
            voice.on(100, 1.0, true);
            let mut peak = 0.0f32;
            for _ in 0..rate as usize {
                let value = voice.sample(&settings, rate);
                assert!(value.is_finite() && value.abs() < 1.0);
                peak = peak.max(value.abs());
            }
            assert!(peak > 0.001);
            voice.off(100);
            let mut tail = 0.0;
            for _ in 0..rate as usize {
                tail = voice.sample(&settings, rate);
            }
            assert!(tail.abs() < 0.0001);
        }
    }
}
