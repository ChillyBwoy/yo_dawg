mod filter;
mod oscillator;
#[cfg(test)]
mod tb303_tests;
mod voice;

use nih_plug::prelude::*;
use std::sync::Arc;
use voice::{Settings, Voice};

#[derive(Params)]
struct AcidParams {
    #[id = "waveform"]
    square: BoolParam,
    #[id = "cutoff"]
    cutoff: FloatParam,
    #[id = "resonance"]
    resonance: FloatParam,
    #[id = "env_mod"]
    env_mod: FloatParam,
    #[id = "decay"]
    decay: FloatParam,
    #[id = "accent"]
    accent: FloatParam,
    #[id = "tuning"]
    tuning: FloatParam,
    #[id = "slide"]
    slide: BoolParam,
}

impl Default for AcidParams {
    fn default() -> Self {
        let unit = FloatRange::Linear { min: 0.0, max: 1.0 };
        Self {
            square: BoolParam::new("Square wave", false),
            cutoff: FloatParam::new(
                "Cutoff",
                350.0,
                FloatRange::Skewed {
                    min: 40.0,
                    max: 12_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz"),
            resonance: FloatParam::new("Resonance", 0.65, unit),
            env_mod: FloatParam::new("Env Mod", 0.6, unit),
            decay: FloatParam::new(
                "Decay",
                0.25,
                FloatRange::Linear {
                    min: 0.03,
                    max: 2.0,
                },
            )
            .with_unit(" s"),
            accent: FloatParam::new("Accent", 0.5, unit),
            tuning: FloatParam::new(
                "Tuning",
                0.0,
                FloatRange::Linear {
                    min: -12.0,
                    max: 12.0,
                },
            )
            .with_unit(" st"),
            slide: BoolParam::new("Legato slide", true),
        }
    }
}

impl AcidParams {
    fn settings(&self) -> Settings {
        Settings {
            square: self.square.value(),
            cutoff: self.cutoff.value(),
            resonance: self.resonance.value(),
            env_mod: self.env_mod.value(),
            decay: self.decay.value(),
            accent: self.accent.value(),
            tuning: self.tuning.value(),
            slide: self.slide.value(),
        }
    }
}

pub struct Tb303 {
    params: Arc<AcidParams>,
    voice: Voice,
    rate: f32,
}
impl Default for Tb303 {
    fn default() -> Self {
        Self {
            params: Arc::new(AcidParams::default()),
            voice: Voice::default(),
            rate: 48_000.0,
        }
    }
}
impl Plugin for Tb303 {
    const NAME: &'static str = "TB-303";
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
        self.rate = config.sample_rate;
        true
    }
    fn reset(&mut self) {
        self.voice = Voice::default();
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let settings = self.params.settings();
        let mut next = context.next_event();
        for (index, channels) in buffer.iter_samples().enumerate() {
            while let Some(event) = next {
                if event.timing() > index as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        self.voice.on(note, velocity, settings.slide)
                    }
                    NoteEvent::NoteOff { note, .. } => self.voice.off(note),
                    _ => {}
                }
                next = context.next_event();
            }
            let value = self.voice.sample(&settings, self.rate);
            for sample in channels {
                *sample = value;
            }
        }
        ProcessStatus::KeepAlive
    }
}
impl Vst3Plugin for Tb303 {
    const VST3_CLASS_ID: [u8; 16] = *b"YoDawgTB30300001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}
nih_export_vst3!(Tb303);
