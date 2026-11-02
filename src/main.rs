mod app;
mod audio;
mod engine;
mod model;
mod persistence;
mod runtime;
mod synths;
mod ui;
mod vst;

fn main() -> anyhow::Result<()> {
    runtime::run()
}
