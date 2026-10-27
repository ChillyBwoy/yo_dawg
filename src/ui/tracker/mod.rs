mod details;
pub(super) use details::draw as details;
#[cfg(test)]
mod tracker_tests;

use crate::{
    app::{
        App,
        tracker::{
            COLUMN_WIDTH, TIME_WIDTH, pitch_name,
            projection::{Cell, Projection},
        },
    },
    model::{BAR, PPQ},
    ui::theme::{ACCENT, BG, LIVE_NOTE, MUTED},
};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Block,
};

pub(super) fn draw(f: &mut Frame, app: &App, area: Rect) {
    let projection = Projection::new(&app.project, app.grid);
    let count = projection.columns.len();
    f.render_widget(
        Block::bordered().title(format!(
            " Tracker · {} tracks / {} note columns · [ / ] rows ",
            app.project.tracks.len(),
            count
        )),
        area,
    );
    let buffer = f.buffer_mut();
    buffer.set_stringn(
        area.x + 1,
        area.y + 1,
        "BAR:ROW",
        (TIME_WIDTH - 2) as usize,
        Style::default().fg(MUTED),
    );
    buffer.set_stringn(
        area.x + 1,
        area.y + 2,
        format!("1/{}", BAR / app.grid),
        (TIME_WIDTH - 2) as usize,
        Style::default().fg(MUTED),
    );
    for (visible, column) in projection
        .columns
        .iter()
        .skip(app.tracker.column_offset)
        .take(app.tracker.visible_columns())
        .enumerate()
    {
        let x = area.x + TIME_WIDTH + visible as u16 * COLUMN_WIDTH;
        let track = &app.project.tracks[column.track];
        let active = column.track == app.track && column.lane == app.tracker.lane;
        let header = format!(
            "{:02} {} /{}",
            column.track + 1,
            track.name,
            column.lane + 1
        );
        buffer.set_stringn(
            x + 1,
            area.y + 1,
            header,
            (COLUMN_WIDTH - 2) as usize,
            Style::default()
                .fg(if active { Color::Yellow } else { ACCENT })
                .add_modifier(Modifier::BOLD),
        );
        buffer.set_stringn(
            x + 1,
            area.y + 2,
            "NOTE INS VEL LEN(ticks)",
            (COLUMN_WIDTH - 2) as usize,
            Style::default().fg(MUTED),
        );
        let held = app.engine.live_notes(column.track);
        for row in 0..app.tracker.visible_rows() {
            let tick = (app.tracker.top_tick / app.grid + row) * app.grid;
            if tick >= BAR * 4096 {
                break;
            }
            let y = area.y + 3 + row as u16;
            let cursor = tick / app.grid == app.cursor / app.grid && active;
            let playing = app.playing() && tick / app.grid == app.position() / app.grid;
            let outside = tick < app.project.loop_start || tick >= app.project.loop_end;
            let cell = column.cell(&app.project, tick, app.grid);
            let note_index = cell.note_index();
            let selected =
                column.track == app.track && note_index.is_some_and(|i| app.selected.contains(&i));
            let live = note_index.is_some_and(|i| held.contains(track.notes[i].key));
            let bg = if cursor {
                Color::Rgb(46, 71, 84)
            } else if live {
                LIVE_NOTE
            } else if selected {
                Color::Rgb(105, 83, 32)
            } else if playing {
                Color::Rgb(47, 51, 65)
            } else if tick.is_multiple_of(PPQ) && !outside {
                Color::Rgb(29, 32, 43)
            } else {
                BG
            };
            let fg = if cursor || selected {
                Color::Yellow
            } else if live {
                Color::White
            } else if outside || !app.project.audible(column.track) {
                MUTED
            } else if matches!(cell, Cell::Start(_)) {
                ACCENT
            } else {
                MUTED
            };
            let style = Style::default().bg(bg).fg(fg);
            for px in x..x + COLUMN_WIDTH {
                buffer[(px, y)].set_symbol(" ").set_style(style);
            }
            let text = match cell {
                Cell::Start(i) => {
                    let n = &track.notes[i];
                    format!(
                        "{:<4} {:03} {:03} {:06}{}",
                        pitch_name(n.key),
                        track.program + 1,
                        n.velocity,
                        n.duration,
                        if n.start != tick { "*" } else { "" }
                    )
                }
                Cell::Hold(_) => "===  ... ... ......".into(),
                Cell::Off => "OFF  ... ... ......".into(),
                Cell::Empty => "...  ... ... ......".into(),
            };
            buffer.set_stringn(x + 1, y, text, (COLUMN_WIDTH - 2) as usize, style);
            if cursor {
                buffer[(x, y)].set_symbol("›");
            }
            buffer[(x + COLUMN_WIDTH - 1, y)]
                .set_symbol("│")
                .set_fg(MUTED);
        }
    }
    draw_row_numbers(buffer, app, area);
}

fn draw_row_numbers(buffer: &mut Buffer, app: &App, area: Rect) {
    for row in 0..app.tracker.visible_rows() {
        let tick = (app.tracker.top_tick / app.grid + row) * app.grid;
        if tick >= BAR * 4096 {
            break;
        }
        let playing = app.playing() && tick / app.grid == app.position() / app.grid;
        let cursor = tick / app.grid == app.cursor / app.grid;
        let text = format!(
            "{}{:03}:{:02}",
            if playing {
                "▶"
            } else if cursor {
                "›"
            } else {
                " "
            },
            tick / BAR + 1,
            tick % BAR / app.grid
        );
        buffer.set_stringn(
            area.x + 1,
            area.y + 3 + row as u16,
            text,
            (TIME_WIDTH - 2) as usize,
            Style::default().fg(if playing {
                Color::White
            } else if cursor {
                Color::Yellow
            } else {
                MUTED
            }),
        );
    }
}
