use super::*;

#[test]
fn builtin_instruments_only_fill_the_unique_slots() {
    let instruments = Instruments::default();
    for track in [0, 9] {
        assert_eq!(instruments.label(0, track, Route::Internal), "1_Soft_Sine");
        assert_eq!(instruments.label(3, track, Route::Internal), "4_Pure_Sine");
        assert_eq!(instruments.label(11, track, Route::Internal), "12_Bass");
        for program in 12..128 {
            assert_eq!(instruments.label(program, track, Route::Internal), "--");
        }
    }
}

#[test]
fn duplicate_names_leave_empty_slots_without_renumbering() {
    let mut names = std::array::from_fn(|_| String::new());
    names[0] = "Piano".into();
    names[2] = "Piano".into();
    names[7] = "Bass".into();
    let names = unique_names(names);
    assert_eq!(names[0], "Piano");
    assert!(names[2].is_empty());
    assert_eq!(names[7], "Bass");
}

#[test]
fn empty_internal_slots_do_not_hide_external_programs() {
    let instruments = Instruments::default();
    assert_eq!(
        instruments.label(12, 0, Route::External),
        "13_External_Program"
    );
    assert_eq!(instruments.label(12, 0, Route::Both), "13_External_Program");
}
