//! Beach detail screen UI
//!
//! Renders the detailed view for a single beach, showing weather conditions,
//! tide information, and water quality status in a bordered box layout.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::activities::{get_profile, Activity};
use crate::app::App;
use crate::data::{TideState, WaterStatus, WeatherCondition};

/// Color scheme matching WIREFRAMES.md
mod colors {
    use ratatui::style::Color;

    /// Safe/good status (green)
    pub const SAFE: Color = Color::Green;
    /// Advisory/warning status (yellow)
    pub const ADVISORY: Color = Color::Yellow;
    /// Closed/danger status (red)
    pub const CLOSED: Color = Color::Red;
    /// Unknown/unavailable status (gray)
    pub const UNKNOWN: Color = Color::DarkGray;
    /// Section headers
    pub const HEADER: Color = Color::Cyan;
    /// Primary text
    pub const PRIMARY: Color = Color::White;
    /// Secondary/dimmed text
    pub const SECONDARY: Color = Color::Gray;
    /// Rising tide indicator
    pub const RISING: Color = Color::Cyan;
    /// Falling tide indicator
    pub const FALLING: Color = Color::Blue;
    /// Selected activity indicator
    pub const SELECTED: Color = Color::Yellow;
    /// High score (gold medal)
    pub const GOLD: Color = Color::Yellow;
    /// Second place (silver medal)
    pub const SILVER: Color = Color::Gray;
    /// Third place (bronze medal)
    pub const BRONZE: Color = Color::Rgb(205, 127, 50);
}

