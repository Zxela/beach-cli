//! Beach list screen rendering
//!
//! Renders the main beach list view showing all Vancouver beaches with their
//! current conditions including temperature, weather, and water quality status.

use chrono::{Datelike, Local, Timelike};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::activities::{get_profile, sunset_time_scorer_dynamic, Activity};
use crate::app::App;
use crate::data::{all_beaches, BeachConditions, WaterStatus, WeatherCondition};

/// Weather condition to icon mapping
fn weather_icon(condition: &WeatherCondition) -> &'static str {
    match condition {
        WeatherCondition::Clear => "\u{2600}",        // ☀
        WeatherCondition::PartlyCloudy => "\u{26C5}", // ⛅
        WeatherCondition::Cloudy => "\u{2601}",       // ☁
        WeatherCondition::Rain => "\u{1F327}",        // 🌧
        WeatherCondition::Showers => "\u{1F326}",     // 🌦
        WeatherCondition::Thunderstorm => "\u{26C8}", // ⛈
        WeatherCondition::Snow => "\u{2744}",         // ❄
        WeatherCondition::Fog => "\u{1F32B}",         // 🌫
    }
}

/// Water status to icon mapping
fn water_status_icon(status: &WaterStatus) -> &'static str {
    match status {
        WaterStatus::Safe => "\u{1F7E2}",     // 🟢
        WaterStatus::Advisory => "\u{1F7E1}", // 🟡
        WaterStatus::Closed => "\u{1F534}",   // 🔴
        WaterStatus::Unknown => "\u{26AA}",   // ⚪
    }
}

/// Color for water status
fn water_status_color(status: &WaterStatus) -> Color {
    match status {
        WaterStatus::Safe => Color::Green,
        WaterStatus::Advisory => Color::Yellow,
        WaterStatus::Closed => Color::Red,
        WaterStatus::Unknown => Color::Gray,
    }
}

/// Color for temperature (warmer = more red, cooler = more blue)
fn temperature_color(temp: f64) -> Color {
    if temp >= 30.0 {
        Color::Red
    } else if temp >= 25.0 {
        Color::LightRed
    } else if temp >= 20.0 {
        Color::Yellow
    } else if temp >= 15.0 {
        Color::Green
    } else if temp >= 10.0 {
        Color::Cyan
    } else {
        Color::Blue
    }
}

/// Block characters for tide height visualization (8 levels)
const TIDE_BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Converts a tide height to a block character
fn height_to_block(height: f64, max_height: f64) -> char {
    let normalized = (height / max_height).clamp(0.0, 1.0);
    let index = ((normalized * 7.0).round() as usize).min(7);
    TIDE_BLOCKS[index]
}

/// Generates a sparkline string for tide heights
fn generate_tide_sparkline(
    heights: &[f64],
    max_height: f64,
    current_hour_index: Option<usize>,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    for (i, height) in heights.iter().enumerate() {
        let block = height_to_block(*height, max_height);
        let style = if current_hour_index == Some(i) {
            Style::default().fg(Color::Yellow) // Highlight current hour
        } else {
            Style::default().fg(Color::Cyan)
        };
        spans.push(Span::styled(block.to_string(), style));
    }

    spans
}

