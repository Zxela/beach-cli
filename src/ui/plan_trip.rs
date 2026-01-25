//! Plan Trip screen UI
//!
//! Renders the Plan Trip view showing a heatmap grid of beaches (rows) vs hours (columns)
//! with activity scores, cursor navigation, and best recommendation section.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::activities::{get_profile, Activity};
use crate::app::App;
use crate::data::{all_beaches, WaterStatus};

/// Color scheme for the plan trip screen
mod colors {
    use ratatui::style::Color;

    /// Section headers
    pub const HEADER: Color = Color::Cyan;
    /// Primary text
    pub const PRIMARY: Color = Color::White;
    /// Secondary/dimmed text
    pub const SECONDARY: Color = Color::Gray;
    /// Selected activity indicator
    pub const SELECTED: Color = Color::Yellow;
    /// Excellent score (80-100)
    pub const EXCELLENT: Color = Color::Green;
    /// Good score (60-79)
    pub const GOOD: Color = Color::LightGreen;
    /// Fair score (40-59)
    pub const FAIR: Color = Color::Yellow;
    /// Poor score (20-39)
    pub const POOR: Color = Color::LightRed;
    /// Bad score (0-19)
    pub const BAD: Color = Color::Red;
}

/// Block characters for different score ranges
const BLOCK_EXCELLENT: &str = "\u{2588}\u{2588}"; // ██
const BLOCK_GOOD: &str = "\u{2593}\u{2593}"; // ▓▓
const BLOCK_FAIR: &str = "\u{2592}\u{2592}"; // ▒▒
const BLOCK_POOR: &str = "\u{2591}\u{2591}"; // ░░

/// Returns the block character and color for a given score
fn score_to_block(score: u8) -> (&'static str, Color) {
    match score {
        80..=100 => (BLOCK_EXCELLENT, colors::EXCELLENT),
        60..=79 => (BLOCK_GOOD, colors::GOOD),
        40..=59 => (BLOCK_FAIR, colors::FAIR),
        20..=39 => (BLOCK_POOR, colors::POOR),
        _ => (BLOCK_POOR, colors::BAD),
    }
}

/// Computes the score for a beach at a given hour
fn compute_score(app: &App, beach_id: &str, hour: u8) -> u8 {
    let activity = match app.current_activity {
        Some(a) => a,
        None => return 50, // Default score when no activity selected
    };

    let conditions = match app.get_conditions(beach_id) {
        Some(c) => c,
        None => return 50, // Default when no conditions available
    };

    let profile = get_profile(activity);

    // Get weather data for scoring
    let (temp, wind, uv) = match &conditions.weather {
        Some(w) => (w.temperature as f32, w.wind as f32, w.uv as f32),
        None => return 50, // Can't score without weather
    };

    // Get water status
    let water_status = conditions
        .water_quality
        .as_ref()
        .map(|wq| wq.status)
        .unwrap_or(WaterStatus::Unknown);

    // Get tide info
    let (tide_height, max_tide) = match &conditions.tides {
        Some(t) => {
            let max_h = t.next_high.as_ref().map(|h| h.height).unwrap_or(4.8);
            (t.current_height as f32, max_h as f32)
        }
        None => (2.4, 4.8), // Default mid-tide
    };

    // Estimate crowd level based on time of day
    let crowd_level = estimate_crowd_level(hour);

    let score = profile.score_time_slot(
        hour,
        beach_id,
        temp,
        wind,
        uv,
        water_status,
        tide_height,
        max_tide,
        crowd_level,
    );

    score.score
}

/// Estimates crowd level based on time of day (0.0 = empty, 1.0 = packed)
fn estimate_crowd_level(hour: u8) -> f32 {
    match hour {
        6..=7 => 0.1,   // Early morning - very quiet
        8..=9 => 0.2,   // Morning - light
        10..=11 => 0.4, // Late morning - moderate
        12..=14 => 0.8, // Midday - busy
        15..=17 => 0.6, // Afternoon - moderate to busy
        18..=19 => 0.4, // Early evening - moderate
        20..=21 => 0.2, // Evening - light
        _ => 0.5,       // Default
    }
}

/// Finds the best beach/hour combination across all beaches and hours
fn find_best_recommendation(app: &App) -> Option<(String, String, u8, u8)> {
    app.current_activity?;

    let beaches = all_beaches();
    let (start_hour, end_hour) = app.plan_time_range;

    let mut best: Option<(String, String, u8, u8)> = None;
    let mut best_score: u8 = 0;

    for beach in beaches {
        for hour in start_hour..=end_hour {
            let score = compute_score(app, beach.id, hour);
            if score > best_score {
                best_score = score;
                best = Some((beach.name.to_string(), beach.id.to_string(), hour, score));
            }
        }
    }

    best
}

