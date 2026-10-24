use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};

pub fn list_devices() -> Result<()> {
    let host = cpal::default_host();
    println!("Audio outputs:");
    for (i, d) in host.output_devices()?.enumerate() {
        println!("  {i}: {}", d.name()?);
    }
    println!("Audio inputs (enable with --input N):");
    for (i, d) in host.input_devices()?.enumerate() {
        println!("  {i}: {}", d.name()?);
    }
    Ok(())
}
