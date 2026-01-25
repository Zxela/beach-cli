//! Vancouver Beach CLI - View beach conditions for Vancouver beaches
//!
//! A terminal UI application that displays weather, tides, and water quality
//! information for beaches in Vancouver, BC.

mod activities;
mod app;
mod cache;
mod crowd;
mod data;
mod ui;

use std::io;
use std::panic;
use std::time::Duration;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, AppState};

/// Sets up a panic hook that restores the terminal before printing the panic message.
/// This ensures the terminal is usable even if the application panics.
fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Attempt to restore the terminal
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        // Call the original panic hook
        original_hook(panic_info);
    }));
}

/// Renders the UI based on the current application state
fn render_ui(frame: &mut ratatui::Frame, app: &App) {
    match &app.state {
        AppState::Loading => {
            render_loading(frame);
        }
        AppState::BeachList => {
            ui::render_beach_list(frame, app);
        }
        AppState::BeachDetail(beach_id) => {
            ui::render_beach_detail(frame, app, beach_id);
        }
    }
}

/// Renders a loading message while data is being fetched
fn render_loading(frame: &mut ratatui::Frame) {
    use ratatui::{
        layout::{Alignment, Constraint, Direction, Layout},
        style::{Color, Style},
        widgets::Paragraph,
    };

    let area = frame.area();

    // Center the loading message vertically
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(3),
            Constraint::Percentage(45),
        ])
        .split(area);

    let loading_text = Paragraph::new("Loading beach data...")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);

    frame.render_widget(loading_text, chunks[1]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up panic hook to restore terminal on crash
    setup_panic_hook();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app instance
    let mut app = App::new();

    // Initial render to show loading state
    terminal.draw(|f| render_ui(f, &app))?;

    // Trigger initial data load
    app.load_all_data().await;

    // Main event loop
    loop {
        // Render UI
        terminal.draw(|f| render_ui(f, &app))?;

        // Poll for keyboard events with 100ms timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
