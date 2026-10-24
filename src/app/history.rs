use super::App;
use crate::engine::Command;

impl App {
    pub fn checkpoint(&mut self) {
        self.undo.push(self.project.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    pub(super) fn undo(&mut self, redo: bool) {
        self.finish_recording();
        let state = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(project) = state {
            if redo {
                self.undo.push(self.project.clone());
            } else {
                self.redo.push(self.project.clone());
            }
            self.project = project;
            self.track = self.track.min(self.project.tracks.len() - 1);
            self.selected.clear();
            self.dirty = true;
            self.engine.send(Command::Selected(self.track));
            self.sync();
        }
    }
}
