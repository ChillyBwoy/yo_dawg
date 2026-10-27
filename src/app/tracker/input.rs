use super::{COLUMN_WIDTH, Projection, TIME_WIDTH};
use crate::{app::App, model::BAR};
use crossterm::event::{
    KeyCode as K, KeyEvent, KeyModifiers as M, MouseButton, MouseEvent, MouseEventKind,
};

impl App {
    pub(in crate::app) fn handle_tracker_key(&mut self, event: KeyEvent) -> bool {
        let shift = event.modifiers.contains(M::SHIFT);
        let alt = event.modifiers.contains(M::ALT);
        match event.code {
            K::Up | K::Down if alt => {
                self.key = (self.key as i16 + if event.code == K::Up { 1 } else { -1 })
                    .clamp(0, 127) as u8;
            }
            K::PageUp | K::PageDown if alt => {
                self.key = (self.key as i16 + if event.code == K::PageUp { 12 } else { -12 })
                    .clamp(0, 127) as u8;
            }
            K::Up if !shift => self.move_tracker_row(-1),
            K::Down if !shift => self.move_tracker_row(1),
            K::PageUp => self.move_tracker_row(-(self.tracker.visible_rows() as i64)),
            K::PageDown => self.move_tracker_row(self.tracker.visible_rows() as i64),
            K::Left | K::Right if !shift => {
                let projection = Projection::new(&self.project, self.grid);
                let index = projection.column_index(self.track, self.tracker.lane);
                let next = if event.code == K::Left {
                    index.saturating_sub(1)
                } else {
                    (index + 1).min(projection.columns.len() - 1)
                };
                self.focus_tracker_column(next);
            }
            K::Home => {
                self.cursor = self.project.loop_start;
                self.selected.clear();
                self.focus_tracker_note();
                self.ensure_tracker_time_visible();
            }
            K::End => {
                self.cursor = (self.project.loop_end - 1) / self.grid * self.grid;
                self.selected.clear();
                self.focus_tracker_note();
                self.ensure_tracker_time_visible();
            }
            K::Char('e') => self.edit_tracker_note(false),
            K::Insert | K::Enter if shift => self.edit_tracker_note(true),
            K::Char('+') | K::Char('=') | K::Char('-') => {
                self.status = "Tracker row resolution: [ / ]".into()
            }
            _ => return false,
        }
        true
    }

    pub(in crate::app) fn tracker_mouse(&mut self, event: MouseEvent) {
        let area = self.tracker.area;
        if !area.contains((event.column, event.row).into()) {
            return;
        }
        match event.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let up = event.kind == MouseEventKind::ScrollUp;
                if event.modifiers.contains(M::SHIFT) {
                    let count = Projection::new(&self.project, self.grid).columns.len();
                    self.tracker.column_offset = if up {
                        self.tracker.column_offset.saturating_sub(1)
                    } else {
                        (self.tracker.column_offset + 1).min(count - 1)
                    };
                } else {
                    self.tracker.top_tick = if up {
                        self.tracker.top_tick.saturating_sub(self.grid * 3)
                    } else {
                        (self.tracker.top_tick + self.grid * 3).min(BAR * 4096 - self.grid)
                    };
                }
            }
            MouseEventKind::Down(button)
                if matches!(button, MouseButton::Left | MouseButton::Right) =>
            {
                if event.column < area.x + TIME_WIDTH
                    || event.column >= area.right() - 1
                    || event.row >= area.bottom() - 1
                {
                    return;
                }
                let local = ((event.column - area.x - TIME_WIDTH) / COLUMN_WIDTH) as usize;
                if local >= self.tracker.visible_columns() {
                    return;
                }
                let index = self.tracker.column_offset + local;
                let count = Projection::new(&self.project, self.grid).columns.len();
                if index >= count {
                    return;
                }
                if event.row >= area.y + 3 {
                    let tick = (self.tracker.top_tick / self.grid
                        + (event.row - area.y - 3) as u32)
                        * self.grid;
                    if tick >= BAR * 4096 {
                        return;
                    }
                    self.cursor = tick;
                }
                self.focus_tracker_column(index);
                if button == MouseButton::Right && event.row >= area.y + 3 {
                    self.delete();
                }
            }
            _ => {}
        }
    }
}
