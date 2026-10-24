//! Audio device lifecycle, real-time rendering, synthesis, and input monitoring.

mod devices;
mod events;
mod instruments;
mod monitor;
mod renderer;
mod streams;
mod synth;

use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::Sender;
use crossbeam_queue::ArrayQueue;
pub use devices::list_devices;
pub use events::{AudioEvent, AudioSink};
pub use instruments::Instruments;
use renderer::Renderer;
use rustysynth::SoundFont;
use std::sync::{Arc, atomic::AtomicBool};
use streams::{build_input, build_output};

pub struct Audio {
    pub sink: AudioSink,
    pub instruments: Instruments,
    _output: Option<cpal::Stream>,
    _input: Option<cpal::Stream>,
}

#[derive(Default)]
pub struct Options {
    pub disabled: bool,
    pub output: Option<usize>,
    pub input: Option<usize>,
    pub soundfont: Option<std::path::PathBuf>,
}

impl Audio {
    pub fn open(options: &Options, errors: Sender<String>) -> Result<Self> {
        let soundfont = options
            .soundfont
            .as_ref()
            .map(|path| -> Result<_> {
                Ok(Arc::new(
                    SoundFont::new(&mut std::fs::File::open(path)?).context("Read SoundFont")?,
                ))
            })
            .transpose()?;
        let instruments = soundfont
            .as_ref()
            .map(|sf| Instruments::from_soundfont(sf))
            .unwrap_or_default();
        let sink = AudioSink {
            queue: Arc::new(ArrayQueue::new(4096)),
            overflow: Arc::new(AtomicBool::new(false)),
            monitor: Arc::new(AtomicBool::new(options.input.is_some())),
        };
        if options.disabled {
            return Ok(Self {
                sink,
                instruments,
                _output: None,
                _input: None,
            });
        }
        let host = cpal::default_host();
        let output = match options.output {
            Some(i) => host
                .output_devices()?
                .nth(i)
                .context("Audio output index not found")?,
            None => host
                .default_output_device()
                .context("No default audio output; try --no-audio")?,
        };
        let config = output.default_output_config()?;
        let input_queue = Arc::new(ArrayQueue::new(32768));
        let mut input_rate = config.sample_rate().0;
        let input_stream = if let Some(index) = options.input {
            let input = host
                .input_devices()?
                .nth(index)
                .context("Audio input index not found")?;
            let cfg = input.default_input_config()?;
            input_rate = cfg.sample_rate().0;
            let stream = match cfg.sample_format() {
                cpal::SampleFormat::F32 => {
                    build_input::<f32>(&input, &cfg.into(), input_queue.clone(), errors.clone())?
                }
                cpal::SampleFormat::I16 => {
                    build_input::<i16>(&input, &cfg.into(), input_queue.clone(), errors.clone())?
                }
                cpal::SampleFormat::U16 => {
                    build_input::<u16>(&input, &cfg.into(), input_queue.clone(), errors.clone())?
                }
                other => bail!("Unsupported input sample format: {other}"),
            };
            stream.play()?;
            Some(stream)
        } else {
            None
        };
        let renderer = Renderer::new(
            config.sample_rate().0,
            input_rate,
            soundfont,
            sink.clone(),
            input_queue,
        )?;
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                build_output::<f32>(&output, &config.into(), renderer, errors)?
            }
            cpal::SampleFormat::I16 => {
                build_output::<i16>(&output, &config.into(), renderer, errors)?
            }
            cpal::SampleFormat::U16 => {
                build_output::<u16>(&output, &config.into(), renderer, errors)?
            }
            other => bail!("Unsupported output sample format: {other}"),
        };
        stream.play()?;
        Ok(Self {
            sink,
            instruments,
            _output: Some(stream),
            _input: input_stream,
        })
    }
}
