//! Startup orchestration, command-line options, and terminal lifecycle.

mod cli;
mod terminal;

use crate::{app::App, audio, engine, model::Project, persistence};
use anyhow::{Context, Result};

pub fn run() -> Result<()> {
    let Some(options) = cli::parse()? else {
        return Ok(());
    };
    if options.devices {
        return list_devices();
    }
    let project = if options.load {
        persistence::project::load(&options.path)?
    } else if options.demo {
        Project::demo()
    } else {
        Project::default()
    };
    let (errors_tx, errors) = crossbeam_channel::bounded(32);
    let audio = audio::Audio::open(&options.audio, errors_tx)
        .context("Initialize audio (use --devices or --no-audio)")?;
    let engine = engine::Engine::start(project.clone(), audio.sink.clone());
    let mut app = App::new(
        project,
        engine,
        audio.sink.clone(),
        options.path,
        audio.instruments.clone(),
    );
    if options.demo {
        app.dirty = true;
    }
    terminal::run(&mut app, errors)?;
    drop(app);
    drop(audio);
    Ok(())
}

fn list_devices() -> Result<()> {
    audio::list_devices()?;
    match engine::ports() {
        Ok((inputs, outputs)) => {
            println!("MIDI inputs:");
            for p in inputs {
                println!("  {p}");
            }
            println!("MIDI outputs:");
            for p in outputs {
                println!("  {p}");
            }
        }
        Err(e) => eprintln!("MIDI unavailable: {e}"),
    }
    Ok(())
}
