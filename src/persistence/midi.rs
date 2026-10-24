use crate::model::{BAR, Note, PPQ, Project, Track};
use anyhow::{Result, bail};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use std::{collections::HashMap, path::Path};

pub fn export(project: &Project, path: &Path) -> Result<()> {
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
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]];
    for t in &project.tracks {
        let mut events = vec![
            (
                0,
                0,
                MidiMessage::ProgramChange {
                    program: t.program.into(),
                },
            ),
            (
                0,
                1,
                MidiMessage::Controller {
                    controller: 7.into(),
                    value: t.volume.into(),
                },
            ),
        ];
        for n in &t.notes {
            events.push((
                n.start,
                3,
                MidiMessage::NoteOn {
                    key: n.key.into(),
                    vel: n.velocity.into(),
                },
            ));
            events.push((
                n.start + n.duration,
                2,
                MidiMessage::NoteOff {
                    key: n.key.into(),
                    vel: 0.into(),
                },
            ));
        }
        events.sort_by_key(|e| (e.0, e.1));
        let mut output = vec![TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(t.name.as_bytes())),
        }];
        let mut prev = 0;
        for (time, _, message) in events {
            output.push(TrackEvent {
                delta: (time - prev).into(),
                kind: TrackEventKind::Midi {
                    channel: t.channel.into(),
                    message,
                },
            });
            prev = time;
        }
        output.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });
        tracks.push(output);
    }
    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical((PPQ as u16).into())),
        tracks,
    };
    smf.save(path)?;
    Ok(())
}

pub fn import(path: &Path) -> Result<Project> {
    let bytes = std::fs::read(path)?;
    decode(&bytes)
}

fn decode(bytes: &[u8]) -> Result<Project> {
    let smf = Smf::parse(bytes)?;
    if smf.header.format == Format::Sequential {
        bail!("Sequential MIDI files are not supported");
    }
    let Timing::Metrical(ppq) = smf.header.timing else {
        bail!("SMPTE MIDI timing is not supported");
    };
    if ppq.as_int() == 0 {
        bail!("Invalid MIDI resolution");
    }
    let scale = |ticks: u64| -> u32 {
        (ticks * PPQ as u64 / ppq.as_int() as u64).min((BAR * 4096) as u64) as u32
    };
    let mut p = Project::default();
    p.tracks.clear();
    let mut end = BAR;
    let mut tempo_seen = None;
    for input in &smf.tracks {
        let mut time = 0u64;
        let mut channels: HashMap<u8, Track> = HashMap::new();
        let mut pending: HashMap<(u8, u8), Vec<(u32, u8)>> = HashMap::new();
        let mut name = String::new();
        for e in input {
            time += e.delta.as_int() as u64;
            match e.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(tempo)) => {
                    if tempo.as_int() == 0 {
                        bail!("Invalid MIDI tempo");
                    }
                    if tempo_seen.is_some_and(|t| t != tempo.as_int()) {
                        bail!("Tempo maps are not supported; use a constant-tempo MIDI file");
                    }
                    tempo_seen = Some(tempo.as_int());
                    p.bpm = (60_000_000 / tempo.as_int()).clamp(20, 300) as u16;
                }
                TrackEventKind::Meta(MetaMessage::TimeSignature(n, d, _, _))
                    if n != 4 || d != 2 =>
                {
                    bail!("Only 4/4 MIDI files are currently supported")
                }
                TrackEventKind::Meta(MetaMessage::TrackName(n)) => {
                    name = String::from_utf8_lossy(n).into_owned()
                }
                TrackEventKind::Midi { channel, message } => {
                    let c = channel.as_int();
                    let t = channels.entry(c).or_insert_with(|| {
                        let mut t = Track::new(c as usize);
                        t.channel = c;
                        t
                    });
                    match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => pending
                            .entry((c, key.as_int()))
                            .or_default()
                            .push((scale(time), vel.as_int())),
                        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                            if let Some(notes) = pending.get_mut(&(c, key.as_int())) {
                                if !notes.is_empty() {
                                    let (start, velocity) = notes.remove(0);
                                    let stop = scale(time).max(start + 1);
                                    t.notes.push(Note {
                                        start,
                                        duration: stop - start,
                                        key: key.as_int(),
                                        velocity,
                                    });
                                    end = end.max(stop);
                                }
                            }
                        }
                        MidiMessage::ProgramChange { program } => t.program = program.as_int(),
                        MidiMessage::Controller { controller, value }
                            if controller.as_int() == 7 =>
                        {
                            t.volume = value.as_int()
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if pending.values().any(|notes| !notes.is_empty()) {
            bail!("MIDI file contains unterminated notes");
        }
        let mut channels: Vec<_> = channels.into_iter().collect();
        channels.sort_by_key(|(c, _)| *c);
        for (c, mut t) in channels {
            if !name.is_empty() {
                t.name = format!("{name} [{}]", c + 1);
            }
            p.tracks.push(t);
        }
    }
    if p.tracks.is_empty() {
        p.tracks.push(Track::new(0));
    }
    p.loop_end = end.div_ceil(BAR) * BAR;
    p.validate()?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let path = std::env::temp_dir().join(format!("yo-dawg-midi-{}.mid", std::process::id()));
        let p = Project::demo();
        export(&p, &path).unwrap();
        let q = import(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(p.tracks[0].notes, q.tracks[0].notes);
        assert_eq!(p.bpm, q.bpm);
    }
}
