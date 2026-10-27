use crate::{app::ViewMode, audio};
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

pub(super) struct Options {
    pub audio: audio::Options,
    pub path: PathBuf,
    pub load: bool,
    pub demo: bool,
    pub devices: bool,
    pub view: ViewMode,
}

pub(super) fn parse() -> Result<Option<Options>> {
    let mut options = Options {
        audio: audio::Options::default(),
        path: "song.yodawg.json".into(),
        load: false,
        demo: false,
        devices: false,
        view: ViewMode::PianoRoll,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-audio" => options.audio.disabled = true,
            "--demo" => options.demo = true,
            "--devices" => options.devices = true,
            "--view" => {
                options.view = match args
                    .next()
                    .context("--view requires piano-roll or tracker")?
                    .as_str()
                {
                    "piano-roll" => ViewMode::PianoRoll,
                    "tracker" => ViewMode::Tracker,
                    value => bail!("Unknown view: {value}; expected piano-roll or tracker"),
                }
            }
            "--input" => {
                options.audio.input =
                    Some(args.next().context("--input requires an index")?.parse()?)
            }
            "--output" => {
                options.audio.output =
                    Some(args.next().context("--output requires an index")?.parse()?)
            }
            "--soundfont" => {
                options.audio.soundfont =
                    Some(args.next().context("--soundfont requires a path")?.into())
            }
            "--project" => {
                options.path = args.next().context("--project requires a path")?.into();
                options.load = true;
            }
            "--help" | "-h" => {
                println!(
                    "yo_dawg — TUI MIDI sequencer for macOS / Linux\n\ncargo run -- [--demo] [--project FILE] [--devices] [--no-audio]\n             [--soundfont FILE.sf2] [--input N] [--output N]\n             [--view piano-roll|tracker]\n\nSpace: play; Enter: draw; F4: switch view; ?: shortcuts; F2: MIDI routing.\nMIDI USB/BLE ports must be connected in the OS.\nAudio input monitoring is opt-in (--input N). Start at low volume."
                );
                return Ok(None);
            }
            _ => bail!("Unknown argument: {arg}; use --help"),
        }
    }
    Ok(Some(options))
}
