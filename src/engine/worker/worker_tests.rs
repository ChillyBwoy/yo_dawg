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

#[test]
fn independent_three_and_five_bar_tracks_keep_their_phases() {
    use crate::{
        audio::AudioEvent,
        model::{BAR, Note, Track},
    };
    let mut w = worker();
    w.project.tracks[0].length = Some(BAR * 3);
    w.project.tracks[0].notes.push(Note {
        start: 0,
        duration: BAR * 10,
        key: 60,
        velocity: 100,
    });
    let mut second = Track::new(1);
    second.length = Some(BAR * 5);
    second.notes.push(Note {
        start: 0,
        duration: BAR * 10,
        key: 64,
        velocity: 100,
    });
    w.project.tracks.push(second);
    w.configure();
    w.advance((BAR * 4) as f64 + 0.25);
    assert_eq!(w.tick, 0.25);
    assert_eq!(
        w.track_positions.get(0),
        TrackPosition {
            tick: BAR,
            cycle: 1
        }
    );
    assert_eq!(
        w.track_positions.get(1),
        TrackPosition {
            tick: BAR * 4,
            cycle: 0
        }
    );
    let mut first_offs = 0;
    while let Some(event) = w.audio.queue.pop() {
        assert!(!matches!(event, AudioEvent::Panic | AudioEvent::Off(1, 64)));
        if matches!(event, AudioEvent::Off(0, 60)) {
            first_offs += 1;
        }
    }
    assert_eq!(first_offs, 1);
    assert!(w.sequence_key_active(1, 64));
    w.advance(BAR as f64);
    assert_eq!(
        w.track_positions.get(0),
        TrackPosition {
            tick: BAR * 2,
            cycle: 1
        }
    );
    assert_eq!(
        w.track_positions.get(1),
        TrackPosition { tick: 0, cycle: 1 }
    );
    let mut second_offs = 0;
    while let Some(event) = w.audio.queue.pop() {
        assert!(!matches!(event, AudioEvent::Panic | AudioEvent::Off(0, 60)));
        if matches!(event, AudioEvent::Off(1, 64)) {
            second_offs += 1;
        }
    }
    assert_eq!(second_offs, 1);
    w.advance((BAR * 10) as f64 - 0.25);
    assert_eq!(
        w.track_positions.get(0),
        TrackPosition { tick: 0, cycle: 5 }
    );
    assert_eq!(
        w.track_positions.get(1),
        TrackPosition { tick: 0, cycle: 3 }
    );
}

#[test]
fn notes_outside_track_length_are_kept_but_not_scheduled() {
    use crate::{
        audio::AudioEvent,
        model::{BAR, Note},
    };
    let mut w = worker();
    w.project.tracks[0].length = Some(BAR);
    w.project.tracks[0].notes = vec![
        Note {
            start: BAR - 120,
            duration: BAR,
            key: 60,
            velocity: 100,
        },
        Note {
            start: BAR,
            duration: 120,
            key: 72,
            velocity: 100,
        },
    ];
    w.configure();
    w.advance((BAR * 2) as f64);
    let mut ons = 0;
    let mut offs = 0;
    while let Some(event) = w.audio.queue.pop() {
        assert!(!matches!(event, AudioEvent::On(_, 72, _)));
        if matches!(event, AudioEvent::On(0, 60, _)) {
            ons += 1;
        }
        if matches!(event, AudioEvent::Off(0, 60)) {
            offs += 1;
        }
    }
    assert_eq!((ons, offs), (2, 2));
    assert_eq!(w.project.tracks[0].notes.len(), 2);
}

#[test]
fn midi_record_events_use_local_track_position_and_cycle() {
    use crate::model::BAR;
    let mut w = worker();
    let (events, rx) = bounded(8);
    w.events = events;
    w.project.tracks[0].length = Some(BAR);
    w.absolute_tick = (BAR * 4 + 120) as f64;
    w.tick = 120.0;
    w.record = true;
    w.playing.store(true, Ordering::Relaxed);
    w.live(&[0x90, 60, 100]);
    assert!(matches!(
        rx.recv().unwrap(),
        Event::Recorded {
            tick: 120,
            cycle: 4,
            ..
        }
    ));
}

#[test]
fn shared_output_guard_checks_destination_activity_and_live_thru() {
    use crate::model::{Route, Track};
    let mut w = worker();
    w.project.tracks[0].midi_output = Some("FM-1".into());
    let mut other = Track::new(1);
    other.channel = 0;
    other.route = Route::External;
    other.midi_output = Some("FM-1".into());
    w.project.tracks.push(other);
    w.rebuild_schedule();
    w.schedule[1].active[60] = 1;
    assert!(w.output_key_held_elsewhere(0, 60));
    assert!(!w.output_key_held_elsewhere(0, 61));
    w.project.tracks[1].channel = 1;
    assert!(!w.output_key_held_elsewhere(0, 60));
    w.project.tracks[1].channel = 0;
    w.project.tracks[1].midi_output = Some("Other device".into());
    assert!(!w.output_key_held_elsewhere(0, 60));
    w.project.tracks[1].midi_output = Some("FM-1".into());
    w.project.tracks[1].mute = true;
    assert!(!w.output_key_held_elsewhere(0, 60));
    w.project.tracks[1].mute = false;
    w.project.tracks[1].route = Route::Internal;
    assert!(!w.output_key_held_elsewhere(0, 60));
    w.project.tracks[1].route = Route::External;
    w.schedule[1].active[60] = 0;
    w.live_notes.set(1, 0, 60, true);
    assert!(!w.output_key_held_elsewhere(0, 60));
    w.thru = true;
    assert!(w.output_key_held_elsewhere(0, 60));
}