/// Format hour as display string (e.g., "6am", "12pm")
fn format_hour_short(hour: u8) -> String {
    match hour {
        0 => "12am".to_string(),
        1..=11 => format!("{}am", hour),
        12 => "12pm".to_string(),
        13..=23 => format!("{}pm", hour - 12),
        _ => format!("{}h", hour),
    }
}

/// Format hour as longer display string (e.g., "6:00 AM", "12:00 PM")
fn format_hour_long(hour: u8) -> String {
    match hour {
        0 => "12:00 AM".to_string(),
        1..=11 => format!("{}:00 AM", hour),
        12 => "12:00 PM".to_string(),
        13..=23 => format!("{}:00 PM", hour - 12),
        _ => format!("{}:00", hour),
    }
}

/// Truncate a beach name to fit in the grid
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        format!("{:<width$}", name, width = max_len)
    } else {
        format!("{:.width$}", name, width = max_len - 1)
    }
}

/// Renders the Plan Trip screen
///
/// # Arguments
/// * `frame` - The ratatui frame to render into
/// * `app` - The application state
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create main bordered block
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::HEADER))
        .title(Span::styled(
            " Plan Your Trip ",
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = main_block.inner(area);
    frame.render_widget(main_block, area);

    // Create layout:
    // - Activity selector row
    // - Heatmap grid
    // - Legend
    // - Best recommendation / Selected cell
    // - Help bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Activity selector
            Constraint::Min(8),    // Heatmap grid
            Constraint::Length(2), // Legend
            Constraint::Length(3), // Best recommendation + selected
            Constraint::Length(1), // Help bar
        ])
        .split(inner_area);

    // Render each section
    render_activity_selector(frame, chunks[0], app.current_activity);
    render_heatmap_grid(frame, chunks[1], app);
    render_legend(frame, chunks[2]);
    render_recommendations(frame, chunks[3], app);
    render_help_bar(frame, chunks[4]);
}

/// Renders the activity selector row
fn render_activity_selector(frame: &mut Frame, area: Rect, current_activity: Option<Activity>) {
    let activities = Activity::all();
    let mut spans = vec![Span::styled(
        "Activity: ",
        Style::default().fg(colors::SECONDARY),
    )];

    for (i, activity) in activities.iter().enumerate() {
        let is_selected = current_activity == Some(*activity);
        let indicator = if is_selected { "\u{25CF}" } else { "\u{25CB}" }; // Filled or empty circle
        let label = match activity {
            Activity::Swimming => "Swimming",
            Activity::Sunbathing => "Sunbathing",
            Activity::Sailing => "Sailing",
            Activity::Sunset => "Sunset",
            Activity::Peace => "Peace",
        };

        let style = if is_selected {
            Style::default()
                .fg(colors::SELECTED)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors::SECONDARY)
        };

        spans.push(Span::raw("["));
        spans.push(Span::styled(indicator, style));
        spans.push(Span::styled(label, style));
        spans.push(Span::raw("]"));

        if i < activities.len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(vec![line]);
    frame.render_widget(paragraph, area);
}

