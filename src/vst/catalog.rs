use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynthKind {
    #[default]
    Basic,
    Tb303,
    Vst3 {
        path: PathBuf,
    },
}

impl SynthKind {
    pub fn name(&self) -> String {
        match self {
            Self::Basic => crate::synths::BASIC.into(),
            Self::Tb303 => crate::synths::TB303.into(),
            Self::Vst3 { path } => path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
        }
    }
    pub fn path(&self) -> PathBuf {
        match self {
            Self::Vst3 { path } => path.clone(),
            _ => bundled_path(&self.name()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SynthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soundfont: Option<PathBuf>,
    #[serde(default)]
    pub kind: SynthKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<u8>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<u32, f64>,
}

fn bundled_path(name: &str) -> PathBuf {
    let candidates = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("plugins"))),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/bundled")),
    ];
    for root in candidates.into_iter().flatten() {
        let path = root.join(format!("{name}.vst3"));
        if path.exists() {
            return path;
        }
    }
    PathBuf::from("plugins").join(format!("{name}.vst3"))
}

/// Filesystem-only discovery: do not execute unknown plugins while opening P.
pub fn available() -> Vec<SynthKind> {
    let mut result = vec![SynthKind::Basic, SynthKind::Tb303];
    if let Ok(mut host) = vst3_host::Vst3Host::new() {
        if let Some(paths) = std::env::var_os("YO_DAWG_VST3_PATH") {
            for path in std::env::split_paths(&paths) {
                let _ = host.add_scan_path(path);
            }
        }
        let mut paths = host.scan_plugin_paths();
        paths.sort();
        paths.dedup();
        for path in paths {
            if result.iter().any(|kind| kind.path() == path) {
                continue;
            }
            result.push(SynthKind::Vst3 { path });
        }
    }
    result
}
