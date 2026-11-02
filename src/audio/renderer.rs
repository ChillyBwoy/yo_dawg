use super::{AudioSink, monitor::Monitor};
use anyhow::Result;
use crossbeam_queue::ArrayQueue;
use std::sync::{Arc, atomic::Ordering};

pub(super) struct Renderer {
    sink: AudioSink,
    monitor: Monitor,
}

impl Renderer {
    pub fn new(
        rate: u32,
        input_rate: u32,
        sink: AudioSink,
        input: Arc<ArrayQueue<[f32; 2]>>,
    ) -> Result<Self> {
        Ok(Self {
            sink,
            monitor: Monitor::new(input, input_rate, rate),
        })
    }
    pub fn next(&mut self) -> [f32; 2] {
        let mut value = self.sink.vst.as_ref().map_or([0.0; 2], |vst| vst.next());
        // Always drain input, including while monitoring is muted.
        let monitored = self.monitor.next();
        if self.sink.monitor.load(Ordering::Relaxed) {
            for (sample, input) in value.iter_mut().zip(monitored) {
                *sample += input;
            }
        }
        value
    }
}
