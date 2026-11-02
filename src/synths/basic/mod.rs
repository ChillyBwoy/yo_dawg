#[cfg(test)]
mod basic_tests;
mod names;
mod voices;
mod waveform;

use nih_plug::prelude::*;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::sync::{Arc, Mutex};
use voices::{AudioEvent, BasicSynth};

#[derive(Params)]
struct BasicParams {
    #[id = "program"]
    program: IntParam,
    #[id = "channel"]
    channel: IntParam,
    #[persist = "soundfont"]
    soundfont: Mutex<String>,
}

impl Default for BasicParams {
    fn default() -> Self {
        Self {
            channel: IntParam::new("Channel", 0, IntRange::Linear { min: 0, max: 15 })
                .with_value_to_string(Arc::new(|channel| (channel + 1).to_string())),
            soundfont: Mutex::new(String::new()),
            program: IntParam::new("Program", 0, IntRange::Linear { min: 0, max: 127 })
                .with_value_to_string(Arc::new(|value| {
                    names::NAMES
                        .get(value as usize)
                        .copied()
                        .unwrap_or("--")
                        .into()
                })),
        }
    }
}

pub struct Basic {
    params: Arc<BasicParams>,
    synth: BasicSynth,
    rate: u32,
    sf: Option<Synthesizer>,
    sf_program: i32,
    sf_channel: i32,
}

impl Default for Basic {
    fn default() -> Self {
        Self {
            params: Arc::new(BasicParams::default()),
            synth: BasicSynth::new(48_000),
            rate: 48_000,
            sf: None,
            sf_program: -1,
            sf_channel: -1,
        }
    }
}

impl Plugin for Basic {
    const NAME: &'static str = "Basic";
    const VENDOR: &'static str = "yo_dawg";
    const URL: &'static str = "https://github.com/ChillyBwoy/yo_dawg";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    type SysExMessage = ();
    type BackgroundTask = ();
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        config: &BufferConfig,
        _: &mut impl InitContext<Self>,
    ) -> bool {
        self.rate = config.sample_rate as u32;
        self.synth = BasicSynth::new(self.rate);
        // Track volume is applied by the host, not twice in the plugin.
        self.synth.event(AudioEvent::Volume(0, 127));
        let path = self.params.soundfont.lock().unwrap().clone();
        self.sf_program = -1;
        self.sf_channel = -1;
        self.sf = if path.is_empty() {
            None
        } else {
            let loaded = std::fs::File::open(path)
                .ok()
                .and_then(|mut file| SoundFont::new(&mut file).ok())
                .and_then(|sf| {
                    Synthesizer::new(&Arc::new(sf), &SynthesizerSettings::new(self.rate as i32))
                        .ok()
                });
            if loaded.is_none() {
                return false;
            }
            loaded
        };
        true
    }
    fn reset(&mut self) {
        self.synth.event(AudioEvent::Panic);
        if let Some(sf) = &mut self.sf {
            sf.note_off_all(true);
        }
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.synth
            .event(AudioEvent::Program(0, self.params.program.value() as u8));
        let channel = self.params.channel.value();
        if let Some(sf) = &mut self.sf {
            if self.sf_program != self.params.program.value() || self.sf_channel != channel {
                if self.sf_channel != channel {
                    sf.note_off_all(true);
                }
                self.sf_channel = channel;
                self.sf_program = self.params.program.value();
                sf.process_midi_message(channel, 0xC0, self.sf_program, 0);
                sf.process_midi_message(channel, 0xB0, 7, 127);
            }
        }
        let mut next = context.next_event();
        for (index, channels) in buffer.iter_samples().enumerate() {
            while let Some(event) = next {
                if event.timing() > index as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        if let Some(sf) = &mut self.sf {
                            sf.note_on(channel, note as i32, (velocity * 127.0).round() as i32);
                        } else {
                            self.synth.event(AudioEvent::On(
                                0,
                                note,
                                (velocity * 127.0).round() as u8,
                            ));
                        }
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        if let Some(sf) = &mut self.sf {
                            sf.note_off(channel, note as i32);
                        } else {
                            self.synth.event(AudioEvent::Off(0, note));
                        }
                    }
                    _ => {}
                }
                next = context.next_event();
            }
            let mut left = [0.0];
            let mut right = [0.0];
            if let Some(sf) = &mut self.sf {
                sf.render(&mut left, &mut right);
            } else {
                self.synth.render(&mut left, &mut right);
            }
            for (channel, sample) in channels.into_iter().enumerate() {
                *sample = if channel == 0 { left[0] } else { right[0] };
            }
        }
        ProcessStatus::KeepAlive
    }
}

impl Vst3Plugin for Basic {
    const VST3_CLASS_ID: [u8; 16] = *b"YoDawgBasic00001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}
nih_export_vst3!(Basic);
