mod midi;
mod scheduler;
#[cfg(test)]
mod tests;

use super::{Command, Event};
use crate::{
    audio::AudioSink,
    model::{PPQ, Project},
};
use crossbeam_channel::{Receiver, Sender};
use midir::{MidiInputConnection, MidiOutputConnection};
use scheduler::Scheduled;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub(super) struct Worker {
    project: Project,
    audio: AudioSink,
    outputs: HashMap<String, MidiOutputConnection>,
    input: Option<MidiInputConnection<()>>,
    tx: Sender<Command>,
    events: Sender<Event>,
    position: Arc<AtomicU32>,
    playing: Arc<AtomicBool>,
    selected: usize,
    record: bool,
    thru: bool,
    tick: f64,
    schedule: Vec<Scheduled>,
    dropped: Arc<AtomicBool>,
}

impl Worker {
    pub(super) fn new(
        project: Project,
        audio: AudioSink,
        tx: Sender<Command>,
        events: Sender<Event>,
        position: Arc<AtomicU32>,
        playing: Arc<AtomicBool>,
    ) -> Self {
        Self {
            project,
            audio,
            outputs: HashMap::new(),
            input: None,
            tx,
            events,
            position,
            playing,
            selected: 0,
            record: false,
            thru: false,
            tick: 0.0,
            schedule: Vec::new(),
            dropped: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(super) fn run(mut self, rx: Receiver<Command>) {
        self.configure();
        self.tick = self.project.loop_start as f64;
        let mut previous = Instant::now();
        'outer: loop {
            // Limit work per pass so incoming MIDI cannot starve the transport.
            for _ in 0..512 {
                let Ok(cmd) = rx.try_recv() else {
                    break;
                };
                match cmd {
                    Command::Reconnect => {
                        self.panic();
                        self.outputs.clear();
                        self.configure();
                    }
                    Command::Shutdown => break 'outer,
                    Command::Project(p) => {
                        self.panic();
                        self.project = p;
                        self.configure();
                        self.tick = self.tick.clamp(
                            self.project.loop_start as f64,
                            (self.project.loop_end - 1) as f64,
                        );
                    }
                    Command::Play(on) => {
                        self.panic();
                        self.tick = self.project.loop_start as f64;
                        self.playing.store(on, Ordering::Relaxed);
                        previous = Instant::now();
                    }
                    Command::Panic => self.panic(),
                    Command::Input(name) => self.connect_input(name),
                    Command::Selected(i) => {
                        self.panic();
                        self.selected = i;
                    }
                    Command::Record(on) => self.record = on,
                    Command::Thru(on) => {
                        self.panic();
                        self.thru = on;
                    }
                    Command::Live(data) => self.live(&data),
                }
            }
            let now = Instant::now();
            let delta = now.duration_since(previous).as_secs_f64();
            previous = now;
            if self.dropped.swap(false, Ordering::Relaxed) {
                self.panic();
                self.message("MIDI input queue overflow: notes stopped");
            }
            if self.playing.load(Ordering::Relaxed) {
                if delta > 0.25 {
                    self.panic();
                    self.message("Transport underrun: notes stopped");
                }
                self.advance(delta.min(0.25) * self.project.bpm as f64 * PPQ as f64 / 60.0);
            }
            self.position.store(self.tick as u32, Ordering::Relaxed);
            thread::sleep(Duration::from_millis(1));
        }
        self.panic();
    }

    fn configure(&mut self) {
        self.configure_outputs();
        self.rebuild_schedule();
        self.configure_instruments();
    }

    fn message(&self, text: impl Into<String>) {
        let _ = self.events.try_send(Event::Message(text.into()));
    }
}
