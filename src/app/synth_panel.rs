use super::{App, InstrumentPicker, SynthPage};
use crate::vst::{ParameterView, SynthKind};

impl App {
    pub(super) fn toggle_synth_panel(&mut self) {
        if self
            .instrument_picker
            .as_ref()
            .is_some_and(|p| matches!(p.page, SynthPage::Parameters(_)))
        {
            self.instrument_picker = None;
            return;
        }
        let parameters = self.synth_parameters();
        self.instrument_picker = Some(InstrumentPicker {
            page: SynthPage::Parameters(parameters),
            selected: 0,
            offset: 0,
            page_size: 8,
        });
    }

    pub(super) fn set_synth_parameter(&mut self, parameter: &ParameterView, value: f64) {
        let value = value.clamp(0.0, 1.0);
        if !value.is_finite() || (value - parameter.value).abs() < f64::EPSILON {
            return;
        }
        self.checkpoint();
        let track = &mut self.project.tracks[self.track];
        track.synth.parameters.insert(parameter.id, value);
        if track.synth.kind == SynthKind::Basic && parameter.name == "Program" {
            track.program = (value * 127.0).round() as u8;
        }
        // Apply controls without rebuilding the sequencer or cutting held notes.
        self.configure_synths();
    }

    pub fn poll_synths(&mut self) {
        if let Some(vst) = &self.audio.vst {
            match vst.service_control() {
                Ok(errors) => {
                    for error in errors {
                        self.status = error;
                    }
                }
                Err(error) => self.status = format!("VST3 control: {error:#}"),
            }
        }
    }
}
