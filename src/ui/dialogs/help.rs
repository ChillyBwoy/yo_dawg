use super::centered;
use crate::app::ViewMode;
use ratatui::{
    Frame,
    widgets::{Block, Clear, Paragraph, Wrap},
};

pub(super) fn draw(f: &mut Frame, view: ViewMode) {
    let rect = centered(f.area(), 100, 27);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(
            if view == ViewMode::Tracker {
                include_str!("tracker_help.txt")
            } else {
                include_str!("help.txt")
            }
            .trim_end(),
        )
        .block(Block::bordered().title(format!(" {} · Help ", view.label())))
        .wrap(Wrap { trim: false }),
        rect,
    );
}
