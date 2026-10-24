use crate::model::Project;
use anyhow::{Context, Result};
use std::{io::Write, path::Path};

pub fn load(path: &Path) -> Result<Project> {
    let project: Project = serde_json::from_slice(&std::fs::read(path).context("Read project")?)?;
    project.validate()?;
    Ok(project)
}

pub fn save(project: &Project, path: &Path) -> Result<()> {
    project.validate()?;
    // A sibling temporary file keeps rename atomic on the same filesystem.
    let tmp = path.with_extension("json.tmp");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .context("Create temporary project (remove stale .json.tmp if necessary)")?;
    let result = (|| -> Result<()> {
        file.write_all(&serde_json::to_vec_pretty(project)?)?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}
