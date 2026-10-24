use super::Note;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Route {
    Internal,
    External,
    Both,
}

impl Route {
    pub fn next(self) -> Self {
        match self {
            Self::Internal => Self::External,
            Self::External => Self::Both,
            Self::Both => Self::Internal,
        }
    }
    pub fn internal(self) -> bool {
        self != Self::External
    }
    pub fn external(self) -> bool {
        self != Self::Internal
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub channel: u8,
    pub program: u8,
    pub volume: u8,
    pub mute: bool,
    pub solo: bool,
    pub route: Route,
    #[serde(default)]
    pub midi_output: Option<String>,
    pub notes: Vec<Note>,
}

impl Track {
    pub fn new(index: usize) -> Self {
        Self {
            name: format!("Track {}", index + 1),
            channel: index as u8 % 16,
            program: 0,
            volume: 100,
            mute: false,
            solo: false,
            route: Route::Internal,
            midi_output: None,
            notes: Vec::new(),
        }
    }
}
