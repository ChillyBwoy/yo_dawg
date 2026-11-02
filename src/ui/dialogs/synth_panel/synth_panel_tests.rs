use crate::{
    app::{App, InstrumentPicker, SynthPage},
    audio::{Audio, Options},
    engine::Engine,
    model::Project,
    vst::{ParameterView, SynthKind},
};
use ratatui::{Terminal, backend::TestBackend};

fn app(count: usize) -> App {
    let (errors, _) = crossbeam_channel::bounded(1);
    let audio = Audio::open(
        &Options {
            disabled: true,
            ..Default::default()
        },
        errors,
    )
    .unwrap();
    let mut project = Project::default();
    project.tracks[0].synth.kind = SynthKind::Tb303;
    let engine = Engine::start(project.clone(), audio.sink.clone());
    let mut app = App::new(
        project,
        engine,
        audio.sink,
        "unused.json".into(),
        audio.instruments,
    );
    let names = [
        "Square wave",
        "Cutoff",
        "Resonance",
        "Env Mod",
        "Decay",
        "Accent",
        "Tuning",
        "Legato slide",
    ];
    app.instrument_picker = Some(InstrumentPicker {
        selected: 0,
        offset: 0,
        page_size: 8,
        page: SynthPage::Parameters(
            (0..count)
                .map(|index| ParameterView {
                    id: index as u32,
                    name: if index < 8 {
                        names[index].into()
                    } else {
                        format!("Parameter {index}")
                    },
                    value: 0.5,
                    default: 0.25,
                    steps: if index == 0 { 1 } else { 0 },
                    display: "350 Hz".into(),
                })
                .collect(),
        ),
    });
    app
}

#[test]
fn acid_panel_renders_parameter_cards_and_sliders_in_the_terminal() {
    let mut app = app(8);
    let mut terminal = Terminal::new(TestBackend::new(120, 35)).unwrap();
    terminal.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    for label in [
        "TB-303",
        "Synth controls",
        "Cutoff",
        "Resonance",
        "Decay",
        "Accent",
        "SQUARE",
        "350 Hz",
    ] {
        assert!(text.contains(label), "missing {label}");
    }
    assert!(!text.contains("plugin window"));
}

#[test]
fn panel_handles_tiny_terminals_empty_parameters_and_long_parameter_lists() {
    for (width, height) in [(1, 1), (30, 12), (70, 20), (120, 35)] {
        for count in [0usize, 8, 128] {
            let mut app = app(count);
            app.instrument_picker.as_mut().unwrap().selected = count.saturating_sub(1);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| super::draw(f, &mut app)).unwrap();
            if count == 128 && width >= 70 {
                let text = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|c| c.symbol())
                    .collect::<String>();
                assert!(text.contains("Parameter 127"));
            }
        }
    }
}
