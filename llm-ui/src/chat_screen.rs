use ratatui::Frame;
use ratatui::{
    layout::{Constraint, Layout, Position,  Rect, Direction},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tui_big_text::{BigText, PixelSize};

use itertools::Itertools;
use itertools::EitherOrBoth::{Both, Left, Right};

use crate::app::{App, InputMode};


pub fn render_chat(frame: &mut Frame, app: &App) {
    let user:&str = "You: ";
    let llm:&str = "LLM: ";
    let mut scroll_offset = app.scroll_offset;

    let vertical = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        // Constraint::Min(1),
        Constraint::Min(1),
        Constraint::Length(3),
    ]);
    let [title_banner,help_area, response_area, input_area] = vertical.areas(frame.area());
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(95), Constraint::Percentage(5)].as_ref())
        .split(response_area);
    let chat_area = horizontal[0];
    let scrollbar_area = horizontal[1];

    let title = BigText::builder().centered()
        .pixel_size(PixelSize::Quadrant)
        .style(Style::new().light_blue())
        .lines(vec![
            "LLM Chat Interface".into(),
        ])
        .build();
    frame.render_widget(title, title_banner);
    
    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                "Press ".into(),
                "e".bold(),
                " to enter a prompt, ".into(),
                "ESC".bold(),
                " to return to main menu.".into(),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                "Press ".into(),
                "ESC".bold(),
                " to stop editing, ".into(),
                "Enter".bold(),
                " to record the message, ".into(),
            ],
            Style::default(),
        ),
        InputMode::Fetching => (
            vec![
                "Press ".into(),
                "ESC".bold(),
                " to return to main menu, ".into(),
                "enter chat ID to resume ".into(),
            ],
            Style::default(),
        ),
        InputMode::Processing => (
            {
                let mut spans = Vec::new();
                spans.push("Processing".into());

                if let Some(start_time) = app.stream_start {
                    if app.token_count > 0 {
                        let elapsed = start_time.elapsed().as_secs_f32();
                        if elapsed > 0.0 {
                            let rate = app.token_count as f32 / elapsed;
                            spans.push(" | Tokens: ".into());
                            spans.push(format!("{}", app.token_count).bold());
                            spans.push(" | Rate: ".into());
                            spans.push(format!("{:.1} tok/s", rate).bold());
                        }
                    }
                }

                spans
            },
            Style::default(),
        ),
        InputMode::ColourSelection => (
            vec![
                "Processing colour selection".into(),
            ],
            Style::default(),
        ),
        InputMode::MainMenu => (
            vec![
                "Main menu".into(),
            ],
            Style::default(),
        ),
    };
    let t = Text::from(Line::from(msg)).patch_style(style);
    let help_message = Paragraph::new(t);
    frame.render_widget(help_message, help_area);

    let input =  match app.input_mode {
        InputMode::Processing => Paragraph::new("Wait for response...")
            .style(Style::default())
            .block(Block::bordered().title("Input")),
        InputMode::Normal => Paragraph::new("Enter a prompt!")
            .style(Style::default())
            .block(Block::bordered().title("Input")),
        InputMode::Editing => 
            Paragraph::new(app.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::bordered().title("Input")),
        InputMode::Fetching => 
            Paragraph::new(app.input.as_str())
            .style(Style::default().fg(Color::Cyan))
            .block(Block::bordered().title("Chat ID Input")),
        InputMode::ColourSelection => 
            Paragraph::new(app.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::bordered().title("Colour Input")),
        InputMode::MainMenu => 
            Paragraph::new(app.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::bordered().title("Main Menu")),
    };
    
    frame.render_widget(input, input_area);
    match app.input_mode {
    // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
        InputMode::Normal => {}

        // Make the cursor visible and ask ratatui to put it at the specified coordinates after
        // rendering
        #[allow(clippy::cast_possible_truncation)]
        InputMode::Editing => frame.set_cursor_position(Position::new(
            // Draw the cursor at the current position in the input field.
            // This position is can be controlled via the left and right arrow key
            input_area.x + app.character_index as u16 + 1,
            // Move one line down, from the border to the input line
            input_area.y + 1,
        )),
        InputMode::Processing => {},
        InputMode::ColourSelection => {},
        InputMode::Fetching => frame.set_cursor_position(Position::new(
            input_area.x + app.character_index as u16 + 1,
            input_area.y + 1,
        )),
        InputMode::MainMenu => {},
    }

    let mut spans=Vec::new();
    let mut messages = "".to_string();
    for item in app.messages.iter().zip_longest(app.llm_messages.iter()) {
        match item {
            Both(a, b) => {
                // messages.push(ListItem::new(user.to_string()+&a));
                let user_span = Span::styled(user.to_string()+&a, Style::default().fg(app.user_colour));
                spans.push(Line::from(vec![user_span]));
                // let llm_span = Span::styled(llm.to_string()+&b, Style::default().fg(app.llm_colour));
                // spans.push(Line::from(vec![llm_span]));
                for line in b.lines(){
                    let llm_span = Span::styled(llm.to_string()+line, Style::default().fg(app.llm_colour));
                    spans.push(Line::from(vec![llm_span]));
                }
                messages+=&format!("{} {}\n",user, a);
                messages+=&format!("{} {}\n",llm, b);
                // messages.push(ListItem::new(llm.to_string()+&b));
            }
            Left(a) => {
                let user_span = Span::styled(user.to_string()+&a, Style::default().fg(app.user_colour));
                spans.push(Line::from(vec![user_span]));
                messages+=&format!("{} {}\n",user, a);
            }
            Right(b) => {
                // let llm_span = Span::styled(llm.to_string()+&b, Style::default().fg(app.llm_colour));
                // spans.push(Line::from(vec![llm_span]));
                for line in b.lines(){
                    let llm_span = Span::styled(llm.to_string()+line, Style::default().fg(app.llm_colour));
                    spans.push(Line::from(vec![llm_span]));
                }
                messages+=&format!("{} {}\n",llm, b);
            }
        }
    }
    if !app.receiving.is_empty(){
        let llm_span = Span::styled(llm.to_string()+&app.receiving, Style::default().fg(app.llm_colour));
        spans.push(Line::from(vec![llm_span]));
        messages+=&format!("{} {}\n",llm, app.receiving);

    }
    // Count total wrapped lines
    let total_lines = count_wrapped_lines(&messages, chat_area.width)+2;

    // Clamp scroll
    scroll_offset = scroll_offset.min(total_lines.saturating_sub(chat_area.height));

    let text = Text::from(spans);
    let paragraph = Paragraph::new(text.clone())
        .block(Block::default().borders(Borders::ALL).title("Chat"))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, chat_area);
    draw_scrollbar(frame, scrollbar_area, scroll_offset, total_lines, chat_area.height);
}