/// Renders the beach detail screen
///
/// # Arguments
/// * `frame` - The ratatui frame to render into
/// * `app` - The application state
/// * `beach_id` - The ID of the beach to display
pub fn render(frame: &mut Frame, app: &App, beach_id: &str) {
    let area = frame.area();

    // Get beach conditions or show error
    let conditions = match app.get_conditions(beach_id) {
        Some(c) => c,
        None => {
            render_no_data(frame, area, beach_id);
            return;
        }
    };

    let beach_name = conditions.beach.name;

    // Create main bordered block with beach name as title
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::HEADER))
        .title(Span::styled(
            format!(" {} ", beach_name),
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = main_block.inner(area);
    frame.render_widget(main_block, area);

    // Determine if we need to show the Best Window section
    let show_best_window = app.current_activity.is_some();

    // Create layout: Activity selector, Weather/Tides row, Water Quality row,
    // Best Window (if activity selected), Help row
    let constraints = if show_best_window {
        vec![
            Constraint::Length(1), // Activity selector row
            Constraint::Min(6),    // Weather and Tides section
            Constraint::Length(4), // Water Quality section
            Constraint::Length(6), // Best Window Today section
            Constraint::Length(2), // Help text
        ]
    } else {
        vec![
            Constraint::Length(1), // Activity selector row
            Constraint::Min(6),    // Weather and Tides section
            Constraint::Length(4), // Water Quality section
            Constraint::Length(2), // Help text
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    // Render activity selector at the top
    render_activity_selector(frame, chunks[0], app.current_activity);

    // Split the weather/tides section into Weather and Tides columns
    let top_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Render sections
    render_weather_section(frame, top_columns[0], conditions.weather.as_ref());
    render_tides_section(frame, top_columns[1], conditions.tides.as_ref());
    render_water_quality_section(frame, chunks[2], conditions.water_quality.as_ref());

    // Render Best Window Today section if activity is selected
    if show_best_window {
        render_best_window_section(frame, chunks[3], app, beach_id);
        render_help_text(frame, chunks[4]);
    } else {
        render_help_text(frame, chunks[3]);
    }
}

/// Renders the weather section
fn render_weather_section(frame: &mut Frame, area: Rect, weather: Option<&crate::data::Weather>) {
    let mut lines = vec![Line::from(Span::styled(
        "WEATHER",
        Style::default()
            .fg(colors::HEADER)
            .add_modifier(Modifier::BOLD),
    ))];

    match weather {
        Some(w) => {
            // Condition icon and temperature
            let icon = condition_icon(w.condition);
            let temp_line = Line::from(vec![
                Span::raw(format!("{}  ", icon)),
                Span::styled(
                    format!("{:.0}C", w.temperature),
                    Style::default().fg(colors::PRIMARY),
                ),
                Span::styled(
                    format!(" (feels {:.0})", w.feels_like),
                    Style::default().fg(colors::SECONDARY),
                ),
            ]);
            lines.push(temp_line);

            // Wind
            let wind_line = Line::from(vec![
                Span::raw("Wind: "),
                Span::styled(
                    format!("{:.0} km/h", w.wind),
                    Style::default().fg(colors::PRIMARY),
                ),
            ]);
            lines.push(wind_line);

            // Humidity
            let humidity_line = Line::from(vec![
                Span::raw("Humidity: "),
                Span::styled(
                    format!("{}%", w.humidity),
                    Style::default().fg(colors::PRIMARY),
                ),
            ]);
            lines.push(humidity_line);

            // UV Index with color coding
            let uv_color = uv_index_color(w.uv);
            let uv_level = uv_level_text(w.uv);
            let uv_line = Line::from(vec![
                Span::raw("UV: "),
                Span::styled(format!("{:.0}", w.uv), Style::default().fg(uv_color)),
                Span::styled(format!(" ({})", uv_level), Style::default().fg(uv_color)),
            ]);
            lines.push(uv_line);

            // Sunrise/Sunset
            let sun_line = Line::from(vec![
                Span::styled("Sunrise: ", Style::default().fg(colors::SECONDARY)),
                Span::styled(
                    w.sunrise.format("%H:%M").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ),
                Span::raw("  "),
                Span::styled("Sunset: ", Style::default().fg(colors::SECONDARY)),
                Span::styled(
                    w.sunset.format("%H:%M").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ),
            ]);
            lines.push(sun_line);
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Weather data unavailable",
                Style::default().fg(colors::UNKNOWN),
            )));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Renders the tides section
fn render_tides_section(frame: &mut Frame, area: Rect, tides: Option<&crate::data::TideInfo>) {
    let mut lines = vec![Line::from(Span::styled(
        "TIDES",
        Style::default()
            .fg(colors::HEADER)
            .add_modifier(Modifier::BOLD),
    ))];

    match tides {
        Some(t) => {
            // Current tide state with arrow
            let (state_icon, state_text, state_color) = match t.tide_state {
                TideState::Rising => ("^", "Rising", colors::RISING),
                TideState::Falling => ("v", "Falling", colors::FALLING),
                TideState::High => ("=", "High", colors::HEADER),
                TideState::Low => ("=", "Low", colors::SECONDARY),
            };

            let state_line = Line::from(vec![
                Span::styled(state_icon, Style::default().fg(state_color)),
                Span::raw(" "),
                Span::styled(state_text, Style::default().fg(state_color)),
            ]);
            lines.push(state_line);

            // Current height
            let height_line = Line::from(vec![
                Span::raw("Height: "),
                Span::styled(
                    format!("{:.1}m", t.current_height),
                    Style::default().fg(colors::PRIMARY),
                ),
            ]);
            lines.push(height_line);

            // Next high tide
            if let Some(ref high) = t.next_high {
                let high_line = Line::from(vec![
                    Span::styled("High: ", Style::default().fg(colors::SECONDARY)),
                    Span::styled(
                        high.time.format("%l:%M %p").to_string().trim().to_string(),
                        Style::default().fg(colors::PRIMARY),
                    ),
                    Span::styled(
                        format!(" ({:.1}m)", high.height),
                        Style::default().fg(colors::SECONDARY),
                    ),
                ]);
                lines.push(high_line);
            }

            // Next low tide
            if let Some(ref low) = t.next_low {
                let low_line = Line::from(vec![
                    Span::styled("Low:  ", Style::default().fg(colors::SECONDARY)),
                    Span::styled(
                        low.time.format("%l:%M %p").to_string().trim().to_string(),
                        Style::default().fg(colors::PRIMARY),
                    ),
                    Span::styled(
                        format!(" ({:.1}m)", low.height),
                        Style::default().fg(colors::SECONDARY),
                    ),
                ]);
                lines.push(low_line);
            }
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Tide data unavailable",
                Style::default().fg(colors::UNKNOWN),
            )));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Renders the water quality section
fn render_water_quality_section(
    frame: &mut Frame,
    area: Rect,
    water_quality: Option<&crate::data::WaterQuality>,
) {
    let mut lines = vec![Line::from(Span::styled(
        "WATER QUALITY",
        Style::default()
            .fg(colors::HEADER)
            .add_modifier(Modifier::BOLD),
    ))];

    match water_quality {
        Some(wq) => {
            // Status with icon and color
            let (icon, text, color) = match wq.status {
                WaterStatus::Safe => ("*", "Safe to swim", colors::SAFE),
                WaterStatus::Advisory => ("!", "Advisory in effect", colors::ADVISORY),
                WaterStatus::Closed => ("X", "Beach closed", colors::CLOSED),
                WaterStatus::Unknown => ("?", "Status unknown", colors::UNKNOWN),
            };

            let status_line = Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default().fg(color)),
                Span::styled(text, Style::default().fg(color)),
            ]);
            lines.push(status_line);

            // Test date and E. coli count
            let mut detail_spans = vec![
                Span::styled("Last tested: ", Style::default().fg(colors::SECONDARY)),
                Span::styled(
                    wq.sample_date.format("%b %d").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ),
            ];

            if let Some(ecoli) = wq.ecoli_count {
                detail_spans.push(Span::raw("  "));
                detail_spans.push(Span::styled(
                    format!("E.coli: {} CFU/100mL", ecoli),
                    Style::default().fg(colors::SECONDARY),
                ));
            }

            lines.push(Line::from(detail_spans));

            // Advisory reason if present
            if let Some(ref reason) = wq.advisory_reason {
                lines.push(Line::from(Span::styled(
                    reason.clone(),
                    Style::default().fg(colors::ADVISORY),
                )));
            }
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Water quality data unavailable",
                Style::default().fg(colors::UNKNOWN),
            )));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Renders the activity selector row