/// Generates a contextual hint for a beach based on current conditions.
///
/// Hints are prioritized in the following order:
/// 1. Water quality issue -> "Water advisory"
/// 2. Within 2h of sunset -> "Sunset in Xh Ym"
/// 3. High wind (>15 km/h) -> "Windy - good sailing"
/// 4. Early morning (6-9am) -> "Good for peace" or "Warming up"
/// 5. Peak hours (12-4pm) + weekend -> "Crowded now"
/// 6. Peak hours + good weather -> "Peak swimming" or "Peak sun hours"
/// 7. Default based on temp/conditions
fn generate_contextual_hint(conditions: Option<&BeachConditions>) -> Option<String> {
    let conditions = conditions?;
    let now = Local::now();
    let current_hour = now.hour() as u8;
    let is_weekend = matches!(now.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun);

    // Priority 1: Water quality issue
    if let Some(ref wq) = conditions.water_quality {
        if wq.status == WaterStatus::Advisory {
            return Some("Water advisory".to_string());
        }
        if wq.status == WaterStatus::Closed {
            return Some("Beach closed".to_string());
        }
    }

    // Get weather data for remaining checks
    let weather = conditions.weather.as_ref();

    // Priority 2: Within 2h of sunset
    if let Some(w) = weather {
        let current_time = now.time();
        let sunset_time = w.sunset;

        // Calculate minutes until sunset
        let current_minutes = current_time.hour() * 60 + current_time.minute();
        let sunset_minutes = sunset_time.hour() * 60 + sunset_time.minute();

        if current_minutes < sunset_minutes {
            let minutes_until_sunset = sunset_minutes - current_minutes;
            if minutes_until_sunset <= 120 {
                let hours = minutes_until_sunset / 60;
                let mins = minutes_until_sunset % 60;
                if hours > 0 {
                    return Some(format!("Sunset in {}h {}m", hours, mins));
                } else {
                    return Some(format!("Sunset in {}m", mins));
                }
            }
        }
    }

    // Priority 3: High wind (>15 km/h)
    if let Some(w) = weather {
        if w.wind > 15.0 {
            return Some("Windy - good sailing".to_string());
        }
    }

    // Priority 4: Early morning (6-9am)
    if (6..9).contains(&current_hour) {
        if let Some(w) = weather {
            if w.temperature < 18.0 {
                return Some("Warming up".to_string());
            }
        }
        return Some("Good for peace".to_string());
    }

    // Priority 5 & 6: Peak hours (12-4pm)
    if (12..16).contains(&current_hour) {
        // Priority 5: Weekend crowds
        if is_weekend {
            return Some("Crowded now".to_string());
        }

        // Priority 6: Good weather during peak hours
        if let Some(w) = weather {
            let is_good_weather = matches!(
                w.condition,
                WeatherCondition::Clear | WeatherCondition::PartlyCloudy
            );

            if is_good_weather && w.temperature >= 20.0 {
                if w.condition == WeatherCondition::Clear {
                    return Some("Peak sun hours".to_string());
                }
                return Some("Peak swimming".to_string());
            }
        }
    }

    // Priority 7: Default based on conditions
    if let Some(w) = weather {
        // Evening hints
        if (17..21).contains(&current_hour) {
            return Some("Evening stroll".to_string());
        }

        // Temperature-based defaults
        if w.temperature >= 25.0 && matches!(w.condition, WeatherCondition::Clear) {
            return Some("Great beach day".to_string());
        }

        if w.temperature >= 20.0 {
            return Some("Good for swimming".to_string());
        }

        if w.temperature < 15.0 {
            return Some("Brisk walk weather".to_string());
        }
    }

    // No specific hint
    None
}

