//! Playback service: command/event channels and ownership of the worker thread.

mod live_notes;
mod messages;
mod ports;
mod worker;

use crate::{audio::AudioSink, model::Project};
use crossbeam_channel::{Receiver, Sender, bounded};
use live_notes::LiveNotes;
pub use live_notes::NoteSnapshot;
pub use messages::{Command, Event};
pub use ports::ports;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32},
    },
    thread,
};
use worker::Worker;

pub struct Engine {
    pub tx: Sender<Command>,
    pub events: Receiver<Event>,
    pub position: Arc<AtomicU32>,
    pub playing: Arc<AtomicBool>,
    live_notes: Arc<LiveNotes>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Engine {
    pub fn start(project: Project, audio: AudioSink) -> Self {
        let (tx, rx) = bounded(2048);
        let (events_tx, events) = bounded(2048);
        let position = Arc::new(AtomicU32::new(project.loop_start));
        let playing = Arc::new(AtomicBool::new(false));
        let worker = Worker::new(
            project,
            audio,
            tx.clone(),
            events_tx,
            position.clone(),
            playing.clone(),
        );
        let live_notes = worker.live_notes.clone();
        let handle = thread::spawn(move || worker.run(rx));
        Self {
            tx,
            events,
            position,
            playing,
            live_notes,
            worker: Some(handle),
        }
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.tx.send(cmd);
    }

    pub fn live_notes(&self, track: usize) -> NoteSnapshot {
        self.live_notes.snapshot(track)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
