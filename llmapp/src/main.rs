use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::io::{self};

#[derive(Debug, Clone)]
struct ChatMessage {
    from_user: bool,
    text: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample messages
    let messages = vec![
        ChatMessage {
            from_user: false,
            text: "Hello! I am ChatGPT.".into(),
        },
        ChatMessage {
            from_user: true,
            text: "Hey! Can you show me chat bubbles in Rust?".into(),
        },
        ChatMessage {
            from_user: false,
            text: "Sure! Here you go 🙂".into(),
        },
    ];

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, messages);

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    messages: Vec<ChatMessage>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    messages
                        .iter()
                        .map(|_| Constraint::Length(3))
                        .collect::<Vec<_>>(),
                )
                .split(f.size());

            for (i, msg) in messages.iter().enumerate() {
                let bubble = Paragraph::new(msg.text.clone())
                    .wrap(Wrap { trim: true })
                    .block(
                        Block::default()
                            .title(if msg.from_user { "You" } else { "Bot" })
                            .borders(Borders::ALL)
                            .style(
                                if msg.from_user {
                                    Style::default().fg(Color::Black).bg(Color::Cyan)
                                } else {
                                    Style::default().fg(Color::White).bg(Color::Blue)
                                },
                            ),
                    );

                // Left-align bot, right-align user
                let area = if msg.from_user {
                    // Right side bubble
                    Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(vec![Constraint::Fill(1), Constraint::Length(50)])
                        .split(chunks[i])[1]
                } else {
                    // Left side bubble
                    Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(vec![Constraint::Length(50), Constraint::Fill(1)])
                        .split(chunks[i])[0]
                };

                f.render_widget(bubble, area);
            }
        })?;

        // Exit on 'q'
        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }
    Ok(())
}
