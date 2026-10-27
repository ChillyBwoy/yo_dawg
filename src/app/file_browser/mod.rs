//! File operation orchestration; the explorer owns navigation, not project I/O.

mod actions;
#[cfg(test)]
mod file_browser_tests;
mod navigation;
mod paths;

use super::App;
pub use actions::FileAction;
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui_explorer::{FileExplorer, FileExplorerBuilder};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BrowserFocus {
    List,
    Path,
}

pub struct FileBrowser {
    pub explorer: FileExplorer,
    pub action: FileAction,
    pub focus: BrowserFocus,
    pub input: String,
    pub selected_text: bool,
    pub error: Option<String>,
    pub overwrite: Option<PathBuf>,
    default_name: String,
}

enum Outcome {
    Continue,
    Cancel,
    Selected(PathBuf),
}

impl FileBrowser {
    pub fn new(action: FileAction, directory: &Path, suggested: &Path) -> Result<Self> {
        let mut explorer = FileExplorerBuilder::default()
            .working_dir(directory)
            .filter_map(move |file| {
                if file.is_dir || (file.path.is_file() && action.accepts(&file.path)) {
                    Some(file)
                } else {
                    None
                }
            })
            .build()?;
        let existing = explorer
            .files()
            .iter()
            .position(|f| !f.is_dir && f.path == suggested);
        if let Some(index) = existing {
            explorer.set_selected_idx(index);
        }
        let default_name = suggested
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let input = if action.is_save() || existing.is_some() {
            default_name.clone()
        } else {
            String::new()
        };
        Ok(Self {
            explorer,
            action,
            focus: if action.is_save() {
                BrowserFocus::Path
            } else {
                BrowserFocus::List
            },
            input,
            selected_text: true,
            error: None,
            overwrite: None,
            default_name,
        })
    }
}

impl App {
    pub(super) fn open_file_browser(&mut self, action: FileAction) {
        if !action.is_save() && self.dirty {
            self.status = "Save current changes before opening a project or importing MIDI".into();
            return;
        }
        let open = || -> Result<FileBrowser> {
            let cwd = std::env::current_dir()?;
            let project_path = if self.path.is_absolute() {
                self.path.clone()
            } else {
                cwd.join(&self.path)
            };
            let directory = self
                .last_file_directory
                .as_deref()
                .filter(|p| p.is_dir())
                .or_else(|| project_path.parent().filter(|p| p.is_dir()))
                .unwrap_or(&cwd);
            let filename = action.suggested_name(&project_path);
            FileBrowser::new(action, directory, &directory.join(filename))
        };
        match open() {
            Ok(browser) => self.file_browser = Some(browser),
            Err(error) => self.status = format!("File browser: {error:#}"),
        }
    }

    pub(super) fn handle_file_browser_key(&mut self, event: KeyEvent) {
        let Some(mut browser) = self.file_browser.take() else {
            return;
        };
        let outcome = match browser.handle_key(event) {
            Ok(outcome) => outcome,
            Err(error) => {
                browser.error = Some(format!("{error:#}"));
                Outcome::Continue
            }
        };
        self.last_file_directory = Some(browser.explorer.cwd().clone());
        match outcome {
            Outcome::Cancel => {}
            Outcome::Continue => self.file_browser = Some(browser),
            Outcome::Selected(path) => {
                if let Err(error) = self.apply_file_action(browser.action, &path) {
                    browser.error = Some(format!("{error:#}"));
                    self.file_browser = Some(browser);
                } else {
                    self.last_file_directory = path.parent().map(Path::to_path_buf);
                }
            }
        }
    }
}