// Count number of wrapped lines for given text and width
fn count_wrapped_lines(text: &str, width: u16) -> u16 {
    let width = width as usize;
    let mut lines = 0;

    for raw_line in text.lines() {
        let mut remaining = raw_line.to_string(); // use String
        while !remaining.is_empty() {
            let take = std::cmp::min(width, remaining.chars().count());
            remaining = remaining.chars().skip(take).collect();
            lines += 1;
        }
    }

    lines
}

fn draw_scrollbar(
    f: &mut ratatui::Frame,
    area: Rect,
    scroll: u16,
    total_lines: u16,
    viewport_height: u16,
) {
    if total_lines <= viewport_height {
        return;
    }

    let scrollbar_height = std::cmp::max(1, viewport_height * viewport_height / total_lines);
    let max_scroll = total_lines.saturating_sub(viewport_height);
    let scroll_pos = if max_scroll > 0 {
        scroll * (viewport_height - scrollbar_height) / max_scroll
    } else {
        0
    };

    for i in 0..scrollbar_height {
        let y = area.y + scroll_pos + i;
        if y < area.y + area.height {
            f.render_widget(
                Paragraph::new("█").style(Style::default().fg(Color::Gray)),
                Rect { x: area.x, y, width: 1, height: 1 },
            );
        }
    }
}
