use std::io;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event, KeyCode},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Layout, Direction},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    text::{Line, Span, Text},
};
use ratatui::widgets::Paragraph;

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Menu options and selected flags
    let options = vec![
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Gray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::White,
    ];
    let mut selected_flags = vec![false; options.len()];

    let mut state = ListState::default();
    state.select(Some(0));
    let mut user_colour_pick:Option<Color> = None;
    let mut llm_colour_pick:Option<Color> = None;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let vertical = Layout::vertical([
                Constraint::Min(1),
                Constraint::Length(5),
            ]).split(size);

            let list_area = vertical[0];
            let display_area = vertical[1];


            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(list_area);

            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(i, &opt)| {
                    let style = if selected_flags[i] {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(opt)
                    };
                    ListItem::new(opt.to_string()).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(
                    match selected_flags.iter().filter(|&n| *n == true).count(){
                        0 => "Select a Colour for You",
                        _ => "Select a Colour for LLM",
                    }))
                .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
                .highlight_symbol(">> ");

            f.render_stateful_widget(list, list_area, &mut state);
            let help = Paragraph::new(
                    // if let Some(mut i) = state.selected() {
                    //         i = previous_selectable(&selected_flags, i);
                    //         state.select(Some(i));
                    //     }

                    if let Some(mut i) = state.selected() {
                        if let Some(user_colour_picked) = user_colour_pick {
                            Text::from(vec![
                                Line::from(vec![
                                    Span::raw("Selecting for user: "),
                                    Span::styled(format!("{}", user_colour_picked.to_string()), Style::default().fg(user_colour_picked)),
                                ]),
                                Line::from(vec![
                                    Span::raw("Selecting for LLM: "),
                                    Span::styled(format!("{}", options[i].to_string()), Style::default().fg(options[i])),
                                ]),
                                Line::from("Press 'ESC' to return to chat"),
                            ])
                        }else{
                            Text::from(vec![
                                Line::from(vec![
                                    Span::raw("Selecting for user: "),
                                    Span::styled(format!("{}", options[i].to_string()), Style::default().fg(options[i])),
                                ]),
                                Line::from("Press 'ESC' to return to chat"),
                            ])
                        }
                        // format!("Selecting:{}\nPress 'ESC' to return to chat",options[i].to_string())
                        
                    } else {
                        //format!("Selecting for you...\nPress 'ESC' to return to chat")
                        Text::from(vec![
                            Line::from("Selecting for you..."),
                            Line::from("Press 'ESC' to return to chat"),
                        ])
                    })
                .block(Block::default().borders(Borders::ALL).title("Help"));

            f.render_widget(help, display_area);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Up => {
                        if let Some(mut i) = state.selected() {
                            i = previous_selectable(&selected_flags, i);
                            state.select(Some(i));
                        }
                    }
                    KeyCode::Down => {
                        if let Some(mut i) = state.selected() {
                            i = next_selectable(&selected_flags, i);
                            state.select(Some(i));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(i) = state.selected() {
                            if !selected_flags[i] {
                                // println!("Selected: {}", options[i]);
                                selected_flags[i] = true;

                                let count = selected_flags.iter().filter(|&n| *n == true).count();
                                if count==2{
                                    break
                                }else if count==1{
                                    user_colour_pick=Some(options[i]);
                                }
                                // Move to next selectable
                                let next = next_selectable(&selected_flags, i);
                                state.select(Some(next));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

/// Get next selectable index
fn next_selectable(selected_flags: &Vec<bool>, mut index: usize) -> usize {
    let len = selected_flags.len();
    for _ in 0..len {
        index = (index + 1) % len;
        if !selected_flags[index] {
            return index;
        }
    }
    index // fallback
}

/// Get previous selectable index
fn previous_selectable(selected_flags: &Vec<bool>, mut index: usize) -> usize {
    let len = selected_flags.len();
    for _ in 0..len {
        if index == 0 {
            index = len - 1;
        } else {
            index -= 1;
        }
        if !selected_flags[index] {
            return index;
        }
    }
    index
}
