use crossbeam_queue::ArrayQueue;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug)]
pub enum AudioEvent {
    On(u8, u8, u8),
    Off(u8, u8),
    Program(u8, u8),
    Volume(u8, u8),
    Panic,
}

#[derive(Clone)]
pub struct AudioSink {
    pub queue: Arc<ArrayQueue<AudioEvent>>,
    pub overflow: Arc<AtomicBool>,
    pub monitor: Arc<AtomicBool>,
}

impl AudioSink {
    pub fn send(&self, event: AudioEvent) {
        if self.queue.push(event).is_err() {
            self.overflow.store(true, Ordering::Relaxed);
        }
    }
}
