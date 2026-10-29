use super::*;

#[test]
fn legacy_projects_inherit_the_project_loop() {
    let mut p = Project::demo();
    p.loop_start = BAR;
    p.loop_end = BAR * 3;
    let json = serde_json::to_value(&p).unwrap();
    assert!(json["tracks"][0].get("length").is_none());
    let q: Project = serde_json::from_value(json).unwrap();
    q.validate().unwrap();
    assert_eq!(q.tracks[0].length, None);
    assert_eq!(q.track_range(0), (BAR, BAR * 3));
    assert_eq!(q.track_length(0), BAR * 2);
}

#[test]
fn explicit_lengths_roundtrip_and_do_not_delete_outside_notes() {
    let mut p = Project::demo();
    p.tracks[0].length = Some(BAR);
    p.tracks.push(Track::new(1));
    p.tracks[1].length = Some(BAR * 5);
    p.validate().unwrap();
    let q: Project = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
    assert_eq!(q.track_range(0), (0, BAR));
    assert_eq!(q.track_range(1), (0, BAR * 5));
    assert_eq!(q.tracks[0].notes, p.tracks[0].notes);
    assert!(q.tracks[0].notes.iter().any(|n| n.start >= BAR));
    assert_eq!(q.track_length_label(0), "1b");
}

#[test]
fn invalid_lengths_are_rejected_and_sub_bar_lengths_are_supported() {
    let mut p = Project::default();
    for length in [0, BAR * 4096 + 1, u32::MAX] {
        p.tracks[0].length = Some(length);
        assert!(p.validate().is_err());
    }
    p.tracks[0].length = Some(PPQ);
    p.validate().unwrap();
    assert_eq!(p.track_length_label(0), "480t");
}
