mod app;
mod audio;
mod engine;
mod model;
mod persistence;
mod runtime;
mod ui;

fn main() -> anyhow::Result<()> {
    runtime::run()
}
