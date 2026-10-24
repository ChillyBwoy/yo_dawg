use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub start: u32,
    pub duration: u32,
    pub key: u8,
    pub velocity: u8,
}
