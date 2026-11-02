use crate::model::Route;
use rustysynth::SoundFont;
use std::collections::HashSet;

#[cfg(test)]
#[path = "instruments_tests.rs"]
mod instruments_tests;

#[derive(Clone)]
pub struct Instruments {
    melodic: [String; 128],
    drums: [String; 128],
}

impl Default for Instruments {
    fn default() -> Self {
        let names = super::builtin::NAMES;
        let melodic = std::array::from_fn(|i| names.get(i).copied().unwrap_or("").to_owned());
        Self {
            drums: melodic.clone(),
            melodic,
        }
    }
}

impl Instruments {
    pub fn from_soundfont(sf: &SoundFont) -> Self {
        let presets = sf.get_presets();
        let names = |bank: i32| {
            unique_names(std::array::from_fn(|program| {
                let exact = presets.iter().find(|p| {
                    p.get_bank_number() == bank && p.get_patch_number() == program as i32
                });
                let Some(preset) = exact else {
                    return String::new();
                };
                let name = preset
                    .get_name()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join("_");
                let name = if name.is_empty() {
                    "Unnamed".into()
                } else {
                    name
                };
                name
            }))
        };
        Self {
            melodic: names(0),
            drums: names(128),
        }
    }

    pub fn label(&self, program: u8, track: usize, route: Route) -> String {
        let internal = if track == 9 {
            &self.drums[program as usize]
        } else {
            &self.melodic[program as usize]
        };
        let name = match route {
            Route::Internal if internal.is_empty() => return "--".into(),
            Route::Internal => internal.as_str(),
            Route::External => "External_Program",
            Route::Both if internal.is_empty() => "External_Program",
            Route::Both => return format!("{}_{}+External_Program", program as u16 + 1, internal),
        };
        format!("{}_{}", program as u16 + 1, name)
    }
}

fn unique_names(mut names: [String; 128]) -> [String; 128] {
    let mut seen = HashSet::new();
    for name in &mut names {
        if !seen.insert(name.clone()) {
            name.clear();
        }
    }
    names
}
