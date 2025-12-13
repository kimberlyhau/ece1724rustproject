use ratatui::Frame;

use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Borders, Paragraph},
};

use crate::app::App;
use crate::app::OPTIONS as options;

pub fn render_colour(frame: &mut Frame, app: &mut App) {
    let vertical = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(5),
    ]).split(frame.area());

    let list_area = vertical[0];
    let display_area = vertical[1];

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, &opt)| {
            let style = if app.selected_flags[i] {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(opt)
            };
            ListItem::new(opt.to_string()).style(style)
        }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(
            match app.selected_flags.iter().filter(|&n| *n == true).count(){
                0 => "Select a Colour for You",
                _ => "Select a Colour for LLM",
            }))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, list_area, &mut app.state);
    let colour_info = Paragraph::new(
if let Some(i) = app.state.selected() {
        if let Some(user_colour_picked) = app.user_colour_pick {
            Text::from(vec![
                Line::from(vec![
                    Span::raw("Selecting for user: "),
                    Span::styled(format!("{}", user_colour_picked.to_string()), Style::default().fg(user_colour_picked)),
                ]),
                Line::from(vec![
                    Span::raw("Selecting for LLM: "),
                    Span::styled(format!("{}", options[i].to_string()), Style::default().fg(options[i])),
                ]),
                Line::from("Press 'ESC' to return to main menu."),
            ])
        }else{
            Text::from(vec![
                Line::from(vec![
                    Span::raw("Selecting for user: "),
                    Span::styled(format!("{}", options[i].to_string()), Style::default().fg(options[i])),
                ]),
                Line::from("Press 'ESC' to return to main menu."),
                ])
        }
        // format!("Selecting:{}\nPress 'ESC' to return to chat",options[i].to_string())
                        
    } else {
        //format!("Selecting for you...\nPress 'ESC' to return to chat")
        Text::from(vec![
            Line::from("Selecting..."),
            Line::from("Press 'ESC' to return to chat"),
        ])
    })
    .block(Block::default().borders(Borders::ALL).title("Info"));

    frame.render_widget(colour_info, display_area);
}