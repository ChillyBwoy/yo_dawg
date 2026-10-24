use crate::model::Route;
use rustysynth::SoundFont;

#[derive(Clone)]
pub struct Instruments {
    melodic: [String; 128],
    drums: [String; 128],
}

impl Default for Instruments {
    fn default() -> Self {
        let names = ["Soft_Sine", "Bright_Sine", "Rich_Sine", "Pure_Sine"];
        let melodic = std::array::from_fn(|i| names[i % 4].to_owned());
        Self {
            drums: melodic.clone(),
            melodic,
        }
    }
}

impl Instruments {
    pub fn from_soundfont(sf: &SoundFont) -> Self {
        let presets = sf.get_presets();
        let fallback = presets
            .iter()
            .min_by_key(|p| (p.get_bank_number(), p.get_patch_number()));
        let names = |bank: i32| {
            std::array::from_fn(|program| {
                let exact = presets.iter().find(|p| {
                    p.get_bank_number() == bank && p.get_patch_number() == program as i32
                });
                let preset = exact
                    .or_else(|| {
                        if bank == 128 {
                            presets
                                .iter()
                                .find(|p| p.get_bank_number() == 128 && p.get_patch_number() == 0)
                        } else {
                            None
                        }
                    })
                    .or(fallback);
                let Some(preset) = preset else {
                    return "Unavailable".into();
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
                if exact.is_none() {
                    format!("{name}_Fallback")
                } else {
                    name
                }
            })
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
            Route::Internal => internal.as_str(),
            Route::External => "External_Program",
            Route::Both => return format!("{}_{}+External_Program", program as u16 + 1, internal),
        };
        format!("{}_{}", program as u16 + 1, name)
    }
}
