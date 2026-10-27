use super::*;
use crossbeam_channel::bounded;

fn worker() -> Worker {
    let (errors, _) = bounded(1);
    let audio = crate::audio::Audio::open(
        &crate::audio::Options {
            disabled: true,
            ..Default::default()
        },
        errors,
    )
    .unwrap();
    let (tx, _) = bounded(1);
    let (events, _) = bounded(1);
    Worker::new(
        Project::default(),
        audio.sink.clone(),
        tx,
        events,
        Arc::new(AtomicU32::new(0)),
        Arc::new(AtomicBool::new(false)),
    )
}

#[test]
fn advance_finishes_with_sub_ulp_residue() {
    let mut w = worker();
    w.tick = 480.37;
    w.advance(1.23456789);
    assert!((w.tick - 481.60456789).abs() < 1e-10);
    // Even a positive distance too small to change tick must terminate.
    w.advance(5.995204332975845e-15);
    assert!((w.tick - 481.60456789).abs() < 1e-10);
}

#[test]
fn advance_wraps_and_preserves_remainder() {
    let mut w = worker();
    w.project.loop_start = 480;
    w.project.loop_end = 1920;
    w.tick = 1919.5;
    w.advance(1.25);
    assert_eq!(w.tick, 480.75);
    w.tick = 1919.5;
    w.advance(0.5);
    assert_eq!(w.tick, 480.0);
    w.advance(2880.25);
    assert_eq!(w.tick, 480.25);
}

#[test]
fn incoming_notes_are_visible_without_playback_or_recording() {
    let mut w = worker();
    w.live(&[0x90, 60, 100]);
    w.live(&[0x90, 64, 80]);
    assert!(w.live_notes.snapshot(0).contains(60));
    assert!(w.live_notes.snapshot(0).contains(64));
    w.live(&[0x80, 60, 64]);
    assert!(!w.live_notes.snapshot(0).contains(60));
    assert!(w.live_notes.snapshot(0).contains(64));
    w.live(&[0x90, 64, 0]);
    assert!(!w.live_notes.snapshot(0).contains(64));
}

#[test]
fn input_channel_reset_does_not_clear_other_channels() {
    let mut w = worker();
    w.live(&[0x90, 60, 100]);
    w.live(&[0x91, 60, 100]);
    w.live(&[0x90, 64, 100]);
    w.live(&[0xB0, 123, 0]);
    assert!(w.live_notes.snapshot(0).contains(60));
    assert!(!w.live_notes.snapshot(0).contains(64));
    w.live(&[0xB1, 120, 0]);
    assert!(!w.live_notes.snapshot(0).contains(60));
}

#[test]
fn invalid_midi_does_not_change_indicators() {
    let mut w = worker();
    for data in [
        &[0x90, 60][..],
        &[0x90, 128, 100],
        &[0x90, 60, 128],
        &[0xB0, 7, 100],
    ] {
        w.live(data);
    }
    assert!(!w.live_notes.snapshot(0).contains(60));
}

#[test]
fn loop_wrap_keeps_held_key_indicators() {
    let mut w = worker();
    w.live(&[0x90, 60, 100]);
    w.tick = w.project.loop_end as f64 - 0.5;
    w.advance(1.0);
    assert!(w.live_notes.snapshot(0).contains(60));
    w.connect_input(None);
    assert!(!w.live_notes.snapshot(0).contains(60));
}
