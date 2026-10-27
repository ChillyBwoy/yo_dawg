use super::{Command, Event, Worker};
use crate::audio::AudioEvent;
use anyhow::Result;
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::atomic::Ordering;

impl Worker {
    pub(super) fn configure_outputs(&mut self) {
        let desired: Vec<_> = self
            .project
            .tracks
            .iter()
            .filter(|t| t.route.external())
            .filter_map(|t| t.midi_output.clone())
            .collect();
        self.outputs.retain(|name, _| desired.contains(name));
        for name in desired {
            if self.outputs.contains_key(&name) {
                continue;
            }
            let connect = || -> Result<MidiOutputConnection> {
                let midi = MidiOutput::new("yo_dawg output")?;
                let port = midi
                    .ports()
                    .into_iter()
                    .find(|p| midi.port_name(p).ok().as_ref() == Some(&name))
                    .ok_or_else(|| anyhow::anyhow!("MIDI output not found: {name}"))?;
                Ok(midi
                    .connect(&port, "yo_dawg sequencer")
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?)
            };
            match connect() {
                Ok(c) => {
                    self.outputs.insert(name, c);
                }
                Err(e) => self.message(e.to_string()),
            }
        }
    }

    pub(super) fn configure_instruments(&mut self) {
        for i in 0..self.project.tracks.len() {
            let t = &self.project.tracks[i];
            self.audio.send(AudioEvent::Program(i as u8, t.program));
            self.audio.send(AudioEvent::Volume(i as u8, t.volume));
            let (channel, program, volume) = (t.channel, t.program, t.volume);
            self.external(i, &[0xC0 | channel, program]);
            self.external(i, &[0xB0 | channel, 7, volume]);
        }
    }

    fn external(&mut self, track: usize, bytes: &[u8]) {
        let t = &self.project.tracks[track];
        if !t.route.external() {
            return;
        }
        if let Some(name) = &t.midi_output {
            if let Some(output) = self.outputs.get_mut(name) {
                if let Err(e) = output.send(bytes) {
                    self.message(format!("MIDI send: {e}; refresh ports with F5"));
                }
            }
        }
    }

    pub(super) fn note(&mut self, track: usize, key: u8, velocity: u8, live: bool) {
        if track >= self.project.tracks.len() || !self.project.audible(track) {
            return;
        }
        let t = &self.project.tracks[track];
        if t.route.internal() {
            self.audio.send(if velocity > 0 {
                AudioEvent::On(track as u8, key, velocity)
            } else {
                AudioEvent::Off(track as u8, key)
            });
        }
        // External live thru is opt-in: FM-1 can already sound locally.
        if !live || self.thru {
            self.external(
                track,
                &[
                    if velocity > 0 { 0x90 } else { 0x80 } | t.channel,
                    key,
                    velocity,
                ],
            );
        }
    }

    pub(super) fn panic(&mut self) {
        self.audio.send(AudioEvent::Panic);
        for output in self.outputs.values_mut() {
            for channel in 0..16 {
                let _ = output.send(&[0xB0 | channel, 64, 0]);
                let _ = output.send(&[0xB0 | channel, 123, 0]);
                let _ = output.send(&[0xB0 | channel, 120, 0]);
            }
        }
    }

    pub(super) fn connect_input(&mut self, name: Option<String>) {
        self.live_notes.clear();
        self.panic();
        self.input = None;
        let Some(name) = name else {
            self.message("MIDI input disconnected");
            return;
        };
        let connect = || -> Result<MidiInputConnection<()>> {
            let mut midi = MidiInput::new("yo_dawg input")?;
            midi.ignore(Ignore::None);
            let port = midi
                .ports()
                .into_iter()
                .find(|p| midi.port_name(p).ok().as_ref() == Some(&name))
                .ok_or_else(|| anyhow::anyhow!("MIDI input not found: {name}"))?;
            let tx = self.tx.clone();
            let dropped = self.dropped.clone();
            Ok(midi
                .connect(
                    &port,
                    "yo_dawg record",
                    move |_, bytes, _| {
                        if bytes.len() <= 3 && tx.try_send(Command::Live(bytes.to_vec())).is_err() {
                            dropped.store(true, Ordering::Relaxed);
                        }
                    },
                    (),
                )
                .map_err(|e| anyhow::anyhow!(e.to_string()))?)
        };
        match connect() {
            Ok(c) => {
                self.input = Some(c);
                self.message(format!("MIDI input: {name}"));
            }
            Err(e) => self.message(e.to_string()),
        }
    }

    pub(super) fn live(&mut self, data: &[u8]) {
        if data.len() != 3 || data[1] > 127 || data[2] > 127 {
            return;
        }
        let status = data[0] & 0xF0;
        let channel = data[0] & 0x0F;
        if status == 0xB0 && matches!(data[1], 120 | 123) {
            let notes = self.live_notes.channel(self.selected, channel);
            for key in 0..128 {
                if notes.contains(key) {
                    self.live(&[0x80 | channel, key, 0]);
                }
            }
            return;
        }
        if status != 0x90 && status != 0x80 {
            return;
        }
        let velocity = if status == 0x90 { data[2] } else { 0 };
        self.live_notes
            .set(self.selected, channel, data[1], velocity > 0);
        if velocity > 0 || !self.live_notes.snapshot(self.selected).contains(data[1]) {
            self.note(self.selected, data[1], velocity, true);
        }
        if self.record && self.playing.load(Ordering::Relaxed) {
            let _ = self.events.try_send(Event::Recorded {
                key: data[1],
                velocity,
                tick: self.tick as u32,
                track: self.selected,
            });
        }
    }
}
