use super::{SharedPlugin, SynthConfig, SynthKind, lock_plugin};
use crate::{audio::AudioEvent, model::Track};
use anyhow::{Context, Result, ensure};
use std::sync::{Arc, Mutex};
use vst3_host::{AudioBuffers, MidiChannel, Vst3Host};

pub(super) const BLOCK: usize = 64;

struct Slot {
    plugin: SharedPlugin,
    config: SynthConfig,
    buffers: AudioBuffers,
    flush_buffers: AudioBuffers,
    program: u8,
    volume: f32,
    held: [bool; 128],
    failed: bool,
}

pub(super) struct Rack {
    host: Vst3Host,
    slots: Vec<Option<Slot>>,
    rate: f64,
}

impl Rack {
    pub fn new(rate: f64) -> Result<Self> {
        Ok(Self {
            host: Vst3Host::builder()
                .sample_rate(rate)
                .block_size(BLOCK)
                .input_channels(0)
                .output_channels(2)
                .build()?,
            slots: Vec::new(),
            rate,
        })
    }
    pub fn configure(&mut self, tracks: &[Track]) -> Vec<String> {
        self.slots.truncate(tracks.len());
        self.slots.resize_with(tracks.len(), || None);
        let mut errors = Vec::new();
        for (index, track) in tracks.iter().enumerate() {
            if let Err(error) = self.configure_track(index, track) {
                self.slots[index] = None;
                errors.push(format!("Track {}: {error:#}", index + 1));
            }
        }
        errors
    }
    fn configure_track(&mut self, index: usize, track: &Track) -> Result<()> {
        if !track.route.internal() {
            self.slots[index] = None;
            return Ok(());
        }
        let replace = self.slots[index]
            .as_ref()
            .is_none_or(|slot| slot.needs_reload(&track.synth));
        if replace {
            let path = track.synth.kind.path();
            let plugin = self.host.load_plugin(&path).with_context(|| {
                format!(
                    "Load {} (build plugins with cargo xtask bundle)",
                    path.display()
                )
            })?;
            ensure!(
                plugin.info().has_midi_input,
                "Selected VST3 does not accept MIDI"
            );
            let layout = plugin.audio_bus_layout()?;
            let inputs: usize = layout
                .inputs
                .iter()
                .filter(|bus| bus.active)
                .map(|bus| bus.channel_count)
                .sum();
            let outputs: usize = layout
                .outputs
                .iter()
                .filter(|bus| bus.active)
                .map(|bus| bus.channel_count)
                .sum();
            ensure!(
                outputs > 0 && outputs <= 256 && inputs <= 256,
                "Unsupported VST3 channel layout"
            );
            self.slots[index] = Some(Slot {
                plugin: Arc::new(Mutex::new(plugin)),
                config: SynthConfig::default(),
                buffers: AudioBuffers::new(inputs, outputs, BLOCK, self.rate),
                flush_buffers: AudioBuffers::new(inputs, outputs, 0, self.rate),
                program: track.program,
                volume: 1.0,
                held: [false; 128],
                failed: false,
            });
        }
        let slot = self.slots[index].as_mut().unwrap();
        let mut plugin = lock_plugin(&slot.plugin)?;
        if replace && track.synth.state.is_empty() && track.synth.kind == SynthKind::Basic {
            if let Some(path) = &track.synth.soundfont {
                // Our own Basic plugin's initialization preset; third-party state stays opaque.
                let state = serde_json::to_vec(&serde_json::json!({
                    "version": "0.1.0",
                    "params": { "program": { "i32": track.program }, "channel": { "i32": if index == 9 { 9 } else { 0 } } },
                    "fields": { "soundfont": serde_json::to_string(&path.to_string_lossy())? }
                }))?;
                plugin.load_state(&state)?;
            }
        }
        if (replace || slot.config.state != track.synth.state) && !track.synth.state.is_empty() {
            if !replace {
                plugin.stop_processing()?;
            }
            plugin.load_state(&track.synth.state)?;
        }
        for (&id, &value) in &track.synth.parameters {
            if replace || slot.config.parameters.get(&id) != Some(&value) {
                plugin.set_parameter(id, value)?;
            }
        }
        if track.synth.kind == SynthKind::Basic {
            plugin.set_parameter_by_name("Program", track.program as f64 / 127.0)?;
        }
        slot.config = track.synth.clone();
        slot.volume = track.volume as f32 / 127.0;
        slot.program = track.program;
        plugin.start_processing()?;
        Ok(())
    }
    pub fn event(&mut self, event: AudioEvent) -> Result<()> {
        match event {
            AudioEvent::Panic => {
                for slot in self.slots.iter_mut().flatten() {
                    slot.stop()?;
                }
            }
            AudioEvent::On(track, key, velocity) => {
                if let Some(Some(slot)) = self.slots.get_mut(track as usize) {
                    lock_plugin(&slot.plugin)?.send_midi_note(key, velocity, MidiChannel::Ch1)?;
                    slot.held[key as usize] = true;
                }
            }
            AudioEvent::Off(track, key) => {
                if let Some(Some(slot)) = self.slots.get_mut(track as usize) {
                    lock_plugin(&slot.plugin)?.send_midi_note_off(key, MidiChannel::Ch1)?;
                    slot.held[key as usize] = false;
                }
            }
            AudioEvent::Program(track, program) => {
                if let Some(Some(slot)) = self.slots.get_mut(track as usize) {
                    if slot.config.kind == SynthKind::Basic {
                        lock_plugin(&slot.plugin)?
                            .set_parameter_by_name("Program", program as f64 / 127.0)?;
                    }
                }
            }
            AudioEvent::Volume(track, value) => {
                if let Some(Some(slot)) = self.slots.get_mut(track as usize) {
                    slot.volume = value as f32 / 127.0;
                }
            }
        }
        Ok(())
    }
    pub fn render(&mut self, output: &mut [[f32; 2]; BLOCK]) -> Vec<String> {
        output.fill([0.0; 2]);
        let mut errors = Vec::new();
        for slot in self.slots.iter_mut().flatten().filter(|s| !s.failed) {
            slot.buffers.clear();
            let result = lock_plugin(&slot.plugin)
                .and_then(|mut plugin| Ok(plugin.process_audio(&mut slot.buffers)?));
            if let Err(error) = result {
                slot.failed = true;
                errors.push(format!("VST3 processing stopped: {error}"));
                continue;
            }
            for (i, frame) in output.iter_mut().enumerate() {
                for (channel, sample) in frame.iter_mut().enumerate() {
                    let source = channel.min(slot.buffers.outputs.len() - 1);
                    let value = slot.buffers.outputs[source][i] * slot.volume;
                    if value.is_finite() {
                        *sample += value;
                    }
                }
            }
        }
        for sample in output.iter_mut().flatten() {
            if !sample.is_finite() {
                *sample = 0.0;
            }
        }
        errors
    }
    pub fn parameters(&self, track: usize) -> Result<Vec<super::ParameterView>> {
        let slot = self
            .slots
            .get(track)
            .and_then(Option::as_ref)
            .context("Synthesizer is not loaded")?;
        let plugin = lock_plugin(&slot.plugin)?;
        Ok(plugin
            .get_parameters()?
            .into_iter()
            .filter(|p| !p.is_read_only)
            .map(|p| {
                // The processor may not have consumed the latest control yet.
                // Display the requested value immediately, not one block behind.
                let value = slot
                    .config
                    .parameters
                    .get(&p.id)
                    .copied()
                    .unwrap_or_else(|| {
                        if slot.config.kind == SynthKind::Basic && p.name == "Program" {
                            slot.program as f64 / 127.0
                        } else {
                            p.value
                        }
                    });
                let display = plugin
                    .format_parameter(p.id, value)
                    .unwrap_or_else(|_| format!("{value:.3}"));
                super::ParameterView {
                    id: p.id,
                    name: p.name,
                    value,
                    steps: p.step_count,
                    display,
                    default: p.default,
                }
            })
            .collect())
    }
    pub fn snapshot(&mut self) -> Result<Vec<(usize, Vec<u8>)>> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_mut().map(|s| (i, s)))
            .map(|(i, s)| {
                let plugin = lock_plugin(&s.plugin)?;
                let state = plugin.save_state()?;
                // Saving must not cause the next parameter edit to reload the
                // same state (which would reset voices and discard live changes).
                s.config.state = state.clone();
                Ok((i, state))
            })
            .collect()
    }
    pub fn transport(&mut self, bpm: f64, playing: bool, quarter_notes: f64) -> Result<()> {
        for slot in self.slots.iter_mut().flatten() {
            let mut plugin = lock_plugin(&slot.plugin)?;
            plugin.set_tempo(bpm)?;
            plugin.set_playing(playing)?;
            plugin.set_transport_position(vst3_host::TransportPosition {
                samples: (quarter_notes * 60.0 / bpm * self.rate) as i64,
                quarter_notes,
            })?;
        }
        Ok(())
    }

    pub fn service_control(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        for slot in &mut self.slots {
            let Some(slot) = slot else {
                continue;
            };
            if let Err(error) =
                lock_plugin(&slot.plugin).and_then(|mut p| Ok(p.service_host_requests()?))
            {
                errors.push(format!("VST3 control: {error}"));
            }
        }
        errors
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn flush_parameters(&mut self) -> Result<()> {
        // VST3's zero-sample parameter flush runs on the processing worker:
        // saving immediately after an edit must not capture stale DSP state.
        for slot in self.slots.iter_mut().flatten().filter(|s| !s.failed) {
            lock_plugin(&slot.plugin)?.process_audio(&mut slot.flush_buffers)?;
        }
        Ok(())
    }
}

impl Slot {
    fn needs_reload(&self, desired: &SynthConfig) -> bool {
        if self.config.kind != desired.kind || self.config.soundfont != desired.soundfont {
            return true;
        }
        if !self.config.state.is_empty() && desired.state.is_empty() {
            return true;
        }
        // Removing an override means returning to the original/default state,
        // not leaving that parameter at its last edited value.
        self.config
            .parameters
            .keys()
            .any(|id| !desired.parameters.contains_key(id))
    }
    fn stop(&mut self) -> Result<()> {
        let mut plugin = lock_plugin(&self.plugin)?;
        for (key, held) in self.held.iter_mut().enumerate() {
            if *held {
                plugin.send_midi_note_off(key as u8, MidiChannel::Ch1)?;
                *held = false;
            }
        }
        // Consume queued note events, then reset the processing lifecycle so
        // Panic also cuts long releases instead of merely lifting the keys.
        if !self.failed {
            plugin.process_audio(&mut self.buffers)?;
            plugin.stop_processing()?;
            plugin.start_processing()?;
        }
        Ok(())
    }
}
