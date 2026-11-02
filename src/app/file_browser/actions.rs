use super::super::App;
use crate::{
    engine::Command,
    persistence::{midi, project},
};
use anyhow::{Result, ensure};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAction {
    OpenProject,
    SaveProject,
    ImportMidi,
    ExportMidi,
}

impl FileAction {
    pub fn is_save(self) -> bool {
        matches!(self, Self::SaveProject | Self::ExportMidi)
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::OpenProject => "Open project",
            Self::SaveProject => "Save project",
            Self::ImportMidi => "Import MIDI",
            Self::ExportMidi => "Export MIDI",
        }
    }
    pub fn filter_label(self) -> &'static str {
        match self {
            Self::OpenProject | Self::SaveProject => "*.yodawg.json / *.json",
            Self::ImportMidi | Self::ExportMidi => "*.mid / *.midi",
        }
    }
}

impl App {
    pub(super) fn apply_file_action(&mut self, action: FileAction, path: &Path) -> Result<()> {
        match action {
            FileAction::SaveProject => {
                self.poll_synths();
                self.finish_recording();
                if let Some(vst) = &self.audio.vst {
                    for (track, state) in vst.snapshot()? {
                        let parameters = vst.parameters(track)?;
                        if let Some(saved_track) = self.project.tracks.get_mut(track) {
                            if saved_track.synth.kind == crate::vst::SynthKind::Basic {
                                if let Some(program) =
                                    parameters.iter().find(|p| p.name == "Program")
                                {
                                    saved_track.program = (program.value * 127.0).round() as u8;
                                }
                            }
                            saved_track.synth.state = state;
                            saved_track.synth.parameters =
                                parameters.into_iter().map(|p| (p.id, p.value)).collect();
                        }
                    }
                }
                project::save(&self.project, path)?;
                self.path = path.to_owned();
                self.dirty = false;
                self.status = format!("Project saved: {}", path.display());
            }
            FileAction::OpenProject | FileAction::ImportMidi => {
                // Recheck: live recording may have changed the project while browsing.
                ensure!(
                    !self.dirty,
                    "Save current changes before opening or importing"
                );
                let p = if action == FileAction::OpenProject {
                    project::load(path)?
                } else {
                    midi::import(path)?
                };
                self.finish_recording();
                self.engine.send(Command::Play(false));
                self.checkpoint();
                self.project = p;
                self.select_track(0);
                self.cursor = self.project.loop_start;
                self.sync();
                if action == FileAction::OpenProject {
                    self.path = path.to_owned();
                    self.dirty = false;
                }
                self.status = format!("{}: {}", action.title(), path.display());
            }
            FileAction::ExportMidi => {
                midi::export(&self.project, path)?;
                self.status = format!("MIDI exported: {}", path.display());
            }
        }
        Ok(())
    }
}
