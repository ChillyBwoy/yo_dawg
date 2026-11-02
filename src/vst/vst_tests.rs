use super::*;
use crate::{
    audio::AudioEvent,
    model::{Project, Track},
};
use rack::{BLOCK, Rack};

#[test]
fn old_projects_default_to_basic_and_plugin_settings_roundtrip() {
    let mut json = serde_json::to_value(Project::default()).unwrap();
    json["tracks"][0].as_object_mut().unwrap().remove("synth");
    let mut project: Project = serde_json::from_value(json).unwrap();
    assert_eq!(project.tracks[0].synth.kind, SynthKind::Basic);
    project.tracks[0].synth.kind = SynthKind::Tb303;
    project.tracks[0].synth.parameters.insert(12, 0.75);
    project.tracks[0].synth.state = vec![1, 2, 3];
    let loaded: Project = serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
    assert_eq!(loaded.tracks[0].synth, project.tracks[0].synth);
    project.tracks[0].synth.parameters.insert(12, f64::NAN);
    assert!(project.validate().is_err());
}

#[test]
fn missing_plugins_are_silent_and_do_not_replace_saved_settings() {
    let mut rack = Rack::new(48_000.0).unwrap();
    let mut track = Track::new(0);
    track.synth.kind = SynthKind::Vst3 {
        path: "missing-synth.vst3".into(),
    };
    track.synth.state = vec![1, 2, 3];
    let saved = track.synth.clone();
    assert_eq!(rack.configure(&[track.clone()]).len(), 1);
    let mut output = [[1.0; 2]; BLOCK];
    assert!(rack.render(&mut output).is_empty());
    assert!(output.iter().flatten().all(|value| *value == 0.0));
    assert_eq!(track.synth, saved);
    track.route = crate::model::Route::External;
    assert!(rack.configure(&[track]).is_empty());
}

#[test]
#[ignore = "requires bundled VST3 plugins"]
fn removing_a_parameter_override_restores_plugin_defaults() {
    let _plugins = super::test_lock();
    let mut rack = Rack::new(48_000.0).unwrap();
    let mut track = Track::new(0);
    track.synth.kind = SynthKind::Tb303;
    assert!(rack.configure(&[track.clone()]).is_empty());
    let cutoff = rack
        .parameters(0)
        .unwrap()
        .into_iter()
        .find(|p| p.name == "Cutoff")
        .unwrap();
    track.synth.parameters.insert(cutoff.id, 0.9);
    assert!(rack.configure(&[track.clone()]).is_empty());
    track.synth.parameters.clear();
    assert!(rack.configure(&[track]).is_empty());
    let restored = rack
        .parameters(0)
        .unwrap()
        .into_iter()
        .find(|p| p.id == cutoff.id)
        .unwrap();
    assert!((restored.value - cutoff.value).abs() < 0.001);
}

#[test]
#[ignore = "requires cargo xtask bundle -p yo_dawg_basic -p yo_dawg_tb303 --release"]
fn real_vst3_bundles_render_restore_and_isolate_track_instances() {
    let _plugins = super::test_lock();
    let mut rack = Rack::new(48_000.0).unwrap();
    let mut tracks = vec![Track::new(0), Track::new(1)];
    tracks[1].synth.kind = SynthKind::Tb303;
    assert!(rack.configure(&tracks).is_empty());
    for kind in [SynthKind::Basic, SynthKind::Tb303] {
        let mut host = vst3_host::Vst3Host::new().unwrap();
        let plugin = host.load_plugin(kind.path()).unwrap();
        assert!(
            !plugin.has_editor(),
            "Our synths must not export external-window editors"
        );
    }
    let mut block = [[0.0; 2]; BLOCK];
    assert!(rack.render(&mut block).is_empty());
    assert!(block.iter().flatten().all(|v| v.abs() < 0.0001));
    rack.event(AudioEvent::On(0, 48, 90)).unwrap();
    assert!(rack.render(&mut block).is_empty());
    assert!(block.iter().flatten().any(|v| v.abs() > 0.001));
    rack.event(AudioEvent::On(1, 36, 120)).unwrap();
    assert!(rack.render(&mut block).is_empty());
    let parameter = rack
        .parameters(1)
        .unwrap()
        .into_iter()
        .find(|p| p.name == "Cutoff")
        .unwrap();
    tracks[1].synth.parameters.insert(parameter.id, 0.8);
    assert!(rack.configure(&tracks).is_empty());
    assert!(rack.render(&mut block).is_empty());
    let state = rack.snapshot().unwrap();
    assert!(state.iter().all(|(_, bytes)| !bytes.is_empty()));
    let mut restored = Rack::new(48_000.0).unwrap();
    for (index, bytes) in state {
        tracks[index].synth.state = bytes;
    }
    assert!(restored.configure(&tracks).is_empty());
    assert!(restored.render(&mut block).is_empty());
    let restored_value = restored
        .parameters(1)
        .unwrap()
        .into_iter()
        .find(|p| p.id == parameter.id)
        .unwrap();
    assert!((restored_value.value - 0.8).abs() < 0.001);
    rack.event(AudioEvent::Panic).unwrap();
    for _ in 0..1000 {
        assert!(rack.render(&mut block).is_empty());
    }
    assert!(block.iter().flatten().all(|v| v.abs() < 0.0001));

    // Same plugin and same key on two tracks must remain independent.
    tracks[1].synth = SynthConfig::default();
    assert!(rack.configure(&tracks).is_empty());
    rack.event(AudioEvent::On(0, 60, 100)).unwrap();
    rack.event(AudioEvent::On(1, 60, 100)).unwrap();
    rack.event(AudioEvent::Off(0, 60)).unwrap();
    for _ in 0..1000 {
        assert!(rack.render(&mut block).is_empty());
    }
    assert!(block.iter().flatten().any(|v| v.abs() > 0.001));
}
