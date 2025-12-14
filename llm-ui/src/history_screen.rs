use ratatui::Frame;
use ratatui::{
    layout::{Constraint, Layout, Position},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::App;

pub fn render_history(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(1),
    ]).split(area);

    let help_area = vertical[0];
    let input_area = vertical[1];
    let history_area = vertical[2];

    let help_msg = vec![
        "Enter chat ID below to resume chat. Press ".into(),
        "Enter".bold(),
        " to submit.".into(),
        " Press ".into(),
        "ESC".bold(),
        " to return to main menu.".into(),
    ];
    let style = Style::default();

    let text = Text::from(Line::from(help_msg)).patch_style(style);
    let help_message= Paragraph::new(text);
    frame.render_widget(help_message, help_area);

    let input_box = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::bordered().title("Chat ID"));
    frame.render_widget(input_box, input_area);
    frame.set_cursor_position(Position::new(
        input_area.x + app.character_index as u16 + 1,
        input_area.y + 1,
    ));

    let history_text: String = app
        .history_messages
        .iter()
        .enumerate()
        .map(|(i, msg)| format!("{}: {}", i + 1, msg))
        .collect();

    let history_paragraph = Paragraph::new(history_text)
        .block(Block::default().borders(Borders::ALL).title("Chat History"))
        .wrap(Wrap { trim: false });

    frame.render_widget(history_paragraph, history_area);
}