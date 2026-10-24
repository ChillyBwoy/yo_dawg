mod help;
mod instrument_picker;
mod routing;

use crate::app::{App, Prompt};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

pub(super) fn draw(f: &mut Frame, app: &mut App) {
    if app.settings {
        routing::draw(f, app);
    }
    if app.help {
        help::draw(f);
    }
    if app.instrument_picker.is_some() {
        instrument_picker::draw(f, app);
    }
    if let Some(prompt) = &app.prompt {
        draw_prompt(f, prompt);
    }
    if app.quit_confirm {
        draw_quit(f);
    }
}

fn draw_prompt(f: &mut Frame, prompt: &Prompt) {
    let area = f.area();
    let rect = centered(area, area.width.saturating_sub(8).min(100), 5);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                prompt.text.as_str(),
                if prompt.selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                },
            )),
            Line::from("Enter: confirm · Ctrl+A: select all · Esc: cancel"),
        ])
        .block(Block::bordered().title(prompt.label))
        .wrap(Wrap { trim: false }),
        rect,
    );
    let col = prompt
        .text
        .chars()
        .count()
        .min(rect.width.saturating_sub(3) as usize) as u16;
    f.set_cursor_position((rect.x + 1 + col, rect.y + 1));
}

fn draw_quit(f: &mut Frame) {
    let rect = centered(f.area(), 56.min(f.area().width), 5);
    f.render_widget(Clear, rect);
    f.render_widget(Paragraph::new("Unsaved changes. Quit and discard them?\ny: quit · any other key: return and save with Ctrl+S")
        .block(Block::bordered().title(" Quit? ")).wrap(Wrap { trim: false }), rect);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
