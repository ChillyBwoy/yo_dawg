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
    EditNote,
}

pub struct InstrumentPicker {
    pub selected: usize,
    pub offset: usize,
    pub page_size: usize,
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
            self.handle_instrument_key(key);
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

    fn handle_instrument_key(&mut self, key: K) {
        let picker = self.instrument_picker.as_mut().unwrap();
        match key {
            K::Up => picker.selected = picker.selected.saturating_sub(1),
            K::Down => picker.selected = (picker.selected + 1).min(127),
            K::PageUp => picker.selected = picker.selected.saturating_sub(picker.page_size),
            K::PageDown => picker.selected = (picker.selected + picker.page_size).min(127),
            K::Home => picker.selected = 0,
            K::End => picker.selected = 127,
            K::Esc => self.instrument_picker = None,
            K::Enter => {
                let program = picker.selected as u8;
                self.instrument_picker = None;
                if self.project.tracks[self.track].program != program {
                    self.checkpoint();
                    self.project.tracks[self.track].program = program;
                    self.sync();
                    self.status = format!("Instrument: {}", self.instrument_label());
                }
            }
            _ => {}
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
        self.checkpoint();
        self.project = candidate;
        if matches!(action, Action::Length | Action::LoopStart | Action::LoopEnd) {
            self.cursor = self
                .cursor
                .clamp(self.project.loop_start, self.project.loop_end - 1);
            self.status = format!(
                "Length: {} bars. Notes outside the loop are preserved.",
                (self.project.loop_end - self.project.loop_start) / BAR
            );
        }
        self.sync();
        Ok(())
    }
}
