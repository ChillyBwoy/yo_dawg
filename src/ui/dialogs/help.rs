use super::centered;
use ratatui::{
    Frame,
    widgets::{Block, Clear, Paragraph, Wrap},
};

pub(super) fn draw(f: &mut Frame) {
    let rect = centered(f.area(), 100, 27);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(include_str!("help.txt").trim_end())
            .block(Block::bordered().title(" Piano roll · Help "))
            .wrap(Wrap { trim: false }),
        rect,
    );
}
