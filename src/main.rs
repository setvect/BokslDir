mod app;
mod core;
mod models;
mod system;
mod ui;
mod utils;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use utils::error::Result;

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.area();

            // Create main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),      // Header
                    Constraint::Min(0),         // Content
                    Constraint::Length(3),      // Footer
                ])
                .split(size);

            // Header
            let header = Paragraph::new("복슬Dir (Boksl Dir) - TUI File Manager")
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Welcome"));
            f.render_widget(header, chunks[0]);

            // Content - Dual Panel Layout
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            let left_panel = Paragraph::new("Left Panel\n\nPress 'q' to quit")
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Left")
                        .border_style(Style::default().fg(Color::Green)),
                );
            f.render_widget(left_panel, content_chunks[0]);

            let right_panel = Paragraph::new("Right Panel")
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Right")
                        .border_style(Style::default().fg(Color::Gray)),
                );
            f.render_widget(right_panel, content_chunks[1]);

            // Footer
            let footer = Paragraph::new("F1:Help | F5:Copy | F6:Move | F8:Delete | q:Quit")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Commands"));
            f.render_widget(footer, chunks[2]);
        })?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.quit(),
                    KeyCode::Esc => app.quit(),
                    _ => {}
                }
            }
        }

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}
