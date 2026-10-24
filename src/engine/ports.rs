use anyhow::Result;
use midir::{MidiInput, MidiOutput};

pub fn ports() -> Result<(Vec<String>, Vec<String>)> {
    let input = MidiInput::new("yo_dawg list")?;
    let output = MidiOutput::new("yo_dawg list")?;
    let ins = input
        .ports()
        .iter()
        .map(|p| input.port_name(p).unwrap_or_else(|_| "Unknown".into()))
        .collect();
    let outs = output
        .ports()
        .iter()
        .map(|p| output.port_name(p).unwrap_or_else(|_| "Unknown".into()))
        .collect();
    Ok((ins, outs))
}
