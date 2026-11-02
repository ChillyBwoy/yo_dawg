use super::{
    SynthConfig,
    rack::{BLOCK, Rack},
};
use crate::{audio::AudioEvent, model::Track};

fn chunk(id: &[u8; 4], mut data: Vec<u8>) -> Vec<u8> {
    let mut result = id.to_vec();
    result.extend_from_slice(&(data.len() as u32).to_le_bytes());
    if data.len() % 2 != 0 {
        data.push(0);
    }
    result.extend(data);
    result
}

fn list(id: &[u8; 4], chunks: Vec<Vec<u8>>) -> Vec<u8> {
    let mut data = id.to_vec();
    data.extend(chunks.into_iter().flatten());
    chunk(b"LIST", data)
}

fn named(name: &str, words: &[u16], dwords: &[u32]) -> Vec<u8> {
    let mut data = vec![0; 20];
    data[..name.len()].copy_from_slice(name.as_bytes());
    for value in words {
        data.extend(value.to_le_bytes());
    }
    for value in dwords {
        data.extend(value.to_le_bytes());
    }
    data
}

/// A generated, royalty-free single-cycle sine bank, including terminal records.
fn sine_bank() -> Vec<u8> {
    let samples: Vec<u8> = (0..558)
        .flat_map(|i| {
            let value = if i < 512 {
                ((i as f32 * std::f32::consts::TAU / 100.0).sin() * 16_000.0) as i16
            } else {
                0
            };
            value.to_le_bytes()
        })
        .collect();
    let mut presets = named("Test Sine", &[0, 0, 0], &[0, 0, 0]);
    presets.extend(named("EOP", &[0, 0, 1], &[0, 0, 0]));
    let mut instruments = named("Sine", &[0], &[]);
    instruments.extend(named("EOI", &[1], &[]));
    let mut header = named("Sine", &[], &[0, 512, 8, 508, 44_100]);
    header.extend([69, 0, 0, 0, 1, 0]);
    let mut terminal = named("EOS", &[], &[512, 512, 512, 512, 44_100]);
    terminal.extend([0, 0, 0, 0, 1, 0]);
    header.extend(terminal);
    let mut riff = b"sfbk".to_vec();
    riff.extend(list(
        b"INFO",
        vec![
            chunk(b"ifil", vec![2, 0, 1, 0]),
            chunk(b"INAM", b"Test\0\0".to_vec()),
        ],
    ));
    riff.extend(list(b"sdta", vec![chunk(b"smpl", samples)]));
    riff.extend(list(
        b"pdta",
        vec![
            chunk(b"phdr", presets),
            chunk(b"pbag", vec![0, 0, 0, 0, 1, 0, 0, 0]),
            chunk(b"pmod", vec![0; 10]),
            chunk(b"pgen", vec![41, 0, 0, 0, 0, 0, 0, 0]),
            chunk(b"inst", instruments),
            chunk(b"ibag", vec![0, 0, 0, 0, 2, 0, 0, 0]),
            chunk(b"imod", vec![0; 10]),
            chunk(b"igen", vec![54, 0, 1, 0, 53, 0, 0, 0, 0, 0, 0, 0]),
            chunk(b"shdr", header),
        ],
    ));
    chunk(b"RIFF", riff)
}

#[test]
#[ignore = "requires the Basic VST3 bundle"]
fn soundfont_plays_and_restores_inside_basic_vst3() {
    let _plugins = super::test_lock();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sine.sf2");
    std::fs::write(&path, sine_bank()).unwrap();
    rustysynth::SoundFont::new(&mut std::fs::File::open(&path).unwrap()).unwrap();
    let mut track = Track::new(0);
    track.synth = SynthConfig {
        soundfont: Some(path),
        ..Default::default()
    };
    let mut rack = Rack::new(48_000.0).unwrap();
    assert!(rack.configure(&[track.clone()]).is_empty());
    rack.event(AudioEvent::On(0, 60, 100)).unwrap();
    let mut block = [[0.0; 2]; BLOCK];
    let mut peak = 0.0f32;
    for _ in 0..100 {
        assert!(rack.render(&mut block).is_empty());
        for value in block.iter().flatten() {
            assert!(value.is_finite());
            peak = peak.max(value.abs());
        }
    }
    assert!(peak > 0.001);
    track.synth.state = rack.snapshot().unwrap().remove(0).1;
    let mut restored = Rack::new(48_000.0).unwrap();
    assert!(restored.configure(&[track]).is_empty());
    restored.event(AudioEvent::On(0, 60, 100)).unwrap();
    for _ in 0..10 {
        assert!(restored.render(&mut block).is_empty());
    }
    assert!(block.iter().flatten().any(|v| v.abs() > 0.001));
}