/// Computes the best time today for a given beach and activity.
/// Returns (hour, score) or None if no data available.
fn compute_best_time_for_beach(
    conditions: Option<&BeachConditions>,
    activity: Activity,
) -> Option<(u8, u8)> {
    let conditions = conditions?;
    let weather = conditions.weather.as_ref()?;
    let profile = get_profile(activity);

    let temp = weather.temperature as f32;
    let wind = weather.wind as f32;
    let uv = weather.uv as f32;
    let sunset_hour = weather.sunset.hour() as u8;

    let water_status = conditions
        .water_quality
        .as_ref()
        .map(|wq| wq.effective_status())
        .unwrap_or(WaterStatus::Unknown);

    let (tide_height, max_tide) = conditions
        .tides
        .as_ref()
        .map(|t| (t.current_height as f32, 4.8f32))
        .unwrap_or((2.4, 4.8));

    let current_hour = Local::now().hour() as u8;
    let start_hour = current_hour.max(6);

    // For sunset, cap at sunset hour
    let end_hour = if activity == Activity::Sunset {
        sunset_hour
    } else {
        21
    };

    if start_hour > end_hour {
        return None;
    }

    let mut best_hour = start_hour;
    let mut best_score: u8 = 0;

    for hour in start_hour..=end_hour {
        // Estimate crowd level
        let crowd = match hour {
            6..=7 => 0.1,
            8..=9 => 0.2,
            10..=11 => 0.4,
            12..=14 => 0.8,
            15..=17 => 0.6,
            18..=19 => 0.4,
            20..=21 => 0.2,
            _ => 0.5,
        };

        let mut score_result = profile.score_time_slot(
            hour,
            conditions.beach.id,
            temp,
            wind,
            uv,
            water_status,
            tide_height,
            max_tide,
            crowd,
        );

        // Apply dynamic sunset scoring
        if activity == Activity::Sunset {
            let time_score = sunset_time_scorer_dynamic(hour, sunset_hour);
            let adjusted = score_result.score as f32 * (0.3 + 0.7 * time_score);
            score_result.score = adjusted.clamp(0.0, 100.0) as u8;
        }

        if score_result.score > best_score {
            best_score = score_result.score;
            best_hour = hour;
        }
    }

    if best_score > 0 {
        Some((best_hour, best_score))
    } else {
        None
    }
}

/// Formats an hour as a time string (e.g., "15:00")
fn format_hour_short(hour: u8) -> String {
    format!("{:02}:00", hour)
}

/// Renders the beach list screen
///
/// Displays all Vancouver beaches in a bordered list with:
/// - Smart header with time, weather, and best recommendation
/// - Beach name
/// - Current temperature
/// - Weather condition icon
/// - Water quality status icon
///
/// The currently selected beach is highlighted with a cursor indicator
/// and different colors.
///
/// # Arguments
/// * `frame` - The ratatui Frame to render to
/// * `app` - The application state containing beach data and selection
pub fn render_beach_list(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create main layout with header, content area, and help text at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Smart header
            Constraint::Min(3),    // Beach list
            Constraint::Length(1), // Help text
        ])
        .split(area);

    // Render smart header
    render_smart_header(frame, app, chunks[0]);

    // Render the beach list
    render_list(frame, app, chunks[1]);

    // Render help text with data freshness
    render_help(frame, chunks[2], app);
}

