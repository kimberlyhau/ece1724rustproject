use ratatui::Frame;
use ratatui::{
    layout::{Constraint, Layout, Direction},
    style::{Style, Stylize},
    widgets::{Block, Borders, Paragraph},
    prelude::Alignment,
};
use tui_big_text::{BigText, PixelSize};

use crate::app::App;

pub fn render_menu(frame: &mut Frame, app: &mut App) {
    let vertical = Layout::vertical([
        Constraint::Min(1),
        Constraint::Min(12),
    ]);
    let [title_banner,buttons] = vertical.areas(frame.area());
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(30), // center column
            Constraint::Percentage(35),
        ])
        .split(buttons);

    // Vertical stack of buttons inside center column
    let button_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(cols[1]);
    let title = BigText::builder().centered()
        .pixel_size(PixelSize::Quadrant)
        .style(Style::new().light_blue())
        .lines(vec![
            "LLM Chat Interface".into(),
            "~~~~~~~".white().into(),
        ])
        .build();
    frame.render_widget(title, title_banner);

    for (i, btn) in app.buttons.iter().enumerate() {
        let block = Block::default()
            .borders(Borders::ALL)
            .style(btn.style())
            .border_style(btn.style());

        let text = Paragraph::new(btn.label).style(btn.style()).alignment(Alignment::Center).block(block);

        frame.render_widget(text, button_chunks[i]);
    }
}