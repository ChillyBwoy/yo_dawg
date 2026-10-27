use super::centered;
use crate::{
    app::{App, BrowserFocus},
    ui::theme::{ACCENT, BG, MUTED},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, FrameExt, Paragraph, Wrap},
};
use ratatui_explorer::Theme;

pub(super) fn draw(f: &mut Frame, app: &mut App) {
    let browser = app.file_browser.as_mut().unwrap();
    let area = f.area();
    let rect = centered(
        area,
        area.width.saturating_sub(6).min(104),
        area.height.saturating_sub(4).min(28),
    );
    f.render_widget(Clear, rect);
    let block = Block::bordered()
        .title(format!(
            " {} · {} ",
            browser.action.title(),
            browser.action.filter_label()
        ))
        .style(Style::default().bg(BG).fg(Color::White));
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .split(inner);
    f.render_widget(
        Paragraph::new(format!("Directory: {}", browser.explorer.cwd().display()))
            .style(Style::default().fg(MUTED)),
        rows[0],
    );

    let active = browser.focus == BrowserFocus::List;
    browser.explorer.set_theme(
        Theme::default()
            .with_block(
                Block::bordered()
                    .title(" Files ")
                    .border_style(Style::default().fg(if active { ACCENT } else { MUTED })),
            )
            .with_style(Style::default().bg(BG).fg(Color::White))
            .with_dir_style(Style::default().fg(ACCENT))
            .with_highlight_item_style(Style::default().fg(Color::White).bg(Color::Rgb(38, 75, 69)))
            .with_highlight_dir_style(Style::default().fg(ACCENT).bg(Color::Rgb(38, 75, 69)))
            .with_highlight_symbol("› "),
    );
    // The explorer widget assumes a current item; handle an empty filtered root ourselves.
    if browser.explorer.files().is_empty() {
        f.render_widget(
            Paragraph::new("No matching files").block(Block::bordered().title(" Files ")),
            rows[1],
        );
    } else {
        f.render_widget_ref(browser.explorer.widget(), rows[1]);
    }

    let active = browser.focus == BrowserFocus::Path;
    let text_style = if active && browser.selected_text {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    let width = rows[2].width.saturating_sub(2).max(1) as usize;
    let scroll = browser
        .input
        .chars()
        .count()
        .saturating_sub(width - 1)
        .min(u16::MAX as usize) as u16;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(browser.input.as_str(), text_style)))
            .scroll((0, scroll))
            .block(
                Block::bordered()
                    .title(if browser.action.is_save() {
                        " File name / path "
                    } else {
                        " File path "
                    })
                    .border_style(Style::default().fg(if active { ACCENT } else { MUTED })),
            ),
        rows[2],
    );
    if active && browser.overwrite.is_none() {
        let col = browser
            .input
            .chars()
            .count()
            .saturating_sub(scroll as usize)
            .min(width - 1) as u16;
        f.set_cursor_position((rows[2].x + 1 + col, rows[2].y + 1));
    }
    let (message, color) = if let Some(path) = &browser.overwrite {
        (
            format!(
                "Replace existing file? {}\ny: overwrite · n / Esc: return without overwriting",
                path.display()
            ),
            Color::Yellow,
        )
    } else if let Some(error) = &browser.error {
        (error.clone(), Color::Red)
    } else {
        (
            "Choose a directory or file. Tab switches between the list and path field.".into(),
            MUTED,
        )
    };
    f.render_widget(
        Paragraph::new(message)
            .style(Style::default().fg(color))
            .wrap(Wrap { trim: false }),
        rows[3],
    );
    f.render_widget(Paragraph::new("↑/↓ PgUp/PgDn Home/End · Enter select/open · ←/Backspace parent\nTab field/list · Ctrl+L path · Ctrl+A select text · . hidden · F5 refresh · Esc cancel")
        .style(Style::default().fg(MUTED)), rows[4]);
}
