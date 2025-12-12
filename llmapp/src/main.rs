use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
    prelude::{Alignment, Stylize},
};
use std::io;
use tui_big_text::PixelSize;
use tui_big_text::BigText;
#[derive(Clone, Copy, PartialEq)]

enum ButtonState {
    Normal,
    Focused,
}

struct Button {
    label: &'static str,
    state: ButtonState,
}

impl Button {
    fn new(label: &'static str) -> Self {
        Button {
            label,
            state: ButtonState::Normal,
        }
    }

    fn style(&self) -> Style {
        match self.state {
            ButtonState::Normal => Style::default().fg(Color::White),
            ButtonState::Focused => Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        }
    }
}

struct App<'a> {
    buttons: Vec<Button>,
    selected: usize,
    text:&'a str,
}

impl App<'_> {
    fn new() -> Self {
        Self {
            buttons: vec![
                Button::new("Chat Screen"),
                Button::new("Chat History"),
                Button::new("Text Colour Selection"),
                Button::new("Quit"),
            ],
            selected: 0,
            text:"hi",
        }
    }

    fn update_button_states(&mut self) {
        for (i, btn) in self.buttons.iter_mut().enumerate() {
            if i == self.selected {
                btn.state = ButtonState::Focused;
            } else {
                btn.state = ButtonState::Normal;
            }
        }
    }

    fn next_button(&mut self) {
        self.selected = (self.selected + 1).min(self.buttons.len() - 1);
        self.update_button_states();
    }

    fn previous_button(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.update_button_states();
    }
}

fn main() -> Result<(), io::Error> {
    // Terminal init
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.update_button_states();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let vertical = Layout::vertical([
                    Constraint::Min(1),
                    Constraint::Min(12),
                ]);
            let [title_banner,buttons] = vertical.areas(f.area());
            // Horizontal layout: LEFT SPACE | BUTTON COLUMN | RIGHT SPACE
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
                // .vertical_margin((size.height.saturating_sub(12)) / 2) // center vertically
                .split(cols[1]);
            // let title = Paragraph::new("LLM Chat Interface").alignment(Alignment::Center);
            let title = BigText::builder().centered()
                        .pixel_size(PixelSize::Quadrant)
                        .style(Style::new().light_blue())
                        .lines(vec![
                            "LLM Chat Interface".into(),
                            "~~~~~~~".white().into(),
                        ])
                        .build();
            f.render_widget(title, title_banner);

            for (i, btn) in app.buttons.iter().enumerate() {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .style(btn.style())
                    .border_style(btn.style());

                let text = Paragraph::new(btn.label).style(btn.style()).alignment(Alignment::Center).block(block);

                f.render_widget(text, button_chunks[i]);
            }
        })?;

        // Input handling
        if event::poll(std::time::Duration::from_millis(40))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => app.previous_button(),
                    KeyCode::Down => app.next_button(),
                    KeyCode::Enter => {
                        if app.selected==0{
                            app.text = "1";
                        } else if app.selected==1{
                            app.text = "2";
                        } else if app.selected==2{
                            app.text = "3";
                        } else if app.selected==3{
                            break
                        }
                    },
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }
    }

    // Shutdown terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;

    Ok(())
}
