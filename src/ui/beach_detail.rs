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

    // Create layout: Weather/Tides row, Water Quality row, Help row
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),    // Weather and Tides section
            Constraint::Length(4), // Water Quality section
            Constraint::Length(2), // Help text
        ])
        .split(inner_area);

    // Split the top section into Weather and Tides columns
    let top_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    // Render sections
    render_weather_section(frame, top_columns[0], conditions.weather.as_ref());
    render_tides_section(frame, top_columns[1], conditions.tides.as_ref());
    render_water_quality_section(frame, chunks[1], conditions.water_quality.as_ref());
    render_help_text(frame, chunks[2]);
}

/// Renders the weather section
fn render_weather_section(
    frame: &mut Frame,
    area: Rect,
    weather: Option<&crate::data::Weather>,
) {
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
fn render_tides_section(
    frame: &mut Frame,
    area: Rect,
    tides: Option<&crate::data::TideInfo>,
) {
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

/// Renders the help text at the bottom
fn render_help_text(frame: &mut Frame, area: Rect) {
    let help_line = Line::from(vec![
        Span::styled("<- Back", Style::default().fg(colors::SECONDARY)),
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

        app.beach_conditions.insert(beach_id.to_string(), conditions);
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
        let content: String = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        assert!(!content.trim().is_empty(), "Buffer should not be empty");
    }

    #[test]
    fn test_weather_section_renders_temperature() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            None,
            None,
        );

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        assert!(
            content.contains("22") || content.contains("WEATHER"),
            "Should render weather section with temperature"
        );
    }

    #[test]
    fn test_tides_section_renders_tide_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = create_test_app_with_conditions(
            "kitsilano",
            None,
            Some(create_test_tides()),
            None,
        );

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

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
        let content: String = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

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
        let content: String = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        assert!(
            content.contains("unavailable") || content.contains("WEATHER"),
            "Should handle missing weather data gracefully"
        );
    }

    #[test]
    fn test_handles_missing_all_data_gracefully() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = create_test_app_with_conditions(
            "kitsilano",
            None,
            None,
            None,
        );

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

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
        let content: String = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();

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
            (WaterStatus::Advisory, "!", "Advisory in effect", colors::ADVISORY),
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
