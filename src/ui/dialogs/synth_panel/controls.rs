use crate::{
    ui::theme::{ACCENT, MUTED},
    vst::ParameterView,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Paragraph},
};
use tui_slider::{Slider, SliderState};

pub(super) fn draw(
    frame: &mut Frame,
    parameter: &ParameterView,
    index: usize,
    selected: bool,
    area: Rect,
) {
    let color = if selected { Color::Yellow } else { ACCENT };
    let block = Block::bordered()
        .title(format!(
            " {} {:02} {} ",
            if selected { "›" } else { " " },
            index + 1,
            parameter.name
        ))
        .border_style(Style::default().fg(if selected { color } else { MUTED }));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let value = if parameter.steps == 1 {
        if parameter.name == "Square wave" {
            if parameter.value >= 0.5 {
                "SQUARE"
            } else {
                "SAW"
            }
        } else if parameter.value >= 0.5 {
            "ON"
        } else {
            "OFF"
        }
    } else {
        &parameter.display
    };
    frame.render_widget(
        Paragraph::new(value).style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    if inner.height > 1 && inner.width >= 4 {
        let state = SliderState::new(parameter.value, 0.0, 1.0);
        frame.render_widget(
            Slider::from_state(&state)
                .show_value(false)
                .filled_color(color)
                .empty_color(MUTED)
                .handle_color(Color::White)
                .focused(selected),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );
    }
}
