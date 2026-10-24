use crate::model::Project;

pub enum Command {
    Reconnect,
    Project(Project),
    Play(bool),
    Panic,
    Input(Option<String>),
    Live(Vec<u8>),
    Selected(usize),
    Record(bool),
    Thru(bool),
    Shutdown,
}

pub enum Event {
    Message(String),
    Recorded {
        key: u8,
        velocity: u8,
        tick: u32,
        track: usize,
    },
}