/// Renders the heatmap grid with beaches as rows and hours as columns
fn render_heatmap_grid(frame: &mut Frame, area: Rect, app: &App) {
    let beaches = all_beaches();
    let (start_hour, end_hour) = app.plan_time_range;
    let hours: Vec<u8> = (start_hour..=end_hour).collect();

    // Calculate column widths
    let beach_name_width = 12; // Truncate beach names to fit
    let cell_width = 5; // Width for each hour cell

    let mut lines: Vec<Line> = Vec::new();

    // Header row with hour labels
    let mut header_spans = vec![Span::raw(format!(
        "{:width$}",
        "",
        width = beach_name_width + 2
    ))];

    for hour in &hours {
        let hour_str = format!("{:^width$}", format_hour_short(*hour), width = cell_width);
        header_spans.push(Span::styled(
            hour_str,
            Style::default().fg(colors::SECONDARY),
        ));
    }
    lines.push(Line::from(header_spans));

    // Empty line after header
    lines.push(Line::from(Span::styled(
        format!("{:width$}", "", width = beach_name_width + 1),
        Style::default().fg(colors::SECONDARY),
    )));

    // Beach rows
    for (beach_idx, beach) in beaches.iter().enumerate() {
        let is_selected_beach = beach_idx == app.plan_cursor.0;
        let beach_name = truncate_name(beach.name, beach_name_width);

        let name_style = if is_selected_beach {
            Style::default()
                .fg(colors::HEADER)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors::PRIMARY)
        };

        let mut row_spans = vec![Span::styled(format!("{} ", beach_name), name_style)];

        for (hour_idx, hour) in hours.iter().enumerate() {
            let is_cursor = beach_idx == app.plan_cursor.0 && hour_idx == app.plan_cursor.1;
            let score = compute_score(app, beach.id, *hour);
            let (block_char, block_color) = score_to_block(score);

            let cell_content = if is_cursor {
                format!("[{}]", block_char)
            } else {
                format!(" {} ", block_char)
            };

            let cell_style = if is_cursor {
                Style::default()
                    .fg(block_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(block_color)
            };

            row_spans.push(Span::styled(cell_content, cell_style));
        }

        lines.push(Line::from(row_spans));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Renders the legend showing score ranges
fn render_legend(frame: &mut Frame, area: Rect) {
    let legend_line = Line::from(vec![
        Span::styled("Legend: ", Style::default().fg(colors::SECONDARY)),
        Span::styled(BLOCK_EXCELLENT, Style::default().fg(colors::EXCELLENT)),
        Span::styled(" Excellent (80+)  ", Style::default().fg(colors::SECONDARY)),
        Span::styled(BLOCK_GOOD, Style::default().fg(colors::GOOD)),
        Span::styled(" Good (60-79)  ", Style::default().fg(colors::SECONDARY)),
        Span::styled(BLOCK_FAIR, Style::default().fg(colors::FAIR)),
        Span::styled(" Fair (40-59)  ", Style::default().fg(colors::SECONDARY)),
        Span::styled(BLOCK_POOR, Style::default().fg(colors::POOR)),
        Span::styled(" Poor (<40)", Style::default().fg(colors::SECONDARY)),
    ]);

    let cursor_line = Line::from(vec![
        Span::styled("        ", Style::default()),
        Span::styled("[ ]", Style::default().fg(colors::PRIMARY)),
        Span::styled(" Cursor", Style::default().fg(colors::SECONDARY)),
    ]);

    let paragraph = Paragraph::new(vec![legend_line, cursor_line]);
    frame.render_widget(paragraph, area);
}

/// Renders the best recommendation and selected cell sections
fn render_recommendations(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    // Best recommendation
    if let Some((beach_name, _beach_id, hour, score)) = find_best_recommendation(app) {
        let time_str = format_hour_long(hour);
        lines.push(Line::from(vec![
            Span::styled(
                "BEST: ",
                Style::default()
                    .fg(colors::HEADER)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} @ {}  ", beach_name, time_str),
                Style::default().fg(colors::PRIMARY),
            ),
            Span::styled("Score: ", Style::default().fg(colors::SECONDARY)),
            Span::styled(
                format!("{}/100", score),
                Style::default()
                    .fg(colors::EXCELLENT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "Select an activity (1-5) to see recommendations",
            Style::default().fg(colors::SECONDARY),
        )));
    }

    // Selected cell info
    let beaches = all_beaches();
    let (start_hour, _end_hour) = app.plan_time_range;
    let hours: Vec<u8> = (start_hour..=_end_hour).collect();

    if let Some(beach) = beaches.get(app.plan_cursor.0) {
        if let Some(hour) = hours.get(app.plan_cursor.1) {
            let score = compute_score(app, beach.id, *hour);
            let time_str = format_hour_long(*hour);

            lines.push(Line::from(vec![
                Span::styled("SELECTED: ", Style::default().fg(colors::SECONDARY)),
                Span::styled(
                    format!("{} @ {}  ", beach.name, time_str),
                    Style::default().fg(colors::PRIMARY),
                ),
                Span::styled("Score: ", Style::default().fg(colors::SECONDARY)),
                Span::styled(
                    format!("{}/100", score),
                    Style::default().fg(score_to_block(score).1),
                ),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Renders the help bar at the bottom
fn render_help_bar(frame: &mut Frame, area: Rect) {
    let help_line = Line::from(vec![
        Span::styled("\u{2190}/h \u{2192}/l", Style::default().fg(colors::HEADER)),
        Span::styled(" Hours  ", Style::default().fg(colors::SECONDARY)),
        Span::styled("\u{2191}/k \u{2193}/j", Style::default().fg(colors::HEADER)),
        Span::styled(" Beaches  ", Style::default().fg(colors::SECONDARY)),
        Span::styled("1-5", Style::default().fg(colors::HEADER)),
        Span::styled(" Activity  ", Style::default().fg(colors::SECONDARY)),
        Span::styled("Enter", Style::default().fg(colors::HEADER)),
        Span::styled(" Go  ", Style::default().fg(colors::SECONDARY)),
        Span::styled("Esc", Style::default().fg(colors::HEADER)),
        Span::styled(" Back", Style::default().fg(colors::SECONDARY)),
    ]);

    let paragraph = Paragraph::new(vec![help_line]);
    frame.render_widget(paragraph, area);
}