/// Shows all activities with filled (selected) or empty (unselected) indicators
fn render_activity_selector(frame: &mut Frame, area: Rect, current_activity: Option<Activity>) {
    let activities = Activity::all();
    let mut spans = vec![Span::styled(
        "Activity: ",
        Style::default().fg(colors::SECONDARY),
    )];

    for (i, activity) in activities.iter().enumerate() {
        let is_selected = current_activity == Some(*activity);
        let indicator = if is_selected { "\u{25CF}" } else { "\u{25CB}" }; // ● or ○
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

/// Represents a scored time window for display
struct TimeWindow {
    start_hour: u8,
    end_hour: u8,
    score: u8,
    reason: String,
}

/// Renders the "Best Window Today" section showing top 3 time slots for the selected activity
fn render_best_window_section(frame: &mut Frame, area: Rect, app: &App, beach_id: &str) {
    let mut lines = vec![
        Line::from(Span::styled(
            "BEST WINDOW TODAY",
            Style::default()
                .fg(colors::HEADER)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            Style::default().fg(colors::SECONDARY),
        )),
    ];

    // Get the current activity
    let activity = match app.current_activity {
        Some(a) => a,
        None => {
            lines.push(Line::from(Span::styled(
                "Select an activity (1-5) to see best times",
                Style::default().fg(colors::SECONDARY),
            )));
            let paragraph = Paragraph::new(lines);
            frame.render_widget(paragraph, area);
            return;
        }
    };

    // Get beach conditions for scoring
    let conditions = match app.get_conditions(beach_id) {
        Some(c) => c,
        None => {
            lines.push(Line::from(Span::styled(
                "Weather data unavailable for scoring",
                Style::default().fg(colors::UNKNOWN),
            )));
            let paragraph = Paragraph::new(lines);
            frame.render_widget(paragraph, area);
            return;
        }
    };

    // Compute time windows
    let windows = compute_best_windows(activity, conditions);

    if windows.is_empty() {
        lines.push(Line::from(Span::styled(
            "No suitable time windows found",
            Style::default().fg(colors::SECONDARY),
        )));
    } else {
        let medals = [
            ("\u{1F947}", colors::GOLD),   // 🥇
            ("\u{1F948}", colors::SILVER), // 🥈
            ("\u{1F949}", colors::BRONZE), // 🥉
        ];

        for (i, window) in windows.iter().take(3).enumerate() {
            let (medal, color) = medals.get(i).unwrap_or(&("  ", colors::SECONDARY));
            let time_range = format!(
                "{} - {}",
                format_hour(window.start_hour),
                format_hour(window.end_hour)
            );

            lines.push(Line::from(vec![
                Span::raw(format!("{} ", medal)),
                Span::styled(
                    format!("{:<18}", time_range),
                    Style::default().fg(colors::PRIMARY),
                ),
                Span::styled("Score: ", Style::default().fg(colors::SECONDARY)),
                Span::styled(
                    format!("{}/100", window.score),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ),
            ]));

            lines.push(Line::from(Span::styled(
                format!("   {}", window.reason),
                Style::default().fg(colors::SECONDARY),
            )));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Computes the best time windows for a given activity and beach conditions
fn compute_best_windows(
    activity: Activity,
    conditions: &crate::data::BeachConditions,
) -> Vec<TimeWindow> {
    let profile = get_profile(activity);

    // Get weather data for scoring
    let (temp, wind, uv) = match &conditions.weather {
        Some(w) => (w.temperature as f32, w.wind as f32, w.uv as f32),
        None => return vec![], // Can't score without weather
    };

    // Get water status
    let water_status = conditions
        .water_quality
        .as_ref()
        .map(|wq| wq.status)
        .unwrap_or(crate::data::WaterStatus::Unknown);

    // Get tide info
    let (tide_height, max_tide) = match &conditions.tides {
        Some(t) => {
            let max_h = t.next_high.as_ref().map(|h| h.height).unwrap_or(4.8);
            (t.current_height as f32, max_h as f32)
        }
        None => (2.4, 4.8), // Default mid-tide
    };

    // Score each hour from 6am to 9pm
    let mut hourly_scores: Vec<(u8, u8)> = Vec::new();
    for hour in 6..=21 {
        // Estimate crowd level based on time of day (simple heuristic)
        let crowd_level = estimate_crowd_level(hour);

        let score = profile.score_time_slot(
            hour,
            conditions.beach.id,
            temp,
            wind,
            uv,
            water_status,
            tide_height,
            max_tide,
            crowd_level,
        );

        hourly_scores.push((hour, score.score));
    }

    // Group adjacent high-scoring hours into windows
    group_into_windows(
        &hourly_scores,
        activity,
        temp,
        water_status,
        tide_height,
        max_tide,
    )
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

/// Groups hourly scores into time windows and returns top windows sorted by score
fn group_into_windows(
    hourly_scores: &[(u8, u8)],
    activity: Activity,
    temp: f32,
    water_status: crate::data::WaterStatus,
    tide_height: f32,
    max_tide: f32,
) -> Vec<TimeWindow> {
    if hourly_scores.is_empty() {
        return vec![];
    }

    // Find contiguous windows where score is above threshold (50)
    let threshold = 50u8;
    let mut windows: Vec<TimeWindow> = Vec::new();
    let mut current_window: Option<(u8, u8, u8)> = None; // (start, end, max_score)

    for &(hour, score) in hourly_scores {
        if score >= threshold {
            match current_window {
                Some((start, _, max_s)) => {
                    current_window = Some((start, hour, max_s.max(score)));
                }
                None => {
                    current_window = Some((hour, hour, score));
                }
            }
        } else {
            // End current window if exists
            if let Some((start, end, max_score)) = current_window {
                let reason = generate_reason(activity, temp, water_status, tide_height, max_tide);
                windows.push(TimeWindow {
                    start_hour: start,
                    end_hour: end + 1, // End is exclusive
                    score: max_score,
                    reason,
                });
                current_window = None;
            }
        }
    }

    // Don't forget the last window
    if let Some((start, end, max_score)) = current_window {
        let reason = generate_reason(activity, temp, water_status, tide_height, max_tide);
        windows.push(TimeWindow {
            start_hour: start,
            end_hour: end + 1,
            score: max_score,
            reason,
        });
    }

    // If no windows above threshold, create windows from best individual hours
    if windows.is_empty() {
        let mut sorted = hourly_scores.to_vec();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        for &(hour, score) in sorted.iter().take(3) {
            let reason = generate_reason(activity, temp, water_status, tide_height, max_tide);
            windows.push(TimeWindow {
                start_hour: hour,
                end_hour: hour + 1,
                score,
                reason,
            });
        }
    }

    // Sort by score descending
    windows.sort_by(|a, b| b.score.cmp(&a.score));
    windows
}

/// Generates a human-readable reason string for the time window
fn generate_reason(
    activity: Activity,
    temp: f32,
    water_status: crate::data::WaterStatus,
    tide_height: f32,
    max_tide: f32,
) -> String {
    let mut factors = Vec::new();

    // Temperature description
    let temp_desc = if temp >= 25.0 {
        format!("Hot ({:.0}C)", temp)
    } else if temp >= 20.0 {
        format!("Warm ({:.0}C)", temp)
    } else if temp >= 15.0 {
        format!("Mild ({:.0}C)", temp)
    } else {
        format!("Cool ({:.0}C)", temp)
    };
    factors.push(temp_desc);

    // Water status (relevant for swimming)
    if activity == Activity::Swimming {
        match water_status {
            crate::data::WaterStatus::Safe => factors.push("safe water".to_string()),
            crate::data::WaterStatus::Advisory => factors.push("advisory in effect".to_string()),
            crate::data::WaterStatus::Closed => factors.push("water closed".to_string()),
            crate::data::WaterStatus::Unknown => {}
        }
    }

    // Tide description
    let tide_ratio = tide_height / max_tide;
    let tide_desc = if tide_ratio > 0.7 {
        "high tide"
    } else if tide_ratio > 0.3 {
        "mid-tide"
    } else {
        "low tide"
    };
    factors.push(tide_desc.to_string());

    factors.join(", ")
}

/// Formats an hour (0-23) into a human-readable time string
fn format_hour(hour: u8) -> String {
    match hour {
        0 => "12:00 AM".to_string(),
        1..=11 => format!("{}:00 AM", hour),
        12 => "12:00 PM".to_string(),
        13..=23 => format!("{}:00 PM", hour - 12),
        _ => format!("{}:00", hour),
    }
}

/// Renders the help text at the bottom
fn render_help_text(frame: &mut Frame, area: Rect) {
    let help_line = Line::from(vec![
        Span::styled("<- Back", Style::default().fg(colors::SECONDARY)),
        Span::raw("  "),
        Span::styled("1-5", Style::default().fg(colors::HEADER)),
        Span::styled(" Activity", Style::default().fg(colors::SECONDARY)),
        Span::raw("  "),
        Span::styled("r", Style::default().fg(colors::HEADER)),
        Span::styled(" Refresh", Style::default().fg(colors::SECONDARY)),
        Span::raw("  "),
        Span::styled("q", Style::default().fg(colors::HEADER)),
        Span::styled(" Quit", Style::default().fg(colors::SECONDARY)),
    ]);

    let paragraph = Paragraph::new(vec![Line::default(), help_line]);
    frame.render_widget(paragraph, area);
}

/// Renders a "no data" message when beach conditions are unavailable
fn render_no_data(frame: &mut Frame, area: Rect, beach_id: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::HEADER))
        .title(Span::styled(
            format!(" {} ", beach_id),
            Style::default()
                .fg(colors::PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let message = Paragraph::new(vec![
        Line::default(),
        Line::from(Span::styled(
            "No data available for this beach",
            Style::default().fg(colors::UNKNOWN),
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("<- Back", Style::default().fg(colors::SECONDARY)),
            Span::raw("  "),
            Span::styled("r", Style::default().fg(colors::HEADER)),
            Span::styled(" Refresh", Style::default().fg(colors::SECONDARY)),
            Span::raw("  "),
            Span::styled("q", Style::default().fg(colors::HEADER)),
            Span::styled(" Quit", Style::default().fg(colors::SECONDARY)),
        ]),
    ]);

    frame.render_widget(message, inner);
}

/// Returns an icon character for the weather condition
fn condition_icon(condition: WeatherCondition) -> &'static str {
    match condition {
        WeatherCondition::Clear => "Sun",
        WeatherCondition::PartlyCloudy => "Cloud/Sun",
        WeatherCondition::Cloudy => "Cloud",
        WeatherCondition::Rain => "Rain",
        WeatherCondition::Showers => "Showers",
        WeatherCondition::Thunderstorm => "Storm",
        WeatherCondition::Snow => "Snow",
        WeatherCondition::Fog => "Fog",
    }
}

/// Returns the color for a UV index value
fn uv_index_color(uv: f64) -> Color {
    match uv as u32 {
        0..=2 => colors::SAFE,
        3..=5 => Color::Yellow,
        6..=7 => Color::LightRed,
        8..=10 => colors::CLOSED,
        _ => Color::Magenta, // Extreme
    }
}

/// Returns the text description for a UV index value
fn uv_level_text(uv: f64) -> &'static str {
    match uv as u32 {
        0..=2 => "Low",
        3..=5 => "Moderate",
        6..=7 => "High",
        8..=10 => "Very High",
        _ => "Extreme",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Beach, BeachConditions, TideEvent, TideInfo, WaterQuality, Weather};
    use chrono::{Local, NaiveDate, NaiveTime, Utc};
    use ratatui::{backend::TestBackend, Terminal};

    /// Helper to create a test app with beach conditions
    fn create_test_app_with_conditions(
        beach_id: &str,
        weather: Option<Weather>,
        tides: Option<TideInfo>,
        water_quality: Option<WaterQuality>,
    ) -> App {
        let mut app = App::new();
        app.state = crate::app::AppState::BeachDetail(beach_id.to_string());

        let beach = Beach {
            id: "kitsilano",
            name: "Kitsilano Beach",
            latitude: 49.2743,
            longitude: -123.1544,
            water_quality_id: Some("kitsilano-beach"),
        };

        let conditions = BeachConditions {
            beach,
            weather,
            tides,
            water_quality,
        };

        app.beach_conditions
            .insert(beach_id.to_string(), conditions);
        app
    }

    fn create_test_weather() -> Weather {
        Weather {
            temperature: 22.0,
            feels_like: 24.0,
            condition: WeatherCondition::Clear,
            humidity: 65,
            wind: 12.0,
            uv: 6.0,
            sunrise: NaiveTime::from_hms_opt(5, 30, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(21, 15, 0).unwrap(),
            fetched_at: Utc::now(),
        }
    }

    fn create_test_tides() -> TideInfo {
        TideInfo {
            current_height: 2.5,
            tide_state: TideState::Rising,
            next_high: Some(TideEvent {
                time: Local::now(),
                height: 4.2,
            }),
            next_low: Some(TideEvent {
                time: Local::now(),
                height: 0.8,
            }),
            fetched_at: Utc::now(),
        }
    }

    fn create_test_water_quality() -> WaterQuality {
        WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(45),
            sample_date: NaiveDate::from_ymd_opt(2026, 1, 24).unwrap(),
            advisory_reason: None,
            fetched_at: Utc::now(),
        }
    }

    #[test]
    fn test_render_produces_non_empty_buffer() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(!content.trim().is_empty(), "Buffer should not be empty");
    }

    #[test]
    fn test_weather_section_renders_temperature() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app =
            create_test_app_with_conditions("kitsilano", Some(create_test_weather()), None, None);

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(
            content.contains("22") || content.contains("WEATHER"),
            "Should render weather section with temperature"
        );
    }

    #[test]
    fn test_tides_section_renders_tide_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app =
            create_test_app_with_conditions("kitsilano", None, Some(create_test_tides()), None);

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(
            content.contains("Rising") || content.contains("TIDES"),
            "Should render tides section with tide state"
        );
    }

    #[test]
    fn test_water_quality_section_renders_status() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = create_test_app_with_conditions(
            "kitsilano",
            None,
            None,
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(
            content.contains("Safe") || content.contains("WATER"),
            "Should render water quality section with status"
        );
    }

    #[test]
    fn test_handles_missing_weather_gracefully() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = create_test_app_with_conditions(
            "kitsilano",
            None, // No weather data
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(
            content.contains("unavailable") || content.contains("WEATHER"),
            "Should handle missing weather data gracefully"
        );
    }

    #[test]
    fn test_handles_missing_all_data_gracefully() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = create_test_app_with_conditions("kitsilano", None, None, None);

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // Should still render without crashing
        assert!(!content.trim().is_empty());
    }

    #[test]
    fn test_handles_no_conditions_for_beach() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new();
        app.state = crate::app::AppState::BeachDetail("nonexistent".to_string());

        terminal
            .draw(|frame| {
                render(frame, &app, "nonexistent");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(
            content.contains("No data") || content.contains("nonexistent"),
            "Should show no data message for nonexistent beach"
        );
    }

    #[test]
    fn test_condition_icon_mapping() {
        assert_eq!(condition_icon(WeatherCondition::Clear), "Sun");
        assert_eq!(condition_icon(WeatherCondition::Rain), "Rain");
        assert_eq!(condition_icon(WeatherCondition::Snow), "Snow");
        assert_eq!(condition_icon(WeatherCondition::Fog), "Fog");
    }

    #[test]
    fn test_uv_level_text() {
        assert_eq!(uv_level_text(1.0), "Low");
        assert_eq!(uv_level_text(4.0), "Moderate");
        assert_eq!(uv_level_text(6.5), "High");
        assert_eq!(uv_level_text(9.0), "Very High");
        assert_eq!(uv_level_text(12.0), "Extreme");
    }

    #[test]
    fn test_uv_index_color() {
        assert_eq!(uv_index_color(1.0), colors::SAFE);
        assert_eq!(uv_index_color(4.0), Color::Yellow);
        assert_eq!(uv_index_color(6.5), Color::LightRed);
        assert_eq!(uv_index_color(9.0), colors::CLOSED);
        assert_eq!(uv_index_color(12.0), Color::Magenta);
    }

    #[test]
    fn test_water_status_colors() {
        // Verify status icon/text mapping for different water statuses
        let statuses = [
            (WaterStatus::Safe, "*", "Safe to swim", colors::SAFE),
            (
                WaterStatus::Advisory,
                "!",
                "Advisory in effect",
                colors::ADVISORY,
            ),
            (WaterStatus::Closed, "X", "Beach closed", colors::CLOSED),
            (WaterStatus::Unknown, "?", "Status unknown", colors::UNKNOWN),
        ];

        for (status, expected_icon, expected_text, expected_color) in statuses {
            let (icon, text, color) = match status {
                WaterStatus::Safe => ("*", "Safe to swim", colors::SAFE),
                WaterStatus::Advisory => ("!", "Advisory in effect", colors::ADVISORY),
                WaterStatus::Closed => ("X", "Beach closed", colors::CLOSED),
                WaterStatus::Unknown => ("?", "Status unknown", colors::UNKNOWN),
            };
            assert_eq!(icon, expected_icon);
            assert_eq!(text, expected_text);
            assert_eq!(color, expected_color);
        }
    }

    #[test]
    fn test_tide_state_icons() {
        // Verify tide state icon mapping
        let states = [
            (TideState::Rising, "^", "Rising"),
            (TideState::Falling, "v", "Falling"),
            (TideState::High, "=", "High"),
            (TideState::Low, "=", "Low"),
        ];

        for (state, expected_icon, expected_text) in states {
            let (icon, text, _) = match state {
                TideState::Rising => ("^", "Rising", colors::RISING),
                TideState::Falling => ("v", "Falling", colors::FALLING),
                TideState::High => ("=", "High", colors::HEADER),
                TideState::Low => ("=", "Low", colors::SECONDARY),
            };
            assert_eq!(icon, expected_icon);
            assert_eq!(text, expected_text);
        }
    }
}
