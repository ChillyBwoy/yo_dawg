use super::App;
use crate::model::BAR;
use anyhow::Result;
use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};

pub struct Prompt {
    pub label: &'static str,
    pub text: String,
    pub action: Action,
    pub selected: bool,
}

#[derive(Clone, Copy)]
pub enum Action {
    Rename,
    Channel,
    Bpm,
    LoopStart,
    LoopEnd,
    Length,
    TrackLength,
    EditNote,
}

pub struct InstrumentPicker {
    pub page: SynthPage,
    pub selected: usize,
    pub offset: usize,
    pub page_size: usize,
}

pub enum SynthPage {
    Synthesizers(Vec<crate::vst::SynthKind>),
    Instruments,
    Parameters(Vec<crate::vst::ParameterView>),
}

impl App {
    pub(super) fn prompt(&mut self, label: &'static str, text: String, action: Action) {
        self.prompt = Some(Prompt {
            label,
            text,
            action,
            selected: matches!(
                action,
                Action::Channel
                    | Action::Bpm
                    | Action::LoopStart
                    | Action::LoopEnd
                    | Action::Length
                    | Action::TrackLength
                    | Action::EditNote
            ),
        });
    }

    /// Returns true when a modal consumes the event, preventing editor shortcuts.
    pub(super) fn handle_dialog_key(&mut self, event: KeyEvent) -> bool {
        let key = event.code;
        if self.quit_confirm {
            match key {
                K::Char('y') | K::Char('Y') => self.quit = true,
                _ => self.quit_confirm = false,
            }
            return true;
        }
        if self.file_browser.is_some() {
            self.handle_file_browser_key(event);
            return true;
        }
        if self.instrument_picker.is_some() {
            self.handle_instrument_key(event);
            return true;
        }
        if self.prompt.is_some() {
            self.handle_prompt_key(event);
            return true;
        }
        if self.help {
            self.help = false;
            return true;
        }
        false
    }

    fn handle_instrument_key(&mut self, event: KeyEvent) {
        let key = event.code;
        if matches!(key, K::F(10) | K::Char('P')) {
            self.toggle_synth_panel();
            return;
        }
        let mut picker = self.instrument_picker.take().unwrap();
        let count = match &picker.page {
            SynthPage::Synthesizers(items) => items.len(),
            SynthPage::Instruments => 128,
            SynthPage::Parameters(items) => items.len(),
        };
        let last = count.saturating_sub(1);
        if matches!(picker.page, SynthPage::Parameters(_)) {
            if event.modifiers.contains(M::CONTROL) && matches!(key, K::Char('z' | 'y')) {
                self.undo(key == K::Char('y'));
                picker.page = SynthPage::Parameters(self.synth_parameters());
                self.instrument_picker = Some(picker);
                return;
            }
            if key == K::Char(' ') {
                self.finish_recording();
                self.engine
                    .send(crate::engine::Command::Play(!self.playing()));
                self.instrument_picker = Some(picker);
                return;
            }
        }
        match key {
            K::Up => picker.selected = picker.selected.saturating_sub(1),
            K::Down => picker.selected = (picker.selected + 1).min(last),
            K::PageUp => picker.selected = picker.selected.saturating_sub(picker.page_size),
            K::PageDown => picker.selected = (picker.selected + picker.page_size).min(last),
            K::Home => picker.selected = 0,
            K::End => picker.selected = last,
            K::Esc => {
                if !matches!(picker.page, SynthPage::Synthesizers(_)) {
                    self.open_synth_picker();
                }
                return;
            }
            K::Enter => match &picker.page {
                SynthPage::Synthesizers(items) => {
                    let Some(kind) = items.get(picker.selected).cloned() else {
                        return;
                    };
                    if self.project.tracks[self.track].synth.kind != kind {
                        self.checkpoint();
                        self.project.tracks[self.track].synth = crate::vst::SynthConfig {
                            kind: kind.clone(),
                            ..Default::default()
                        };
                        self.sync();
                    }
                    picker.selected = 0;
                    picker.offset = 0;
                    picker.page = if kind == crate::vst::SynthKind::Basic {
                        picker.selected = self.project.tracks[self.track].program as usize;
                        SynthPage::Instruments
                    } else {
                        SynthPage::Parameters(self.synth_parameters())
                    };
                }
                SynthPage::Instruments => {
                    let program = picker.selected as u8;
                    if self.project.tracks[self.track].program != program {
                        self.checkpoint();
                        self.project.tracks[self.track].program = program;
                        self.sync();
                        self.status = format!("Instrument: {}", self.instrument_label());
                    }
                    return;
                }
                SynthPage::Parameters(parameters) => {
                    if let Some(parameter) =
                        parameters.get(picker.selected).filter(|p| p.steps == 1)
                    {
                        self.set_synth_parameter(
                            parameter,
                            if parameter.value >= 0.5 { 0.0 } else { 1.0 },
                        );
                        picker.page = SynthPage::Parameters(self.synth_parameters());
                    }
                }
            },
            K::Left | K::Right => {
                if let SynthPage::Parameters(parameters) = &picker.page {
                    if let Some(parameter) = parameters.get(picker.selected) {
                        let step = if parameter.steps > 0 {
                            1.0 / parameter.steps as f64
                        } else if event.modifiers.contains(M::SHIFT) {
                            0.001
                        } else {
                            0.01
                        };
                        let direction = if key == K::Left { -1.0 } else { 1.0 };
                        let value = (parameter.value + direction * step).clamp(0.0, 1.0);
                        self.set_synth_parameter(parameter, value);
                        picker.page = SynthPage::Parameters(self.synth_parameters());
                    }
                }
            }
            K::Char('0') => {
                if let SynthPage::Parameters(parameters) = &picker.page {
                    if let Some(parameter) = parameters.get(picker.selected) {
                        self.set_synth_parameter(parameter, parameter.default);
                        picker.page = SynthPage::Parameters(self.synth_parameters());
                    }
                }
            }
            _ => {}
        }
        self.instrument_picker = Some(picker);
    }

