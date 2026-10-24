use super::theme::{ACCENT, BG, MUTED};
use crate::{
    app::App,
    model::{BAR, PPQ},
};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Widget},
};

pub(super) fn draw(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.project.tracks[app.track];
    let title = format!(
        " {} · {:?} · ch {} · {} · vol {} ",
        t.name,
        t.route,
        t.channel + 1,
        app.instrument_label(),
        t.volume
    );
    f.render_widget(Block::bordered().title(title), area);
    f.render_widget(Roll(app), area);
}

struct Roll<'a>(&'a App);
struct GridArea {
    x: u16,
    y: u16,
    right: u16,
    bottom: u16,
    width: u16,
}

impl Widget for Roll<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let grid = GridArea {
            x: area.x + 6,
            y: area.y + 3,
            right: area.right() - 1,
            bottom: area.bottom() - 1,
            width: (area.right() - 1).saturating_sub(area.x + 6),
        };
        self.ruler(area, &grid, buf);
        self.background(area, &grid, buf);
        self.notes(&grid, buf);
        self.overlays(area, &grid, buf);
    }
}

impl Roll<'_> {
    fn ruler(&self, area: Rect, grid: &GridArea, buf: &mut Buffer) {
        let app = self.0;
        for x in grid.x..grid.right {
            let tick = app.offset + ((x - grid.x) / app.cell_width) as u32 * app.grid;
            if (x - grid.x).is_multiple_of(app.cell_width) && tick.is_multiple_of(PPQ) {
                let text = if tick.is_multiple_of(BAR) {
                    format!("{}", tick / BAR + 1)
                } else {
                    "·".into()
                };
                buf.set_stringn(
                    x,
                    area.y + 1,
                    text,
                    (grid.right - x) as usize,
                    Style::default().fg(MUTED),
                );
            }
        }
    }

    fn background(&self, area: Rect, grid: &GridArea, buf: &mut Buffer) {
        let app = self.0;
        for y in grid.y..grid.bottom {
            let row = y - grid.y;
            if row > app.top_key as u16 {
                break;
            }
            let key = app.top_key - row as u8;
            let black = matches!(key % 12, 1 | 3 | 6 | 8 | 10);
            let color = if black {
                Color::Rgb(21, 23, 32)
            } else {
                Color::Rgb(29, 32, 43)
            };
            buf.set_stringn(
                area.x + 1,
                y,
                format!("{:4}", note_name(key)),
                4,
                Style::default().fg(if black { MUTED } else { Color::White }),
            );
            for x in grid.x..grid.right {
                let tick = app.offset + ((x - grid.x) / app.cell_width) as u32 * app.grid;
                let boundary = (x - grid.x).is_multiple_of(app.cell_width);
                let outside = tick < app.project.loop_start || tick >= app.project.loop_end;
                buf[(x, y)]
                    .set_symbol(if boundary && tick.is_multiple_of(BAR) {
                        "┃"
                    } else if boundary {
                        "│"
                    } else {
                        " "
                    })
                    .set_bg(if outside { BG } else { color })
                    .set_fg(if outside {
                        Color::Rgb(29, 31, 40)
                    } else {
                        Color::Rgb(45, 48, 62)
                    });
            }
        }
    }

    fn notes(&self, grid: &GridArea, buf: &mut Buffer) {
        let app = self.0;
        for (i, n) in app.project.tracks[app.track].notes.iter().enumerate() {
            if n.key > app.top_key {
                continue;
            }
            let y = grid.y + (app.top_key - n.key) as u16;
            if y >= grid.bottom {
                continue;
            }
            let end = n.start + n.duration;
            if end <= app.offset {
                continue;
            }
            let begin_col = ((n.start.saturating_sub(app.offset) as u64 * app.cell_width as u64)
                / app.grid as u64)
                .min(grid.width as u64) as u16;
            let end_col = (((end - app.offset) as u64 * app.cell_width as u64)
                .div_ceil(app.grid as u64))
            .min(grid.width as u64) as u16;
            let selected = app.selected.contains(&i);
            for col in begin_col..end_col {
                buf[(grid.x + col, y)]
                    .set_symbol(if col == begin_col {
                        "▐"
                    } else if col + 1 == end_col {
                        "▌"
                    } else {
                        "━"
                    })
                    .set_bg(if selected {
                        Color::Rgb(139, 111, 40)
                    } else {
                        Color::Rgb(29, 104, 85)
                    })
                    .set_fg(if selected { Color::Yellow } else { ACCENT });
            }
        }
    }

    fn overlays(&self, area: Rect, grid: &GridArea, buf: &mut Buffer) {
        let app = self.0;
        for x in grid.x..grid.right {
            let tick = app.offset + (x - grid.x) as u32 * app.grid / app.cell_width as u32;
            if tick < app.project.loop_start || tick >= app.project.loop_end {
                for y in grid.y..grid.bottom {
                    buf[(x, y)].set_style(Style::default().fg(MUTED).add_modifier(Modifier::DIM));
                }
            }
        }
        if app.project.loop_end >= app.offset {
            let col = (app.project.loop_end - app.offset) as u64 * app.cell_width as u64
                / app.grid as u64;
            if col < grid.width as u64 {
                let x = grid.x + col as u16;
                buf.set_stringn(
                    x,
                    area.y + 2,
                    "END",
                    (grid.right - x) as usize,
                    Style::default().fg(Color::Yellow),
                );
                for y in grid.y..grid.bottom {
                    buf[(x, y)].set_symbol("┃").set_fg(Color::Yellow);
                }
            }
        }
        if app.playing() && app.position() >= app.offset {
            let col =
                (app.position() - app.offset) as u64 * app.cell_width as u64 / app.grid as u64;
            if col < grid.width as u64 {
                for y in grid.y..grid.bottom {
                    buf[(grid.x + col as u16, y)].set_style(
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(66, 70, 85))
                            .add_modifier(Modifier::BOLD),
                    );
                }
            }
        }
        if app.cursor >= app.offset && app.key <= app.top_key {
            let col = (app.cursor - app.offset) as u64 * app.cell_width as u64 / app.grid as u64;
            let y = grid.y + (app.top_key - app.key) as u16;
            if col < grid.width as u64 && y < grid.bottom {
                buf[(grid.x + col as u16, y)]
                    .set_symbol("◆")
                    .set_fg(Color::Yellow);
            }
        }
    }
}

fn note_name(key: u8) -> String {
    format!(
        "{}{}",
        [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"
        ][key as usize % 12],
        key as i16 / 12 - 1
    )
}
