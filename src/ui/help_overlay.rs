//! Help overlay showing all keybindings
//!
//! Renders a centered modal overlay with keyboard shortcuts.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Renders the help overlay on top of the current view
pub fn render(frame: &mut Frame) {
    let area = frame.area();

    // Calculate centered overlay area
    let overlay_width = 50;
    let overlay_height = 20;
    let overlay_area = centered_rect(overlay_width, overlay_height, area);

    // Clear the area behind the overlay
    frame.render_widget(Clear, overlay_area);

    // Build help content
    let lines = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        help_line("↑/k, ↓/j", "Move selection up/down"),
        help_line("Enter", "Open beach details"),
        help_line("Esc", "Go back / Close"),
        help_line("q", "Quit application"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Activities",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        help_line("1", "Swimming"),
        help_line("2", "Sunbathing"),
        help_line("3", "Sailing"),
        help_line("4", "Sunset viewing"),
        help_line("5", "Peace & quiet"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Other",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        help_line("p", "Plan trip grid"),
        help_line("r", "Refresh data"),
        help_line("?", "Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc or ? to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, overlay_area);
}

/// Creates a help line with key and description
fn help_line(key: &str, description: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<12}", key), Style::default().fg(Color::Yellow)),
        Span::raw(description.to_string()),
    ])
}

/// Helper function to create a centered rect
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Length((area.height.saturating_sub(height)) / 2),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Length((area.width.saturating_sub(width)) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_help_overlay_renders() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render(frame);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(content.contains("Help"), "Should render help title");
        assert!(
            content.contains("Navigation"),
            "Should show navigation section"
        );
    }
}
