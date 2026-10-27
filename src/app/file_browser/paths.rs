use super::{BrowserFocus, FileAction, FileBrowser, Outcome};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

impl FileAction {
    pub fn accepts(self, path: &Path) -> bool {
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        match self {
            Self::OpenProject | Self::SaveProject => extension == "json",
            Self::ImportMidi | Self::ExportMidi => matches!(extension.as_str(), "mid" | "midi"),
        }
    }

    pub(super) fn suggested_name(self, project: &Path) -> String {
        let name = project.file_name().unwrap_or_default().to_string_lossy();
        if matches!(self, Self::OpenProject | Self::SaveProject) {
            return name.into_owned();
        }
        let stem = name
            .strip_suffix(".yodawg.json")
            .or_else(|| name.strip_suffix(".json"))
            .unwrap_or(&name);
        format!("{stem}.mid")
    }

    fn add_extension(self, path: PathBuf) -> PathBuf {
        if path.extension().is_none() {
            let mut name = path.into_os_string();
            name.push(if self == Self::SaveProject {
                ".yodawg.json"
            } else {
                ".mid"
            });
            PathBuf::from(name)
        } else if self == Self::SaveProject && path.extension().is_some_and(|e| e == "yodawg") {
            let mut name = path.into_os_string();
            name.push(".json");
            PathBuf::from(name)
        } else {
            path
        }
    }
}

impl FileBrowser {
    pub(super) fn submit(&mut self, path: PathBuf) -> Result<Outcome> {
        if path.is_dir() {
            self.explorer.set_cwd(path)?;
            self.focus = BrowserFocus::List;
            self.input = if self.action.is_save() {
                self.default_name.clone()
            } else {
                String::new()
            };
            self.selected_text = true;
            return Ok(Outcome::Continue);
        }
        let path = if self.action.is_save() {
            self.action.add_extension(path)
        } else {
            path
        };
        ensure!(
            self.action.accepts(&path),
            "Expected {}",
            self.action.filter_label()
        );
        if self.action.is_save() {
            ensure!(path.file_name().is_some(), "Enter a file name");
            ensure!(
                path.parent().is_some_and(Path::is_dir),
                "Destination directory does not exist"
            );
            if path.exists() {
                self.overwrite = Some(path);
                return Ok(Outcome::Continue);
            }
        } else {
            ensure!(path.is_file(), "File does not exist: {}", path.display());
        }
        Ok(Outcome::Selected(path))
    }

    pub(super) fn typed_path(&self) -> Result<PathBuf> {
        ensure!(!self.input.is_empty(), "Enter a file name or path");
        let path = if self.input == "~" || self.input.starts_with("~/") {
            let home =
                std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
            PathBuf::from(home).join(self.input.strip_prefix("~/").unwrap_or(""))
        } else {
            PathBuf::from(&self.input)
        };
        Ok(if path.is_absolute() {
            path
        } else {
            self.explorer.cwd().join(path)
        })
    }
}
