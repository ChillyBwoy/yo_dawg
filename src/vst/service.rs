use super::rack::{BLOCK, Rack};
use crate::{audio::AudioEvent, model::Track};
use anyhow::{Result, ensure};
use crossbeam_channel::{Sender, bounded};
use crossbeam_queue::ArrayQueue;
use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle, ThreadId},
    time::Duration,
};

struct Transport {
    bpm: AtomicU64,
    quarter_notes: AtomicU64,
    playing: AtomicBool,
}

#[derive(Clone, Debug)]
pub struct ParameterView {
    pub id: u32,
    pub name: String,
    pub value: f64,
    pub steps: i32,
    pub display: String,
    pub default: f64,
}

struct Control {
    rack: Arc<Mutex<Rack>>,
    thread: ThreadId,
    worker: Mutex<Option<JoinHandle<()>>>,
    stop: Arc<AtomicBool>,
    output: Arc<ArrayQueue<[f32; 2]>>,
    transport: Arc<Transport>,
    flush: Sender<Sender<Result<()>>>,
}

impl Control {
    fn stop_worker(&self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        self.stop_worker();
    }
}

/// Control calls stay on the creating (UI) thread; only sample reads and
/// transport publication are exposed to device/transport threads.
#[derive(Clone)]
pub struct Client(Arc<Control>);

impl Client {
    pub fn start(
        rate: u32,
        events: Arc<ArrayQueue<AudioEvent>>,
        overflow: Arc<AtomicBool>,
        errors: Sender<String>,
    ) -> Result<Self> {
        let rack = Arc::new(Mutex::new(Rack::new(rate as f64)?));
        let output = Arc::new(ArrayQueue::new(BLOCK * 32));
        let stop = Arc::new(AtomicBool::new(false));
        let transport = Arc::new(Transport {
            bpm: AtomicU64::new(120.0f64.to_bits()),
            quarter_notes: AtomicU64::new(0),
            playing: AtomicBool::new(false),
        });
        let processing = rack.clone();
        let frames = output.clone();
        let stopping = stop.clone();
        let position = transport.clone();
        let (flush, requests) = bounded::<Sender<Result<()>>>(1);
        let worker = thread::Builder::new()
            .name("vst3-audio".into())
            .spawn(move || {
                let report = |message| {
                    let _ = errors.try_send(message);
                };
                let mut block = [[0.0; 2]; BLOCK];
                while !stopping.load(Ordering::Acquire) {
                    while let Ok(reply) = requests.try_recv() {
                        let result = processing
                            .lock()
                            .map_err(|_| anyhow::anyhow!("VST3 rack lock poisoned"))
                            .and_then(|mut rack| rack.flush_parameters());
                        let _ = reply.send(result);
                    }
                    if frames.len() > frames.capacity() - BLOCK {
                        thread::sleep(Duration::from_micros(250));
                        continue;
                    }
                    let Ok(mut rack) = processing.lock() else {
                        report("VST3 rack lock poisoned".into());
                        break;
                    };
                    if overflow.swap(false, Ordering::Relaxed) {
                        while events.pop().is_some() {}
                        if let Err(error) = rack.event(AudioEvent::Panic) {
                            report(error.to_string());
                        }
                    }
                    for _ in 0..4096 {
                        let Some(event) = events.pop() else {
                            break;
                        };
                        if let Err(error) = rack.event(event) {
                            report(error.to_string());
                        }
                    }
                    let bpm = f64::from_bits(position.bpm.load(Ordering::Relaxed));
                    let quarter_notes =
                        f64::from_bits(position.quarter_notes.load(Ordering::Relaxed));
                    if let Err(error) =
                        rack.transport(bpm, position.playing.load(Ordering::Relaxed), quarter_notes)
                    {
                        report(error.to_string());
                    }
                    for error in rack.render(&mut block) {
                        report(error);
                    }
                    drop(rack);
                    for frame in block {
                        let _ = frames.push(frame);
                    }
                }
            })?;
        Ok(Self(Arc::new(Control {
            rack,
            thread: thread::current().id(),
            worker: Mutex::new(Some(worker)),
            stop,
            output,
            transport,
            flush,
        })))
    }

    fn control_rack(&self) -> Result<MutexGuard<'_, Rack>> {
        ensure!(
            thread::current().id() == self.0.thread,
            "VST3 control calls must stay on the UI thread"
        );
        self.0
            .rack
            .lock()
            .map_err(|_| anyhow::anyhow!("VST3 rack lock poisoned"))
    }
    pub fn set_transport(&self, bpm: f64, playing: bool, quarter_notes: f64) {
        self.0.transport.bpm.store(bpm.to_bits(), Ordering::Relaxed);
        self.0
            .transport
            .quarter_notes
            .store(quarter_notes.to_bits(), Ordering::Relaxed);
        self.0.transport.playing.store(playing, Ordering::Relaxed);
    }
    pub fn next(&self) -> [f32; 2] {
        self.0.output.pop().unwrap_or([0.0; 2])
    }
    pub fn configure(&self, tracks: &[Track]) -> Result<Vec<String>> {
        Ok(self.control_rack()?.configure(tracks))
    }
    pub fn parameters(&self, track: usize) -> Result<Vec<ParameterView>> {
        self.control_rack()?.parameters(track)
    }
    pub fn snapshot(&self) -> Result<Vec<(usize, Vec<u8>)>> {
        ensure!(
            thread::current().id() == self.0.thread,
            "VST3 snapshots must stay on the TUI thread"
        );
        let (reply, result) = bounded(1);
        self.0.flush.send_timeout(reply, Duration::from_secs(2))?;
        result.recv_timeout(Duration::from_secs(5))??;
        self.control_rack()?.snapshot()
    }
    pub fn service_control(&self) -> Result<Vec<String>> {
        Ok(self.control_rack()?.service_control())
    }
    pub fn shutdown(&self) {
        self.0.stop_worker();
        if let Ok(mut rack) = self.control_rack() {
            rack.clear();
        }
    }
}
