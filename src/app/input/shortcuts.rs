use crate::app::{Action, App};
use crossterm::event::KeyCode as K;

impl App {
    pub(super) fn handle_control_key(&mut self, key: K) {
        match key {
            K::Char('s') => self.prompt(
                "Save project",
                self.path.display().to_string(),
                Action::Save,
            ),
            K::Char('o') => {
                if self.dirty {
                    self.status = "Save current changes before loading another project".into();
                } else {
                    self.prompt(
                        "Load project",
                        self.path.display().to_string(),
                        Action::Load,
                    );
                }
            }
            K::Char('e') => self.prompt("Export MIDI", "song.mid".into(), Action::Export),
            K::Char('i') => {
                if self.dirty {
                    self.status = "Save current changes before importing MIDI".into();
                } else {
                    self.prompt("Import MIDI", "song.mid".into(), Action::Import);
                }
            }
            K::Char('a') => {
                self.selected = (0..self.project.tracks[self.track].notes.len()).collect()
            }
            K::Char('c') => self.copy(),
            K::Char('x') => {
                self.copy();
                self.delete();
            }
            K::Char('v') => self.paste(),
            K::Char('z') => self.undo(false),
            K::Char('y') => self.undo(true),
            K::Char('q') => self.request_quit(),
            K::Left => self.transform(-self.grid.cast_signed(), 0, true, 0),
            K::Right => self.transform(self.grid as i32, 0, true, 0),
            K::Up => self.transform(0, 0, false, 5),
            K::Down => self.transform(0, 0, false, -5),
            _ => {}
        }
    }
}