/// Renders the smart header with time, weather, recommendation, and sunset info
fn render_smart_header(frame: &mut Frame, app: &App, area: Rect) {
    let now = Local::now();
    let time_str = now.format("%a %b %d, %H:%M").to_string();

    // Get current weather from first beach with data
    let current_temp = app
        .beach_conditions
        .values()
        .find_map(|c| c.weather.as_ref())
        .map(|w| format!("{:.0}°C {}", w.temperature, weather_icon(&w.condition)))
        .unwrap_or_else(|| "--°C".to_string());

    // Get sunset info
    let sunset_info = app
        .beach_conditions
        .values()
        .find_map(|c| c.weather.as_ref())
        .map(|w| {
            let now_time = now.time();
            let sunset = w.sunset;
            let mins_until = (sunset.hour() as i32 * 60 + sunset.minute() as i32)
                - (now_time.hour() as i32 * 60 + now_time.minute() as i32);
            if mins_until > 0 {
                let hours = mins_until / 60;
                let mins = mins_until % 60;
                if hours > 0 {
                    format!("Sunset in {}h {}m", hours, mins)
                } else {
                    format!("Sunset in {}m", mins)
                }
            } else {
                "Sunset passed".to_string()
            }
        })
        .unwrap_or_default();

    // Build header lines
    let width = area.width as usize;
    let separator = "─".repeat(width.saturating_sub(2));

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "VANBEACH",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(time_str, Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled(current_temp, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(Span::styled(
            separator,
            Style::default().fg(Color::DarkGray),
        )),
    ];

    // Best beach recommendation
    if let Some(best) = app.find_best_beach_now() {
        lines.push(Line::from(vec![
            Span::styled("★ ", Style::default().fg(Color::Yellow)),
            Span::styled("Best now: ", Style::default().fg(Color::White)),
            Span::styled(
                best.beach_name.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" — {}", best.reasons.join(", ")),
                Style::default().fg(Color::Gray),
            ),
        ]));
    } else if app.current_activity.is_some() {
        lines.push(Line::from(Span::styled(
            "No great options right now — check back later",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Press 1-5 to select an activity for recommendations",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Sunset info
    if !sunset_info.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", sunset_info),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Renders the beach list content
fn render_list(frame: &mut Frame, app: &App, area: Rect) {
    let beaches = all_beaches();
    let mut lines: Vec<Line> = Vec::with_capacity(beaches.len());

    // Calculate current hour index for sparkline highlighting (6am = 0, 7am = 1, etc.)
    let current_hour = Local::now().hour() as usize;
    let sparkline_pos = if (6..=21).contains(&current_hour) {
        Some(current_hour - 6)
    } else {
        None
    };

    for (index, beach) in beaches.iter().enumerate() {
        let is_selected = index == app.selected_index;

        // Get conditions for this beach
        let conditions = app.get_conditions(beach.id);

        // Build the line content
        let cursor = if is_selected { "\u{25B8} " } else { "  " }; // ▸ or space

        // Get temperature string and color
        let (temp_str, temp_color) = match conditions.and_then(|c| c.weather.as_ref()) {
            Some(weather) => {
                let temp = weather.temperature.round() as i32;
                (
                    format!("{:>3}\u{00B0}C", temp),
                    temperature_color(weather.temperature),
                )
            }
            None => ("--\u{00B0}C".to_string(), Color::Gray),
        };

        // Get weather icon
        let weather_icon_str = match conditions.and_then(|c| c.weather.as_ref()) {
            Some(weather) => weather_icon(&weather.condition),
            None => "?",
        };

        // Get water status icon and color
        let (water_icon_str, water_color) = match conditions.and_then(|c| c.water_quality.as_ref())
        {
            Some(wq) => (
                water_status_icon(&wq.status),
                water_status_color(&wq.status),
            ),
            None => ("\u{26AA}", Color::Gray), // ⚪
        };

        // Generate tide sparkline
        let tide_sparkline_spans = match conditions.and_then(|c| c.tides.as_ref()) {
            Some(tides) => {
                let heights = tides.hourly_heights(4.8);
                generate_tide_sparkline(&heights, 4.8, sparkline_pos)
            }
            None => vec![Span::styled(
                "────────────────",
                Style::default().fg(Color::DarkGray),
            )],
        };

        // Generate contextual hint
        let hint = generate_contextual_hint(conditions);

        // Build the line with spans
        let name_style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let cursor_style = if is_selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        // Format: " ▸ Beach Name              22°C ☀ 🟢 ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁  Hint"
        // Pad beach name to fixed width for alignment
        let name_padded = format!("{:<18}", beach.name);

        let mut spans = vec![
            Span::styled(cursor, cursor_style),
            Span::styled(name_padded, name_style),
            Span::raw(" "),
            Span::styled(temp_str, Style::default().fg(temp_color)),
            Span::raw(" "),
            Span::raw(weather_icon_str),
            Span::raw(" "),
            Span::styled(water_icon_str, Style::default().fg(water_color)),
            Span::raw(" "),
        ];

        // Add tide sparkline spans
        spans.extend(tide_sparkline_spans);

        // Add best time column if an activity is selected
        if let Some(activity) = app.current_activity {
            spans.push(Span::raw(" "));
            if let Some((best_hour, score)) = compute_best_time_for_beach(conditions, activity) {
                let score_color = if score >= 80 {
                    Color::Green
                } else if score >= 60 {
                    Color::Yellow
                } else {
                    Color::Red
                };
                spans.push(Span::styled(
                    format_hour_short(best_hour),
                    Style::default().fg(Color::White),
                ));
                spans.push(Span::styled(
                    format!(" ({})", score),
                    Style::default().fg(score_color),
                ));
            } else {
                spans.push(Span::styled(
                    "--:-- (--)",
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        // Add contextual hint in muted color if present (only when no activity selected)
        if app.current_activity.is_none() {
            if let Some(hint_text) = hint {
                spans.push(Span::raw("   "));
                spans.push(Span::styled(
                    hint_text,
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        let line = Line::from(spans);

        lines.push(line);
    }

    let block = Block::default()
        .title(" Vancouver Beaches ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(paragraph, area);
}

/// Renders the help text at the bottom of the screen with data freshness
fn render_help(frame: &mut Frame, area: Rect, app: &App) {
    let mut help_spans = vec![
        Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" Select  "),
        Span::styled("1-5", Style::default().fg(Color::Yellow)),
        Span::raw(" Activity  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" Refresh  "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(" Help  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit"),
    ];

    // Add data freshness indicator
    if let Some(last_refresh) = app.last_refresh {
        let elapsed = Local::now() - last_refresh;
        let mins_ago = elapsed.num_minutes();
        let freshness_text = if mins_ago < 1 {
            " │ Data: just now".to_string()
        } else if mins_ago < 60 {
            format!(" │ Data: {}m ago", mins_ago)
        } else {
            format!(" │ Data: {}h ago", elapsed.num_hours())
        };
        help_spans.push(Span::styled(
            freshness_text,
            Style::default().fg(Color::DarkGray),
        ));
    }

    let help_text = Line::from(help_spans);
    let paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}

/// Alias for render_beach_list for compatibility
#[allow(dead_code)]
pub fn render(frame: &mut Frame, app: &App) {
    render_beach_list(frame, app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, AppState};
    use crate::data::{WaterQuality, WaterStatus, Weather, WeatherCondition};
    use chrono::{NaiveDate, NaiveTime, Utc};
    use ratatui::{backend::TestBackend, Terminal};

    /// Helper to create a test app with some beach conditions
    fn create_test_app() -> App {
        let mut app = App::new();
        app.state = AppState::BeachList;
        app
    }

    /// Helper to create mock weather data
    #[allow(dead_code)]
    fn create_mock_weather(temp: f64, condition: WeatherCondition) -> Weather {
        Weather {
            temperature: temp,
            feels_like: temp + 1.0,
            condition,
            humidity: 65,
            wind: 10.0,
            uv: 5.0,
            sunrise: NaiveTime::from_hms_opt(5, 30, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
            fetched_at: Utc::now(),
            hourly: Vec::new(),
        }
    }

    /// Helper to create mock water quality data
    #[allow(dead_code)]
    fn create_mock_water_quality(status: WaterStatus) -> WaterQuality {
        WaterQuality {
            status,
            ecoli_count: Some(50),
            sample_date: NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(),
            advisory_reason: None,
            fetched_at: Utc::now(),
        }
    }

    #[test]
    fn test_render_produces_non_empty_buffer() {
        let app = create_test_app();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render_beach_list(frame, &app);
            })
            .unwrap();

        // Check that the buffer is not empty (has some content)
        let buffer = terminal.backend().buffer();
        let has_content = buffer.content().iter().any(|cell| cell.symbol() != " ");
        assert!(has_content, "Buffer should contain rendered content");
    }

    #[test]
    fn test_selected_item_is_highlighted() {
        let mut app = create_test_app();
        app.selected_index = 0;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render_beach_list(frame, &app);
            })
            .unwrap();

        // Check that the cursor indicator is present
        let buffer = terminal.backend().buffer();
        let buffer_str: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // The selected item should have the cursor indicator
        assert!(
            buffer_str.contains("\u{25B8}") || buffer_str.contains(">"),
            "Selected item should have cursor indicator"
        );
    }

    #[test]
    fn test_missing_weather_shows_placeholder() {
        let app = create_test_app();
        // App has no beach_conditions, so weather is missing

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render_beach_list(frame, &app);
            })
            .unwrap();

        // Check that placeholder is shown for missing weather
        let buffer = terminal.backend().buffer();
        let buffer_str: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // Should show "--°C" for missing temperature
        assert!(
            buffer_str.contains("--") || buffer_str.contains("?"),
            "Missing weather should show placeholder"
        );
    }

    #[test]
    fn test_all_beaches_are_rendered() {
        let app = create_test_app();
        let _beaches = all_beaches();

        let backend = TestBackend::new(80, 30); // Taller to fit all beaches
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render_beach_list(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let buffer_str: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // Check that at least some beaches are rendered
        // (the buffer might not be tall enough for all, but at least the first few should be there)
        assert!(
            buffer_str.contains("Kitsilano"),
            "First beach should be rendered"
        );
        assert!(
            buffer_str.contains("English Bay"),
            "Second beach should be rendered"
        );
    }

    #[test]
    fn test_weather_icons_mapping() {
        assert_eq!(weather_icon(&WeatherCondition::Clear), "\u{2600}");
        assert_eq!(weather_icon(&WeatherCondition::PartlyCloudy), "\u{26C5}");
        assert_eq!(weather_icon(&WeatherCondition::Cloudy), "\u{2601}");
        assert_eq!(weather_icon(&WeatherCondition::Rain), "\u{1F327}");
        assert_eq!(weather_icon(&WeatherCondition::Showers), "\u{1F326}");
        assert_eq!(weather_icon(&WeatherCondition::Thunderstorm), "\u{26C8}");
        assert_eq!(weather_icon(&WeatherCondition::Snow), "\u{2744}");
        assert_eq!(weather_icon(&WeatherCondition::Fog), "\u{1F32B}");
    }

    #[test]
    fn test_water_status_icons_mapping() {
        assert_eq!(water_status_icon(&WaterStatus::Safe), "\u{1F7E2}");
        assert_eq!(water_status_icon(&WaterStatus::Advisory), "\u{1F7E1}");
        assert_eq!(water_status_icon(&WaterStatus::Closed), "\u{1F534}");
        assert_eq!(water_status_icon(&WaterStatus::Unknown), "\u{26AA}");
    }

    #[test]
    fn test_water_status_colors() {
        assert_eq!(water_status_color(&WaterStatus::Safe), Color::Green);
        assert_eq!(water_status_color(&WaterStatus::Advisory), Color::Yellow);
        assert_eq!(water_status_color(&WaterStatus::Closed), Color::Red);
        assert_eq!(water_status_color(&WaterStatus::Unknown), Color::Gray);
    }

    #[test]
    fn test_temperature_colors() {
        // Hot temperatures should be red
        assert_eq!(temperature_color(35.0), Color::Red);
        assert_eq!(temperature_color(30.0), Color::Red);

        // Warm temperatures should be light red
        assert_eq!(temperature_color(27.0), Color::LightRed);

        // Comfortable temperatures should be yellow
        assert_eq!(temperature_color(22.0), Color::Yellow);

        // Cool temperatures should be green
        assert_eq!(temperature_color(17.0), Color::Green);

        // Cold temperatures should be cyan
        assert_eq!(temperature_color(12.0), Color::Cyan);

        // Very cold temperatures should be blue
        assert_eq!(temperature_color(5.0), Color::Blue);
    }

    #[test]
    fn test_help_text_is_rendered() {
        let app = create_test_app();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render_beach_list(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let buffer_str: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // Check that help text elements are present
        assert!(
            buffer_str.contains("Navigate") || buffer_str.contains("Quit"),
            "Help text should be rendered"
        );
    }

    #[test]
    fn test_title_is_rendered() {
        let app = create_test_app();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render_beach_list(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let buffer_str: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(
            buffer_str.contains("Vancouver Beaches"),
            "Title should be rendered"
        );
    }

    #[test]
    fn test_render_alias_works() {
        let app = create_test_app();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Using the render() alias should work the same as render_beach_list()
        terminal
            .draw(|frame| {
                render(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let has_content = buffer.content().iter().any(|cell| cell.symbol() != " ");
        assert!(has_content, "Buffer should contain rendered content");
    }
}
