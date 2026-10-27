use super::App;
use crate::model::BAR;

impl App {
    pub fn ensure_visible(&mut self) {
        self.ensure_time_visible();
        self.ensure_pitch_visible();
    }

    pub(super) fn ensure_time_visible(&mut self) {
        let steps = (self.roll.width.saturating_sub(6) / self.cell_width).max(1) as u32;
        if self.cursor < self.offset {
            self.offset = self.cursor / self.grid * self.grid;
        }
        if self.cursor >= self.offset + steps * self.grid {
            self.offset = (self.cursor / self.grid + 1).saturating_sub(steps) * self.grid;
        }
    }

    pub(super) fn ensure_pitch_visible(&mut self) {
        let rows = self.roll.height.saturating_sub(4).clamp(1, 128) as u8;
        if self.key > self.top_key {
            self.top_key = self.key;
        }
        if self.key < self.top_key.saturating_sub(rows - 1) {
            self.top_key = self.key.saturating_add(rows - 1).min(127);
        }
    }

    pub(super) fn point(&self, x: u16, y: u16) -> Option<(u32, u8)> {
        let r = self.roll;
        if x < r.x + 6
            || x >= r.right().saturating_sub(1)
            || y < r.y + 3
            || y >= r.bottom().saturating_sub(1)
        {
            return None;
        }
        let row = y - r.y - 3;
        if row > self.top_key as u16 {
            return None;
        }
        let tick = self.offset + ((x - r.x - 6) / self.cell_width) as u32 * self.grid;
        if tick >= BAR * 4096 {
            return None;
        }
        Some((tick, self.top_key - row as u8))
    }
}
