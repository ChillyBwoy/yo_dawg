use crate::{
    app::{App, Drag, ViewMode},
    model::BAR,
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers as M, MouseButton, MouseEvent, MouseEventKind,
};

impl App {
    pub fn mouse(&mut self, event: MouseEvent) {
        if self.file_browser.is_some() {
            let key = match event.kind {
                MouseEventKind::ScrollUp => Some(KeyCode::Up),
                MouseEventKind::ScrollDown => Some(KeyCode::Down),
                _ => None,
            };
            if let Some(key) = key {
                self.handle_file_browser_key(KeyEvent::new(key, M::NONE));
            }
            return;
        }
        if self.prompt.is_some()
            || self.instrument_picker.is_some()
            || self.help
            || self.settings
            || self.quit_confirm
        {
            return;
        }
        if self.view == ViewMode::Tracker {
            self.tracker_mouse(event);
            return;
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left)
            && self.track_area.contains((event.column, event.row).into())
        {
            let index =
                event.row.saturating_sub(self.track_area.y + 1) as usize + self.track_offset;
            if index < self.project.tracks.len() {
                self.select_track(index);
            }
            return;
        }
        if self.edit_velocity(event) {
            return;
        }
        if let Some((tick, key)) = self.point(event.column, event.row) {
            match event.kind {
                MouseEventKind::Down(MouseButton::Left) => self.begin_drag(event, tick, key),
                MouseEventKind::Down(MouseButton::Right) => {
                    self.cursor = tick;
                    self.key = key;
                    self.selected.clear();
                    self.delete();
                }
                MouseEventKind::Drag(MouseButton::Left) => self.update_drag(tick, key),
                _ => {}
            }
        }
        match event.kind {
            MouseEventKind::Up(MouseButton::Left) => {
                if self.drag.take().is_some() {
                    self.sync();
                }
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => self.scroll(event),
            _ => {}
        }
    }

    fn edit_velocity(&mut self, event: MouseEvent) -> bool {
        if event.kind != MouseEventKind::Down(MouseButton::Left)
            || !self
                .velocity_area
                .contains((event.column, event.row).into())
            || event.column < self.roll.x + 6
            || event.row <= self.velocity_area.y
            || event.row >= self.velocity_area.bottom() - 1
        {
            return false;
        }
        let tick =
            self.offset + ((event.column - self.roll.x - 6) / self.cell_width) as u32 * self.grid;
        let indices: Vec<_> = self.project.tracks[self.track]
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.start / self.grid == tick / self.grid)
            .map(|(i, _)| i)
            .collect();
        if !indices.is_empty() {
            self.checkpoint();
            let height = self.velocity_area.height.saturating_sub(2).max(1);
            let velocity =
                ((self.velocity_area.bottom() - 1 - event.row) * 127 / height).clamp(1, 127) as u8;
            for i in indices {
                self.project.tracks[self.track].notes[i].velocity = velocity;
            }
            self.sync();
        }
        true
    }

    fn begin_drag(&mut self, event: MouseEvent, tick: u32, key: u8) {
        self.cursor = tick;
        self.key = key;
        if let Some(i) = self.note_at(tick, key) {
            if event.modifiers.contains(M::SHIFT) {
                if !self.selected.insert(i) {
                    self.selected.remove(&i);
                }
                return;
            }
            if !self.selected.contains(&i) {
                self.selected.clear();
                self.selected.insert(i);
            }
            let note = self.project.tracks[self.track].notes[i].clone();
            let end_column = ((note.start + note.duration - self.offset) as u64
                * self.cell_width as u64)
                .div_ceil(self.grid as u64);
            let resize = (event.column - self.roll.x - 6) as u64 + 1 >= end_column;
            self.checkpoint();
            self.drag = Some(Drag {
                index: i,
                tick,
                key,
                original: note,
                resize,
                group: self
                    .selected
                    .iter()
                    .map(|i| (*i, self.project.tracks[self.track].notes[*i].clone()))
                    .collect(),
            });
        } else {
            self.toggle_note();
            let i = self.project.tracks[self.track].notes.len() - 1;
            self.drag = Some(Drag {
                index: i,
                tick,
                key,
                original: self.project.tracks[self.track].notes[i].clone(),
                resize: true,
                group: Vec::new(),
            });
        }
    }

    fn update_drag(&mut self, tick: u32, key: u8) {
        let Some(drag) = self.drag.clone() else {
            return;
        };
        if drag.resize {
            let duration = (drag.original.duration as i64 + tick as i64 - drag.tick as i64)
                .clamp(1, (BAR * 4096 - drag.original.start) as i64)
                as u32;
            self.project.tracks[self.track].notes[drag.index].duration = duration;
        } else {
            let min_start = drag.group.iter().map(|(_, n)| n.start).min().unwrap_or(0) as i64;
            let max_end = drag
                .group
                .iter()
                .map(|(_, n)| n.start + n.duration)
                .max()
                .unwrap_or(0) as i64;
            let min_key = drag.group.iter().map(|(_, n)| n.key).min().unwrap_or(0) as i16;
            let max_key = drag.group.iter().map(|(_, n)| n.key).max().unwrap_or(127) as i16;
            let dt =
                (tick as i64 - drag.tick as i64).clamp(-min_start, (BAR * 4096) as i64 - max_end);
            let dk = (key as i16 - drag.key as i16).clamp(-min_key, 127 - max_key);
            for (i, n) in drag.group {
                let note = &mut self.project.tracks[self.track].notes[i];
                note.start = (n.start as i64 + dt) as u32;
                note.key = (n.key as i16 + dk) as u8;
            }
        }
    }

    fn scroll(&mut self, event: MouseEvent) {
        let up = event.kind == MouseEventKind::ScrollUp;
        if event.modifiers.contains(M::CONTROL) {
            self.cell_width =
                (self.cell_width as i16 + if up { 1 } else { -1 }).clamp(1, 12) as u16;
        } else if event.modifiers.contains(M::SHIFT) {
            self.offset = if up {
                self.offset.saturating_sub(self.grid * 4)
            } else {
                (self.offset + self.grid * 4).min(BAR * 4096 - self.grid)
            };
        } else {
            self.top_key = (self.top_key as i16 + if up { 3 } else { -3 }).clamp(0, 127) as u8;
        }
    }
}
