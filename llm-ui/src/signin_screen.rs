use ratatui::Frame;

use ratatui::{
    layout::{Constraint, Layout, Position},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Paragraph},
};
use crate::app::App;

pub fn render_signin(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let vertical = Layout::vertical( [
        Constraint::Length(3),
        Constraint::Length(3),
    ]);
    let [help_area, input_area] = vertical.areas(area);
    let msg = vec![
        "Enter username below to sign in, press ".into(),
        "Enter".bold(),
        " to submit.".into(),
    ];
    let style = Style::default();
    
    let text = Text::from(Line::from(msg)).patch_style(style);
    let help_msg = Paragraph::new(text);
    frame.render_widget(help_msg, help_area);
    let input_box = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::bordered().title("Username"));
    frame.render_widget(input_box, input_area);

    frame.set_cursor_position(Position::new(
        input_area.x + app.character_index as u16 + 1,
        input_area.y + 1,
    ));

}
