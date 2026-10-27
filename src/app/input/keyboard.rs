use crate::{
    app::{Action, App, FileAction, InstrumentPicker, ViewMode},
    engine::Command,
    model::{BAR, Track},
};
use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
use std::sync::atomic::Ordering;

impl App {
    pub fn key_event(&mut self, event: KeyEvent) {
        if self.drag.take().is_some() {
            self.sync();
        }
        if self.handle_dialog_key(event) {
            return;
        }
        let key = event.code;
        let shift = event.modifiers.contains(M::SHIFT);
        if key == K::Esc {
            self.settings = false;
            self.selected.clear();
            return;
        }
        if event.modifiers.contains(M::CONTROL) {
            self.handle_control_key(key);
            return;
        }
        if key == K::F(4) {
            self.set_view(if self.view == ViewMode::PianoRoll {
                ViewMode::Tracker
            } else {
                ViewMode::PianoRoll
            });
            return;
        }
        if self.view == ViewMode::Tracker && self.handle_tracker_key(event) {
            return;
        }
        match key {
            K::Char('q') => self.request_quit(),
            K::Char('?') | K::F(1) => self.help = true,
            K::F(2) => self.settings = !self.settings,
            K::F(3) => self.open_file_browser(FileAction::ImportMidi),
            K::F(5) => self.refresh_ports(),
            K::F(6) => self.cycle_input(),
            K::F(7) => self.cycle_output(),
            K::F(8) => {
                self.thru = !self.thru;
                self.engine.send(Command::Thru(self.thru));
            }
            K::F(9) => {
                let on = !self.audio.monitor.load(Ordering::Relaxed);
                self.audio.monitor.store(on, Ordering::Relaxed);
            }
            K::Char(' ') => {
                self.finish_recording();
                self.engine.send(Command::Play(!self.playing()));
            }
            K::Char('!') => {
                self.finish_recording();
                self.engine.send(Command::Panic);
            }
            K::Char('R') => {
                self.finish_recording();
                self.recording = !self.recording;
                self.engine.send(Command::Record(self.recording));
            }
            K::Tab => self.select_track((self.track + 1) % self.project.tracks.len()),
            K::BackTab => self.select_track(
                (self.track + self.project.tracks.len() - 1) % self.project.tracks.len(),
            ),
            K::Char('a') => {
                if self.project.tracks.len() < 16 {
                    self.checkpoint();
                    self.project
                        .tracks
                        .push(Track::new(self.project.tracks.len()));
                    self.select_track(self.project.tracks.len() - 1);
                    self.sync();
                }
            }
            K::Char('m') => {
                self.checkpoint();
                self.project.tracks[self.track].mute ^= true;
                self.sync();
            }
            K::Char('s') => {
                self.checkpoint();
                self.project.tracks[self.track].solo ^= true;
                self.sync();
            }
            K::Char('r') => {
                self.checkpoint();
                let t = &mut self.project.tracks[self.track];
                t.route = t.route.next();
                self.sync();
            }
            K::Char('n') => self.prompt(
                "Track name",
                self.project.tracks[self.track].name.clone(),
                Action::Rename,
            ),
            K::Char('p') => {
                self.instrument_picker = Some(InstrumentPicker {
                    selected: self.project.tracks[self.track].program as usize,
                    offset: 0,
                    page_size: 10,
                })
            }
            K::Char('c') => self.prompt(
                "MIDI channel (1..16)",
                (self.project.tracks[self.track].channel + 1).to_string(),
                Action::Channel,
            ),
            K::Char('b') => self.prompt("BPM (20..300)", self.project.bpm.to_string(), Action::Bpm),
            K::Char('l') => self.prompt(
                "Length in bars (from loop start)",
                ((self.project.loop_end - self.project.loop_start) / BAR).to_string(),
                Action::Length,
            ),
            K::Char('{') => self.prompt(
                "Loop start bar (1..4096)",
                (self.project.loop_start / BAR + 1).to_string(),
                Action::LoopStart,
            ),
            K::Char('}') => self.prompt(
                "Loop end bar (exclusive)",
                (self.project.loop_end / BAR + 1).to_string(),
                Action::LoopEnd,
            ),
            K::Char('<') | K::Char('>') => {
                self.checkpoint();
                let v = &mut self.project.tracks[self.track].volume;
                *v = (*v as i16 + if key == K::Char('>') { 5 } else { -5 }).clamp(0, 127) as u8;
                self.sync();
            }
            K::Left | K::Right | K::Up | K::Down if shift => {
                let dt = if key == K::Left {
                    -(self.grid as i32)
                } else if key == K::Right {
                    self.grid as i32
                } else {
                    0
                };
                let dk = if key == K::Up {
                    1
                } else if key == K::Down {
                    -1
                } else {
                    0
                };
                self.transform(dt, dk, false, 0);
            }
            K::Left => self.cursor = self.cursor.saturating_sub(self.grid),
            K::Right => self.cursor = (self.cursor + self.grid).min(BAR * 4096 - self.grid),
            K::Up => self.key = self.key.saturating_add(1).min(127),
            K::Down => self.key = self.key.saturating_sub(1),
            K::Home => self.cursor = self.project.loop_start,
            K::PageUp => self.key = self.key.saturating_add(12).min(127),
            K::PageDown => self.key = self.key.saturating_sub(12),
            K::Enter | K::Insert => self.toggle_note(),
            K::Delete | K::Backspace => self.delete(),
            K::Char('v') => {
                if let Some(i) = self.note_under_cursor() {
                    if !self.selected.insert(i) {
                        self.selected.remove(&i);
                    }
                }
            }
            K::Char('g') => self.quantize(),
            K::Char('[') => self.grid = (self.grid / 2).max(30),
            K::Char(']') => self.grid = (self.grid * 2).min(BAR),
            K::Char('+') | K::Char('=') => self.cell_width = (self.cell_width + 1).min(12),
            K::Char('-') => self.cell_width = self.cell_width.saturating_sub(1).max(1),
            _ => {}
        }
        // Follow deliberate cursor navigation only, and only along its axis.
        // Other shortcuts must preserve a viewport scrolled away from the cursor.
        match key {
            _ if self.view == ViewMode::Tracker => {}
            K::Left | K::Right if !shift => self.ensure_time_visible(),
            K::Up | K::Down if !shift => self.ensure_pitch_visible(),
            K::Home => self.ensure_time_visible(),
            K::PageUp | K::PageDown => self.ensure_pitch_visible(),
            _ => {}
        }
    }
}
