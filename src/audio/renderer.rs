use super::{AudioEvent, AudioSink, monitor::Monitor, synth::BasicSynth};
use anyhow::Result;
use crossbeam_queue::ArrayQueue;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::sync::{Arc, atomic::Ordering};

const BLOCK_SIZE: usize = 64;

pub(super) struct Renderer {
    sf: Option<Synthesizer>,
    basic: BasicSynth,
    sink: AudioSink,
    monitor: Monitor,
    left: [f32; BLOCK_SIZE],
    right: [f32; BLOCK_SIZE],
    cursor: usize,
}

impl Renderer {
    pub fn new(
        rate: u32,
        input_rate: u32,
        soundfont: Option<Arc<SoundFont>>,
        sink: AudioSink,
        input: Arc<ArrayQueue<[f32; 2]>>,
    ) -> Result<Self> {
        let sf = soundfont
            .map(|sf| Synthesizer::new(&sf, &SynthesizerSettings::new(rate as i32)))
            .transpose()?;
        Ok(Self {
            sf,
            basic: BasicSynth::new(rate),
            sink,
            monitor: Monitor::new(input, input_rate, rate),
            left: [0.0; BLOCK_SIZE],
            right: [0.0; BLOCK_SIZE],
            cursor: BLOCK_SIZE,
        })
    }

    fn event(&mut self, event: AudioEvent) {
        if let Some(sf) = &mut self.sf {
            match event {
                AudioEvent::On(c, k, v) => sf.note_on(c as i32, k as i32, v as i32),
                AudioEvent::Off(c, k) => sf.note_off(c as i32, k as i32),
                AudioEvent::Program(c, p) => sf.process_midi_message(c as i32, 0xC0, p as i32, 0),
                AudioEvent::Volume(c, v) => sf.process_midi_message(c as i32, 0xB0, 7, v as i32),
                AudioEvent::Panic => sf.note_off_all(true),
            }
        }
        self.basic.event(event);
    }

    pub fn next(&mut self) -> [f32; 2] {
        if self.cursor == BLOCK_SIZE {
            if self.sink.overflow.swap(false, Ordering::Relaxed) {
                while self.sink.queue.pop().is_some() {}
                self.event(AudioEvent::Panic);
            }
            while let Some(event) = self.sink.queue.pop() {
                self.event(event);
            }
            if let Some(sf) = &mut self.sf {
                sf.render(&mut self.left, &mut self.right);
            } else {
                self.basic.render(&mut self.left, &mut self.right);
            }
            self.cursor = 0;
        }
        let mut value = [self.left[self.cursor], self.right[self.cursor]];
        self.cursor += 1;
        // Keep draining input even when monitoring is muted.
        let monitored = self.monitor.next();
        if self.sink.monitor.load(Ordering::Relaxed) {
            for (sample, input) in value.iter_mut().zip(monitored) {
                *sample += input;
            }
        }
        value
    }
}
