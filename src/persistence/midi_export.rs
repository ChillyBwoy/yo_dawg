//! Standard MIDI is linear: bake independent loops into a finite arrangement.

use crate::model::{Note, PPQ, Project};
use anyhow::{Result, ensure};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use std::path::Path;

pub fn export(project: &Project, path: &Path) -> Result<()> {
    project.validate()?;
    let end = project
        .tracks
        .iter()
        .flat_map(|track| {
            let raw_end = if track.length.is_none() {
                track
                    .notes
                    .iter()
                    .map(|n| n.start + n.duration)
                    .max()
                    .unwrap_or(0)
            } else {
                0
            };
            [track.length.unwrap_or(0), raw_end]
        })
        .chain([project.loop_end])
        .max()
        .unwrap();
    let mut tracks = vec![vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(
                (60_000_000 / project.bpm as u32).into(),
            )),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        },
        TrackEvent {
            delta: end.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]];
    let mut budget = 1_000_000usize;
    for track in &project.tracks {
        let mut events = vec![
            (
                0,
                0,
                MidiMessage::ProgramChange {
                    program: track.program.into(),
                },
            ),
            (
                0,
                1,
                MidiMessage::Controller {
                    controller: 7.into(),
                    value: track.volume.into(),
                },
            ),
        ];
        if let Some(length) = track.length {
            let count = track.notes.iter().filter(|n| n.start < length).count();
            let expanded = count
                .checked_mul(end.div_ceil(length) as usize)
                .unwrap_or(usize::MAX);
            ensure!(
                expanded <= budget,
                "MIDI export exceeds one million notes; shorten the project cycle"
            );
            budget -= expanded;
            if count > 0 {
                for base in (0..end).step_by(length as usize) {
                    for note in track.notes.iter().filter(|n| n.start < length) {
                        let start = base + note.start;
                        let stop = (base + (note.start + note.duration).min(length)).min(end);
                        if start < stop {
                            append_note(&mut events, note, start, stop);
                        }
                    }
                }
            }
        } else {
            ensure!(
                track.notes.len() <= budget,
                "MIDI export exceeds one million notes"
            );
            budget -= track.notes.len();
            for note in &track.notes {
                append_note(&mut events, note, note.start, note.start + note.duration);
            }
        }
        events.sort_by_key(|e| (e.0, e.1));
        let mut output = vec![TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(track.name.as_bytes())),
        }];
        let mut previous = 0;
        for (time, _, message) in events {
            output.push(TrackEvent {
                delta: (time - previous).into(),
                kind: TrackEventKind::Midi {
                    channel: track.channel.into(),
                    message,
                },
            });
            previous = time;
        }
        output.push(TrackEvent {
            delta: (end - previous).into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        tracks.push(output);
    }
    Smf {
        header: Header::new(Format::Parallel, Timing::Metrical((PPQ as u16).into())),
        tracks,
    }
    .save(path)?;
    Ok(())
}

fn append_note(events: &mut Vec<(u32, u8, MidiMessage)>, note: &Note, start: u32, end: u32) {
    events.push((
        start,
        3,
        MidiMessage::NoteOn {
            key: note.key.into(),
            vel: note.velocity.into(),
        },
    ));
    events.push((
        end,
        2,
        MidiMessage::NoteOff {
            key: note.key.into(),
            vel: 0.into(),
        },
    ));
}

#[cfg(test)]
mod midi_export_tests {
    use super::*;
    use crate::{
        model::{BAR, Track},
        persistence::midi,
    };

    #[test]
    fn export_bakes_track_loops_and_preserves_the_finite_arrangement_end() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("independent.mid");
        let mut p = Project::default();
        p.tracks[0].length = Some(BAR);
        p.tracks[0].notes.push(Note {
            start: 0,
            duration: 120,
            key: 60,
            velocity: 100,
        });
        let mut second = Track::new(1);
        second.length = Some(BAR * 3);
        second.notes.push(Note {
            start: 0,
            duration: BAR * 4,
            key: 64,
            velocity: 100,
        });
        p.tracks.push(second);
        export(&p, &path).unwrap();
        let q = midi::import(&path).unwrap();
        assert_eq!(q.loop_end, BAR * 4);
        assert_eq!(
            q.tracks[0]
                .notes
                .iter()
                .map(|n| n.start)
                .collect::<Vec<_>>(),
            [0, BAR, BAR * 2, BAR * 3]
        );
        assert_eq!(
            q.tracks[1]
                .notes
                .iter()
                .map(|n| n.duration)
                .collect::<Vec<_>>(),
            [BAR * 3, BAR]
        );
        assert!(q.tracks.iter().all(|t| t.length.is_none()));
        assert_eq!(p.tracks[0].notes.len(), 1);
        assert_eq!(p.tracks[1].notes[0].duration, BAR * 4);
    }
}
