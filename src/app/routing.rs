use super::App;
use crate::engine::{self, Command};

impl App {
    pub fn refresh_ports(&mut self) {
        match engine::ports() {
            Ok((inputs, outputs)) => {
                self.input_ports = inputs;
                self.output_ports = outputs;
                self.status = "MIDI ports refreshed".into();
            }
            Err(e) => self.status = format!("MIDI: {e}"),
        }
        self.engine.send(Command::Input(self.input.clone()));
        self.sync();
        self.engine.send(Command::Reconnect);
    }

    pub(super) fn cycle_input(&mut self) {
        self.finish_recording();
        let next = self
            .input
            .as_ref()
            .and_then(|n| self.input_ports.iter().position(|p| p == n))
            .map_or(0, |i| i + 1);
        self.input = self.input_ports.get(next).cloned();
        self.engine.send(Command::Input(self.input.clone()));
    }

    pub(super) fn cycle_output(&mut self) {
        self.checkpoint();
        let t = &mut self.project.tracks[self.track];
        let next = t
            .midi_output
            .as_ref()
            .and_then(|n| self.output_ports.iter().position(|p| p == n))
            .map_or(0, |i| i + 1);
        t.midi_output = self.output_ports.get(next).cloned();
        self.sync();
    }
}
