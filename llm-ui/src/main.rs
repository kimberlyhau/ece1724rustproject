
use anyhow::{Result};
use ratatui::Terminal;
use std::io;
use tokio::sync::mpsc;
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};

mod app;
mod chat_screen;
mod signin_screen;
mod history_screen;
mod colour_screen;
mod key_handler;
mod menu_screen;

use app::{App, ChatOutcome, Screen};




#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    // Setup terminal   
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Channel for streaming text
    let (tx, mut rx) = mpsc::channel::<String>(100);

    let mut app = App::new();

    loop {
        terminal.draw(|frame| {
            match app.screen {
                Screen::Chat => chat_screen::render_chat(frame, &app),
                Screen::SignIn => signin_screen::render_signin(frame, &app),
                Screen::History => history_screen::render_history(frame, &app),
                Screen::ColourSelection => colour_screen::render_colour(frame, &mut app),
                Screen::MainMenu => menu_screen::render_menu(frame, &mut app),
            }
        })?;

        match key_handler::key_handler(&mut app, tx.clone(), &mut rx).await.unwrap() {
            ChatOutcome::Continue => {},
            ChatOutcome::Quit => break,
        }
    }

        

            /* 
            _ => {
                terminal.draw(|frame| {
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
            */ 
                
    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    // ratatui::restore();

    Ok(())
}
