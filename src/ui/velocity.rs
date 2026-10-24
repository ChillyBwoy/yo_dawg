use super::theme::ACCENT;
use crate::app::App;
use ratatui::{Frame, layout::Rect, style::Color, widgets::Block};

pub(super) fn draw(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Block::bordered().title(" Velocity · Ctrl+↑/↓ "), area);
    let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
    for (i, n) in app.project.tracks[app.track].notes.iter().enumerate() {
        if n.start < app.offset {
            continue;
        }
        let column = (n.start - app.offset) as u64 * app.cell_width as u64 / app.grid as u64;
        if column >= app.roll.width.saturating_sub(7) as u64 {
            continue;
        }
        let x = app.roll.x + 6 + column as u16;
        if x >= inner.right() {
            continue;
        }
        let height = (n.velocity as u16 * inner.height).div_ceil(127).max(1);
        let color = if app.selected.contains(&i) {
            Color::Yellow
        } else {
            ACCENT
        };
        for y in inner.bottom() - height..inner.bottom() {
            f.buffer_mut()[(x, y)].set_symbol("▎").set_fg(color);
        }
    }
}