    pub(super) fn open_synth_picker(&mut self) {
        let items = crate::vst::available();
        let selected = items
            .iter()
            .position(|kind| *kind == self.project.tracks[self.track].synth.kind)
            .unwrap_or(0);
        self.instrument_picker = Some(InstrumentPicker {
            page: SynthPage::Synthesizers(items),
            selected,
            offset: 0,
            page_size: 10,
        });
    }

    pub(super) fn synth_parameters(&mut self) -> Vec<crate::vst::ParameterView> {
        let Some(vst) = &self.audio.vst else {
            self.status = "Audio is disabled; plugin parameters are unavailable".into();
            return Vec::new();
        };
        match vst.parameters(self.track) {
            Ok(parameters) => parameters,
            Err(error) => {
                self.status = format!("VST3: {error:#}");
                Vec::new()
            }
        }
    }

    fn handle_prompt_key(&mut self, event: KeyEvent) {
        let ctrl = event.modifiers.contains(M::CONTROL);
        let prompt = self.prompt.as_mut().unwrap();
        match event.code {
            K::Esc => self.prompt = None,
            K::Backspace => {
                if prompt.selected {
                    prompt.text.clear();
                } else {
                    prompt.text.pop();
                }
                prompt.selected = false;
            }
            K::Char('a') if ctrl => prompt.selected = true,
            K::Char(c) if !ctrl => {
                if prompt.selected {
                    prompt.text.clear();
                    prompt.selected = false;
                }
                prompt.text.push(c);
            }
            K::Enter => {
                let prompt = self.prompt.take().unwrap();
                if let Err(e) = self.apply_project_setting(prompt.action, &prompt.text) {
                    self.status = format!("Error: {e:#}");
                }
            }
            _ => {}
        }
    }

    fn apply_project_setting(&mut self, action: Action, text: &str) -> Result<()> {
        if matches!(action, Action::EditNote) {
            return self.apply_tracker_note(text);
        }
        let mut candidate = self.project.clone();
        let t = &mut candidate.tracks[self.track];
        match action {
            Action::Rename => t.name = text.chars().take(64).collect(),
            Action::Channel => {
                let n: u8 = text.parse()?;
                anyhow::ensure!((1..=16).contains(&n), "Expected 1..16");
                t.channel = n - 1;
            }
            Action::Bpm => candidate.bpm = text.parse()?,
            Action::Length => {
                let bars: u32 = text.parse()?;
                anyhow::ensure!((1..=4096).contains(&bars), "Length must be 1..4096 bars");
                candidate.loop_end = candidate.loop_start + bars * BAR;
            }
            Action::TrackLength => {
                let length = if let Some(ticks) = text.strip_suffix('t') {
                    ticks.parse::<u32>()?
                } else {
                    let bars: u32 = text.parse()?;
                    anyhow::ensure!(bars <= 4096, "Track length must be 0..4096 bars");
                    bars * BAR
                };
                t.length = if length == 0 { None } else { Some(length) };
            }
            Action::LoopStart | Action::LoopEnd => {
                let bar: u32 = text.parse()?;
                anyhow::ensure!((1..=4097).contains(&bar), "Invalid bar");
                let tick = (bar - 1) * BAR;
                if matches!(action, Action::LoopStart) {
                    candidate.loop_start = tick;
                } else {
                    candidate.loop_end = tick;
                }
            }
            Action::EditNote => unreachable!(),
        }
        candidate.validate()?;
        self.finish_recording();
        // Finalizing held input notes must not be overwritten by the settings snapshot.
        for (candidate, current) in candidate.tracks.iter_mut().zip(&self.project.tracks) {
            candidate.notes.clone_from(&current.notes);
        }
        self.checkpoint();
        self.project = candidate;
        if matches!(
            action,
            Action::Length | Action::TrackLength | Action::LoopStart | Action::LoopEnd
        ) {
            let (start, end) = self.project.track_range(self.track);
            self.cursor = self.cursor.clamp(start, end - 1);
            self.status = format!(
                "Track {}: {} (project cycle {} bars). Notes outside its loop are preserved.",
                self.track + 1,
                self.project.track_length_label(self.track),
                (self.project.loop_end - self.project.loop_start) / BAR
            );
        }
        self.sync();
        Ok(())
    }
}
