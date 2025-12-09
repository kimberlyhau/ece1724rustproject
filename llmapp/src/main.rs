use std::io;
use std::time::{Duration, Instant};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event, KeyCode},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
    style::{Color, Style},
    text::Text,
};

fn main() -> Result<(), io::Error> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initial text
    let mut text_content = String::from(
        "Welcome! This is a scrollable paragraph.\nScroll manually or wait for new text to appear...\n",
    );

    let mut scroll_offset: u16 = 0;
    let mut last_update = Instant::now();

    loop {
        terminal.draw(|f| {
            let size = f.size();

            // Vertical split: chat area + help
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(90), Constraint::Length(3)].as_ref())
                .split(size);

            let chat_area = vertical_chunks[0];
            let help_area = vertical_chunks[1];

            // Horizontal split for paragraph + scrollbar
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(95), Constraint::Percentage(5)].as_ref())
                .split(chat_area);

            let paragraph_area = chunks[0];
            let scrollbar_area = chunks[1];

            // Count wrapped lines
            let total_lines = count_wrapped_lines(&text_content, paragraph_area.width);

            // Auto-scroll if we are at the bottom
            if scroll_offset + paragraph_area.height >= total_lines {
                scroll_offset = total_lines.saturating_sub(paragraph_area.height);
            }

            // Paragraph widget
            let paragraph = Paragraph::new(Text::from(text_content.clone()))
                .block(Block::default().borders(Borders::ALL).title("Chat"))
                .wrap(Wrap { trim: false })
                .scroll((scroll_offset, 0));

            f.render_widget(paragraph, paragraph_area);

            // Draw scrollbar
            draw_scrollbar(f, scrollbar_area, scroll_offset, total_lines, paragraph_area.height);

            // Help area
            let help_text = Paragraph::new("Help: Up/Down = scroll, PageUp/PageDown = scroll faster, q = quit")
                .block(Block::default().borders(Borders::ALL).title("Help"))
                .wrap(Wrap { trim: true });
            f.render_widget(help_text, help_area);
        })?;

        // Simulate new text every 2 seconds
        if last_update.elapsed() > Duration::from_secs(2) {
            text_content.push_str("New message arrived!\n");
            last_update = Instant::now();
        }

        // Input handling
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Up => scroll_offset = scroll_offset.saturating_sub(1),
                    KeyCode::Down => scroll_offset = scroll_offset.saturating_add(1),
                    KeyCode::PageUp => scroll_offset = scroll_offset.saturating_sub(5),
                    KeyCode::PageDown => scroll_offset = scroll_offset.saturating_add(5),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

/// Count number of wrapped lines for a given text and width
fn count_wrapped_lines(text: &str, width: u16) -> u16 {
    let width = width as usize;
    let mut lines = 0;

    for raw_line in text.lines() {
        let mut remaining = raw_line.to_string();
        while !remaining.is_empty() {
            let take = std::cmp::min(width, remaining.chars().count());
            remaining = remaining.chars().skip(take).collect();
            lines += 1;
        }
    }

    lines
}

/// Draw vertical scrollbar
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
