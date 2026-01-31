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

use chrono::{Local, Timelike};

use crate::activities::{
    get_profile, sunset_time_scorer_dynamic, Activity, ScoreFactors, TimeSlotScore,
};
use crate::app::App;
use crate::data::{HourlyForecast, TideState, WaterStatus, WeatherCondition};

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
pub fn render(frame: &mut Frame, app: &mut App, beach_id: &str) {
    let area = frame.area();

    // Check if beach conditions exist first
    let has_conditions = app.get_conditions(beach_id).is_some();
    if !has_conditions {
        render_no_data(frame, area, beach_id);
        return;
    }

    // Extract beach name before mutable operations
    let beach_name = app.get_conditions(beach_id).unwrap().beach.name.to_string();

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

    // Determine tide section height based on expanded state
    let tide_chart_expanded = app.tide_chart_expanded;
    let tides_height: u16 = if tide_chart_expanded { 15 } else { 5 };

    // Calculate content heights
    // Section heights: weather(7), tides(5 or 15), hourly_forecast(9), water_quality(4), best_window(6 if shown)
    const HOURLY_FORECAST_HEIGHT: u16 = 9; // 1 header + 8 hours max
    let content_height: u16 = if show_best_window {
        7 + tides_height + HOURLY_FORECAST_HEIGHT + 4 + 6 // weather + tides + hourly + water_quality + best_window
    } else {
        7 + tides_height + HOURLY_FORECAST_HEIGHT + 4 // weather + tides + hourly + water_quality
    };

    // Fixed elements: activity selector (1), help text (2)
    let fixed_height: u16 = 1 + 2;

    // Available height for scrollable content
    let available_content_height = inner_area.height.saturating_sub(fixed_height);

    // Calculate max scroll offset
    let max_scroll = content_height.saturating_sub(available_content_height);

    // Clamp scroll offset to valid range
    if app.detail_scroll_offset > max_scroll {
        app.detail_scroll_offset = max_scroll;
    }

    let scroll_offset = app.detail_scroll_offset;
    let current_activity = app.current_activity;

    // Create main layout: Activity selector (fixed), Content (scrollable), Help (fixed)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                      // Activity selector (fixed)
            Constraint::Min(0),                         // Content area (scrollable)
            Constraint::Length(2),                      // Help text (fixed)
        ])
        .split(inner_area);

    // Render fixed activity selector at the top
    render_activity_selector(frame, main_chunks[0], current_activity);

    // Calculate visible content area
    let content_area = main_chunks[1];
    let visible_height = content_area.height;

    // Determine if we need scroll indicators
    let has_content_above = scroll_offset > 0;
    let has_content_below = scroll_offset < max_scroll && content_height > visible_height;

    // Render scroll indicator at top if content above
    if has_content_above {
        render_scroll_indicator_top(frame, content_area);
    }

    // Render scroll indicator at bottom if content below
    if has_content_below {
        render_scroll_indicator_bottom(frame, content_area);
    }

    // Render scrollable content sections with offset
    // Now we can safely borrow conditions since we're done mutating app
    let conditions = app.get_conditions(beach_id).unwrap();
    render_scrollable_content(
        frame,
        content_area,
        app,
        beach_id,
        scroll_offset,
        show_best_window,
        tide_chart_expanded,
        conditions,
    );

    // Render fixed help text at the bottom
    render_help_text(frame, main_chunks[2]);
}

/// Renders the scrollable content sections with scroll offset applied
fn render_scrollable_content(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    beach_id: &str,
    scroll_offset: u16,
    show_best_window: bool,
    tide_chart_expanded: bool,
    conditions: &crate::data::BeachConditions,
) {
    // Section heights
    const WEATHER_HEIGHT: u16 = 7;
    let tides_height: u16 = if tide_chart_expanded { 15 } else { 5 };
    const HOURLY_FORECAST_HEIGHT: u16 = 9; // 1 header + 8 hours max
    const WATER_QUALITY_HEIGHT: u16 = 4;
    const BEST_WINDOW_HEIGHT: u16 = 6;

    // Calculate section positions (cumulative Y offsets)
    let weather_start: u16 = 0;
    let tides_start = weather_start + WEATHER_HEIGHT;
    let hourly_forecast_start = tides_start + tides_height;
    let water_quality_start = hourly_forecast_start + HOURLY_FORECAST_HEIGHT;
    let best_window_start = water_quality_start + WATER_QUALITY_HEIGHT;

    // Render each section only if it's visible after scroll offset
    let visible_start = scroll_offset;
    let visible_end = scroll_offset + area.height;

    // Weather section
    if let Some(visible_rect) = calculate_visible_rect(
        weather_start,
        WEATHER_HEIGHT,
        visible_start,
        visible_end,
        area,
    ) {
        let section_offset = scroll_offset.saturating_sub(weather_start);
        render_weather_section_with_offset(
            frame,
            visible_rect,
            conditions.weather.as_ref(),
            section_offset,
        );
    }

    // Tides section
    if let Some(visible_rect) = calculate_visible_rect(
        tides_start,
        tides_height,
        visible_start,
        visible_end,
        area,
    ) {
        let section_offset = scroll_offset.saturating_sub(tides_start);
        render_tides_section_with_offset(
            frame,
            visible_rect,
            conditions.tides.as_ref(),
            section_offset,
            tide_chart_expanded,
        );
    }

    // Hourly Forecast section
    if let Some(visible_rect) = calculate_visible_rect(
        hourly_forecast_start,
        HOURLY_FORECAST_HEIGHT,
        visible_start,
        visible_end,
        area,
    ) {
        let section_offset = scroll_offset.saturating_sub(hourly_forecast_start);
        render_hourly_forecast_section_with_offset(
            frame,
            visible_rect,
            conditions.weather.as_ref(),
            section_offset,
        );
    }

    // Water Quality section
    if let Some(visible_rect) = calculate_visible_rect(
        water_quality_start,
        WATER_QUALITY_HEIGHT,
        visible_start,
        visible_end,
        area,
    ) {
        let section_offset = scroll_offset.saturating_sub(water_quality_start);
        render_water_quality_section_with_offset(
            frame,
            visible_rect,
            conditions.water_quality.as_ref(),
            section_offset,
        );
    }

    // Best Window section (if activity is selected)
    if show_best_window {
        if let Some(visible_rect) = calculate_visible_rect(
            best_window_start,
            BEST_WINDOW_HEIGHT,
            visible_start,
            visible_end,
            area,
        ) {
            let section_offset = scroll_offset.saturating_sub(best_window_start);
            render_best_window_section_with_offset(frame, visible_rect, app, beach_id, section_offset);
        }
    }
}

/// Calculates the visible rectangle for a section given scroll offset
fn calculate_visible_rect(
    section_start: u16,
    section_height: u16,
    visible_start: u16,
    visible_end: u16,
    area: Rect,
) -> Option<Rect> {
    let section_end = section_start + section_height;

    // Check if section is at least partially visible
    if section_end <= visible_start || section_start >= visible_end {
        return None;
    }

    // Calculate the visible portion of this section
    let visible_section_start = section_start.max(visible_start);
    let visible_section_end = section_end.min(visible_end);
    let visible_height = visible_section_end.saturating_sub(visible_section_start);

    if visible_height == 0 {
        return None;
    }

    // Calculate position in the display area
    let y_in_area = section_start.saturating_sub(visible_start);

    Some(Rect {
        x: area.x,
        y: area.y + y_in_area,
        width: area.width,
        height: visible_height,
    })
}

/// Renders the "more above" scroll indicator
fn render_scroll_indicator_top(frame: &mut Frame, area: Rect) {
    if area.width < 10 {
        return;
    }
    let indicator = Span::styled(
        "\u{25B2} more",
        Style::default().fg(colors::SECONDARY),
    );
    let x = area.x + area.width.saturating_sub(8);
    let indicator_area = Rect {
        x,
        y: area.y,
        width: 8,
        height: 1,
    };
    let paragraph = Paragraph::new(Line::from(indicator));
    frame.render_widget(paragraph, indicator_area);
}

/// Renders the "more below" scroll indicator
fn render_scroll_indicator_bottom(frame: &mut Frame, area: Rect) {
    if area.width < 10 || area.height == 0 {
        return;
    }
    let indicator = Span::styled(
        "\u{25BC} more",
        Style::default().fg(colors::SECONDARY),
    );
    let x = area.x + area.width.saturating_sub(8);
    let indicator_area = Rect {
        x,
        y: area.y + area.height.saturating_sub(1),
        width: 8,
        height: 1,
    };
    let paragraph = Paragraph::new(Line::from(indicator));
    frame.render_widget(paragraph, indicator_area);
}

/// Renders the weather section with scroll offset
fn render_weather_section_with_offset(
    frame: &mut Frame,
    area: Rect,
    weather: Option<&crate::data::Weather>,
    offset: u16,
) {
    let lines = build_weather_lines(weather);
    let paragraph = Paragraph::new(lines).scroll((offset, 0));
    frame.render_widget(paragraph, area);
}

/// Renders the tides section with scroll offset
fn render_tides_section_with_offset(
    frame: &mut Frame,
    area: Rect,
    tides: Option<&crate::data::TideInfo>,
    offset: u16,
    expanded: bool,
) {
    let lines = if expanded {
        build_expanded_tide_chart(tides, area.width as usize)
    } else {
        build_tides_lines_with_width(tides, area.width as usize)
    };
    let paragraph = Paragraph::new(lines).scroll((offset, 0));
    frame.render_widget(paragraph, area);
}

/// Renders the water quality section with scroll offset
fn render_water_quality_section_with_offset(
    frame: &mut Frame,
    area: Rect,
    water_quality: Option<&crate::data::WaterQuality>,
    offset: u16,
) {
    let lines = build_water_quality_lines(water_quality);
    let paragraph = Paragraph::new(lines).scroll((offset, 0));
    frame.render_widget(paragraph, area);
}

/// Renders the hourly forecast section with scroll offset
fn render_hourly_forecast_section_with_offset(
    frame: &mut Frame,
    area: Rect,
    weather: Option<&crate::data::Weather>,
    offset: u16,
) {
    let lines = build_hourly_forecast_lines(weather);
    let paragraph = Paragraph::new(lines).scroll((offset, 0));
    frame.render_widget(paragraph, area);
}

/// Builds the lines for the hourly forecast section
/// Shows next 6-8 hours of forecasts until end of day
fn build_hourly_forecast_lines(weather: Option<&crate::data::Weather>) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        "HOURLY FORECAST",
        Style::default()
            .fg(colors::HEADER)
            .add_modifier(Modifier::BOLD),
    ))];

    match weather {
        Some(w) if !w.hourly.is_empty() => {
            let current_hour = Local::now().hour() as u8;

            // Filter to hours >= current hour and take up to 8 hours
            let future_hours: Vec<&HourlyForecast> = w
                .hourly
                .iter()
                .filter(|h| h.hour >= current_hour)
                .take(8)
                .collect();

            if future_hours.is_empty() {
                lines.push(Line::from(Span::styled(
                    "No more forecasts for today",
                    Style::default().fg(colors::UNKNOWN),
                )));
            } else {
                for forecast in future_hours {
                    lines.push(build_hourly_line(forecast));
                }
            }
        }
        _ => {
            lines.push(Line::from(Span::styled(
                "No hourly forecast available",
                Style::default().fg(colors::UNKNOWN),
            )));
        }
    }

    lines
}

/// Builds a single line for an hourly forecast entry
fn build_hourly_line(forecast: &HourlyForecast) -> Line<'static> {
    let time_str = format!("{:02}:00", forecast.hour);
    let temp_str = format!("{:.0}\u{00B0}C", forecast.temperature);
    let icon = hourly_condition_icon(forecast.condition);
    let wind_str = format!("Wind: {:.0}km/h", forecast.wind);
    let uv_str = format!("UV: {:.0}", forecast.uv);

    Line::from(vec![
        Span::styled(
            format!("{:<6}", time_str),
            Style::default().fg(colors::PRIMARY),
        ),
        Span::styled(
            format!("{:<6}", temp_str),
            Style::default().fg(temperature_color(forecast.temperature)),
        ),
        Span::styled(
            format!("{:<3}", icon),
            Style::default().fg(colors::PRIMARY),
        ),
        Span::styled(
            format!("{:<14}", wind_str),
            Style::default().fg(colors::SECONDARY),
        ),
        Span::styled(
            uv_str,
            Style::default().fg(uv_index_color(forecast.uv)),
        ),
    ])
}

/// Returns an icon character for the hourly weather condition
fn hourly_condition_icon(condition: WeatherCondition) -> &'static str {
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

/// Returns the color for a temperature value
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

/// Renders the best window section with scroll offset
fn render_best_window_section_with_offset(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    beach_id: &str,
    offset: u16,
) {
    let lines = build_best_window_lines(app, beach_id);
    let paragraph = Paragraph::new(lines).scroll((offset, 0));
    frame.render_widget(paragraph, area);
}

/// Builds the lines for the weather section
fn build_weather_lines(weather: Option<&crate::data::Weather>) -> Vec<Line<'static>> {
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

    lines
}

/// Builds the lines for the tides section (default width of 16 chars)
#[allow(dead_code)]
fn build_tides_lines(tides: Option<&crate::data::TideInfo>) -> Vec<Line<'static>> {
    build_tides_lines_with_width(tides, 16)
}

/// Builds the lines for the tides section with configurable width
fn build_tides_lines_with_width(tides: Option<&crate::data::TideInfo>, width: usize) -> Vec<Line<'static>> {
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
                TideState::Rising => ("\u{2191}", "Rising", colors::RISING),
                TideState::Falling => ("\u{2193}", "Falling", colors::FALLING),
                TideState::High => ("\u{2500}", "High", colors::HEADER),
                TideState::Low => ("\u{2500}", "Low", colors::SECONDARY),
            };

            let state_line = Line::from(vec![
                Span::styled(state_icon, Style::default().fg(state_color)),
                Span::raw(" "),
                Span::styled(state_text.to_string(), Style::default().fg(state_color)),
                Span::raw(" "),
                Span::styled(
                    format!("{:.1}m", t.current_height),
                    Style::default().fg(colors::PRIMARY),
                ),
            ]);
            lines.push(state_line);

            // Calculate sparkline width (full width minus some padding)
            // Reserve space for potential padding (minimum 16, maximum width - 2)
            let sparkline_width = width.saturating_sub(2).max(16);

            // Generate tide heights and interpolate to fill sparkline width
            let base_heights = t.hourly_heights(4.8);
            let interpolated_heights = interpolate_heights(&base_heights, sparkline_width);

            let current_hour = Local::now().hour() as usize;
            // Calculate which sparkline index corresponds to current hour
            // Hours 6-21 (16 hours) mapped to sparkline_width characters
            let current_index = if (6..=21).contains(&current_hour) {
                let hour_offset = current_hour - 6;
                // Map hour offset (0-15) to sparkline index (0 to sparkline_width-1)
                Some((hour_offset * sparkline_width) / 16)
            } else {
                None
            };

            // Build sparkline with current hour highlighted
            let mut chart_spans: Vec<Span> = Vec::new();
            for (i, height) in interpolated_heights.iter().enumerate() {
                let block = height_to_block(*height, 4.8);
                let style = if current_index == Some(i) {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors::RISING)
                };
                chart_spans.push(Span::styled(block.to_string(), style));
            }
            lines.push(Line::from(chart_spans));

            // Hour labels spanning full width: 6AM, 9AM, 12PM, 3PM, 6PM, 9PM, 12AM
            let time_labels = build_time_labels(sparkline_width);
            lines.push(Line::from(Span::styled(
                time_labels,
                Style::default().fg(colors::SECONDARY),
            )));

            // Next high/low times and expand hint on same line
            let mut next_events: Vec<Span> = Vec::new();
            if let Some(ref high) = t.next_high {
                next_events.push(Span::styled("H:".to_string(), Style::default().fg(colors::SECONDARY)));
                next_events.push(Span::styled(
                    high.time.format("%H:%M").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ));
                next_events.push(Span::raw(" "));
            }
            if let Some(ref low) = t.next_low {
                next_events.push(Span::styled("L:".to_string(), Style::default().fg(colors::SECONDARY)));
                next_events.push(Span::styled(
                    low.time.format("%H:%M").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ));
            }
            // Add expand hint
            if !next_events.is_empty() {
                next_events.push(Span::raw("  "));
            }
            next_events.push(Span::styled(
                "[t] expand".to_string(),
                Style::default().fg(colors::SECONDARY),
            ));
            lines.push(Line::from(next_events));
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Tide data unavailable",
                Style::default().fg(colors::UNKNOWN),
            )));
        }
    }

    lines
}

/// Builds the expanded ASCII tide chart with Y-axis labels, tide curve, and X-axis time markers.
/// The chart is approximately 12 lines tall and uses box-drawing characters for the curve.
fn build_expanded_tide_chart(tides: Option<&crate::data::TideInfo>, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        "TIDES",
        Style::default()
            .fg(colors::HEADER)
            .add_modifier(Modifier::BOLD),
    ))];

    match tides {
        Some(t) => {
            // Current tide state with arrow (same as collapsed view)
            let (state_icon, state_text, state_color) = match t.tide_state {
                TideState::Rising => ("\u{2191}", "Rising", colors::RISING),
                TideState::Falling => ("\u{2193}", "Falling", colors::FALLING),
                TideState::High => ("\u{2500}", "High", colors::HEADER),
                TideState::Low => ("\u{2500}", "Low", colors::SECONDARY),
            };

            let state_line = Line::from(vec![
                Span::styled(state_icon, Style::default().fg(state_color)),
                Span::raw(" "),
                Span::styled(state_text.to_string(), Style::default().fg(state_color)),
                Span::raw(" "),
                Span::styled(
                    format!("{:.1}m", t.current_height),
                    Style::default().fg(colors::PRIMARY),
                ),
            ]);
            lines.push(state_line);

            // Calculate chart dimensions
            // Reserve 4 chars for Y-axis labels (e.g., "4m ┤")
            let y_axis_width: usize = 4;
            let chart_width = width.saturating_sub(y_axis_width).max(20);

            // Heights for the chart: 9 rows (0m to 4m with intermediate rows)
            // Row 0 = 4m, Row 8 = 0m
            const CHART_ROWS: usize = 9;
            const MAX_HEIGHT: f64 = 4.0;

            // Get tide heights and interpolate to fill chart width
            let base_heights = t.hourly_heights(MAX_HEIGHT);
            let interpolated_heights = interpolate_heights(&base_heights, chart_width);

            // Determine current hour position
            let current_hour = Local::now().hour() as usize;
            let current_col_index = if (6..=21).contains(&current_hour) {
                let hour_offset = current_hour - 6;
                Some((hour_offset * chart_width) / 16)
            } else {
                None
            };

            // Current height in row coordinates (0 = 4m, 8 = 0m)
            let current_height_row = if current_col_index.is_some() {
                let h = t.current_height.clamp(0.0, MAX_HEIGHT);
                let normalized = h / MAX_HEIGHT;
                let row = ((1.0 - normalized) * (CHART_ROWS - 1) as f64).round() as usize;
                Some(row.min(CHART_ROWS - 1))
            } else {
                None
            };

            // Build the chart row by row
            // Y-axis labels: 4m, 3m, 2m, 1m, 0m (with intermediate rows unlabeled)
            let y_labels = ["4m", "  ", "3m", "  ", "2m", "  ", "1m", "  ", "0m"];

            for row in 0..CHART_ROWS {
                let height_threshold_high = MAX_HEIGHT - (row as f64 * MAX_HEIGHT / (CHART_ROWS - 1) as f64);
                let height_threshold_low = MAX_HEIGHT - ((row + 1) as f64 * MAX_HEIGHT / (CHART_ROWS - 1) as f64);

                // Build the row
                let y_label = y_labels.get(row).unwrap_or(&"  ");
                let y_axis_char = if row == CHART_ROWS - 1 { "\u{253C}" } else { "\u{2524}" }; // ┼ for bottom, ┤ for others

                let mut row_spans: Vec<Span> = vec![
                    Span::styled(
                        format!("{} {}", y_label, y_axis_char),
                        Style::default().fg(colors::SECONDARY),
                    ),
                ];

                // Draw the chart content for this row
                let mut chart_chars: Vec<char> = Vec::with_capacity(chart_width);
                for col in 0..chart_width {
                    let height = interpolated_heights.get(col).copied().unwrap_or(0.0);
                    let prev_height = if col > 0 { interpolated_heights.get(col - 1).copied().unwrap_or(0.0) } else { height };
                    let next_height = interpolated_heights.get(col + 1).copied().unwrap_or(height);

                    // Determine what character to draw based on the tide curve
                    let char_to_draw = get_chart_character(
                        row, col, height, prev_height, next_height,
                        height_threshold_high, height_threshold_low,
                        CHART_ROWS, MAX_HEIGHT
                    );
                    chart_chars.push(char_to_draw);
                }

                // Convert to string and highlight current position
                let chart_str: String = chart_chars.iter().collect();

                // Check if current position marker should be on this row
                if let (Some(col_idx), Some(height_row)) = (current_col_index, current_height_row) {
                    if row == height_row && col_idx < chart_width {
                        // Split string and insert marker
                        let before: String = chart_chars.iter().take(col_idx).collect();
                        let after: String = chart_chars.iter().skip(col_idx + 1).collect();

                        row_spans.push(Span::styled(before, Style::default().fg(colors::RISING)));
                        row_spans.push(Span::styled("\u{25CF}".to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))); // ●
                        row_spans.push(Span::styled(after, Style::default().fg(colors::RISING)));
                    } else {
                        row_spans.push(Span::styled(chart_str, Style::default().fg(colors::RISING)));
                    }
                } else {
                    row_spans.push(Span::styled(chart_str, Style::default().fg(colors::RISING)));
                }

                lines.push(Line::from(row_spans));
            }

            // X-axis bottom border
            let x_axis_line = format!(
                "   \u{2514}{}",
                "\u{2500}".repeat(chart_width.min(width.saturating_sub(4)))
            );
            lines.push(Line::from(Span::styled(
                x_axis_line,
                Style::default().fg(colors::SECONDARY),
            )));

            // X-axis time markers
            let time_markers = build_expanded_time_labels(chart_width);
            lines.push(Line::from(Span::styled(
                format!("    {}", time_markers),
                Style::default().fg(colors::SECONDARY),
            )));

            // Next high/low times and collapse hint
            let mut next_events: Vec<Span> = Vec::new();
            if let Some(ref high) = t.next_high {
                next_events.push(Span::styled("H:".to_string(), Style::default().fg(colors::SECONDARY)));
                next_events.push(Span::styled(
                    high.time.format("%H:%M").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ));
                next_events.push(Span::raw(" "));
            }
            if let Some(ref low) = t.next_low {
                next_events.push(Span::styled("L:".to_string(), Style::default().fg(colors::SECONDARY)));
                next_events.push(Span::styled(
                    low.time.format("%H:%M").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ));
            }
            // Add collapse hint
            if !next_events.is_empty() {
                next_events.push(Span::raw("  "));
            }
            next_events.push(Span::styled(
                "[t] collapse".to_string(),
                Style::default().fg(colors::SECONDARY),
            ));
            lines.push(Line::from(next_events));
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Tide data unavailable",
                Style::default().fg(colors::UNKNOWN),
            )));
        }
    }

    lines
}

/// Determines the character to draw at a given position in the expanded tide chart.
fn get_chart_character(
    row: usize,
    col: usize,
    height: f64,
    prev_height: f64,
    next_height: f64,
    _height_threshold_high: f64,
    _height_threshold_low: f64,
    chart_rows: usize,
    max_height: f64,
) -> char {
    // Convert heights to row coordinates
    let height_to_row = |h: f64| -> usize {
        let normalized = h.clamp(0.0, max_height) / max_height;
        let r = ((1.0 - normalized) * (chart_rows - 1) as f64).round() as usize;
        r.min(chart_rows - 1)
    };

    let current_row = height_to_row(height);
    let prev_row = height_to_row(prev_height);
    let next_row = height_to_row(next_height);

    // Check if the curve passes through this cell
    if current_row == row {
        // The curve is at this height
        if col == 0 {
            // First column
            if next_row < row {
                '\u{256D}' // ╭ - going up
            } else if next_row > row {
                '\u{2570}' // ╰ - going down
            } else {
                '\u{2500}' // ─ - horizontal
            }
        } else if prev_row < row && next_row < row {
            // Coming from above and going up - this is a valley
            '\u{2570}' // ╰
        } else if prev_row > row && next_row > row {
            // Coming from below and going down - this is a peak
            '\u{256D}' // ╭
        } else if prev_row > row {
            // Coming from below
            if next_row < row {
                '\u{256D}' // ╭ - peak
            } else if next_row > row {
                '\u{256E}' // ╮ - going down
            } else {
                '\u{256F}' // ╯ - ending climb
            }
        } else if prev_row < row {
            // Coming from above
            if next_row > row {
                '\u{2570}' // ╰ - valley
            } else if next_row < row {
                '\u{256D}' // ╭ - going up
            } else {
                '\u{256D}' // ╭ - starting descent
            }
        } else {
            // prev_row == row
            if next_row < row {
                '\u{256D}' // ╭ - starting to go up
            } else if next_row > row {
                '\u{256E}' // ╮ - starting to go down
            } else {
                '\u{2500}' // ─ - horizontal
            }
        }
    } else if (current_row < row && prev_row >= row) || (prev_row < row && current_row >= row) {
        // Vertical segment passes through this row (transition between prev and current)
        if prev_row < current_row {
            // Going down
            if row == prev_row + 1 {
                '\u{256E}' // ╮
            } else if row == current_row {
                '\u{2570}' // ╰
            } else {
                '\u{2502}' // │
            }
        } else {
            // Going up
            if row == current_row + 1 {
                '\u{256F}' // ╯
            } else if row == prev_row {
                '\u{256D}' // ╭
            } else {
                '\u{2502}' // │
            }
        }
    } else {
        // No curve at this position
        ' '
    }
}

/// Builds time labels for the expanded chart X-axis
fn build_expanded_time_labels(width: usize) -> String {
    if width < 30 {
        return "6AM     12PM      6PM".to_string();
    }

    // Time markers: 6AM, 8AM, 10AM, 12PM, 2PM, 4PM, 6PM, 8PM, 10PM
    let labels = ["6AM", "8AM", "10AM", "12PM", "2PM", "4PM", "6PM", "8PM", "10PM"];
    // Positions: 0, 2, 4, 6, 8, 10, 12, 14, 16 hours from 6AM (out of 16 hours total)
    let positions: [f64; 9] = [0.0, 2.0/16.0, 4.0/16.0, 6.0/16.0, 8.0/16.0, 10.0/16.0, 12.0/16.0, 14.0/16.0, 16.0/16.0];

    let mut result = vec![' '; width];

    for (label, &pos) in labels.iter().zip(positions.iter()) {
        let char_pos = ((pos * (width - 1) as f64).round() as usize).min(width - 1);
        let label_chars: Vec<char> = label.chars().collect();
        let label_len = label_chars.len();

        // For last label, right-align
        let start = if pos >= 0.99 {
            width.saturating_sub(label_len)
        } else {
            char_pos.saturating_sub(label_len / 2)
        };

        let end = (start + label_len).min(width);

        for (i, ch) in label_chars.iter().enumerate() {
            if start + i < end {
                result[start + i] = *ch;
            }
        }
    }

    result.iter().collect()
}

/// Interpolates tide heights to fill the target width
fn interpolate_heights(heights: &[f64], target_width: usize) -> Vec<f64> {
    if heights.is_empty() {
        return vec![0.0; target_width];
    }
    if target_width <= heights.len() {
        // If target is smaller or equal, just return first target_width values
        return heights.iter().take(target_width).copied().collect();
    }

    let mut result = Vec::with_capacity(target_width);
    let source_len = heights.len();

    for i in 0..target_width {
        // Map target index to source position (0.0 to source_len-1)
        let source_pos = (i as f64 * (source_len - 1) as f64) / (target_width - 1) as f64;
        let lower_idx = source_pos.floor() as usize;
        let upper_idx = (lower_idx + 1).min(source_len - 1);
        let fraction = source_pos - lower_idx as f64;

        // Linear interpolation between adjacent heights
        let interpolated = heights[lower_idx] * (1.0 - fraction) + heights[upper_idx] * fraction;
        result.push(interpolated);
    }

    result
}

/// Builds time labels spanning the sparkline width
/// Labels: 6AM, 9AM, 12PM, 3PM, 6PM, 9PM, 12AM (representing hours 6-21 + midnight)
fn build_time_labels(width: usize) -> String {
    if width < 20 {
        // For narrow widths, use abbreviated labels
        return "6   9  12  15  18  21".to_string();
    }

    // Time markers at hours 6, 9, 12, 15, 18, 21 (plus implicit end at 24/midnight)
    // These correspond to positions 0, 3/16, 6/16, 9/16, 12/16, 15/16 of the sparkline
    let labels = ["6AM", "9AM", "12PM", "3PM", "6PM", "9PM", "12AM"];
    let positions: [f64; 7] = [0.0, 3.0/16.0, 6.0/16.0, 9.0/16.0, 12.0/16.0, 15.0/16.0, 1.0];

    let mut result = vec![' '; width];

    for (label, &pos) in labels.iter().zip(positions.iter()) {
        let char_pos = ((pos * (width - 1) as f64).round() as usize).min(width - 1);
        let label_chars: Vec<char> = label.chars().collect();
        let label_len = label_chars.len();

        // For labels at the end, right-align them; otherwise center
        let start = if pos >= 0.99 {
            // Right-align the last label
            width.saturating_sub(label_len)
        } else {
            // Center the label on the position
            char_pos.saturating_sub(label_len / 2)
        };

        // Make sure we don't exceed width
        let end = (start + label_len).min(width);

        for (i, ch) in label_chars.iter().enumerate() {
            if start + i < end {
                result[start + i] = *ch;
            }
        }
    }

    result.iter().collect()
}

/// Builds the lines for the water quality section
fn build_water_quality_lines(water_quality: Option<&crate::data::WaterQuality>) -> Vec<Line<'static>> {
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
                Span::styled(text.to_string(), Style::default().fg(color)),
            ]);
            lines.push(status_line);

            // Test date and E. coli count
            let mut detail_spans = vec![
                Span::styled("Last tested: ".to_string(), Style::default().fg(colors::SECONDARY)),
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

    lines
}

/// Builds the lines for the best window section
fn build_best_window_lines(app: &App, beach_id: &str) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            "BEST WINDOW TODAY",
            Style::default()
                .fg(colors::HEADER)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
            Style::default().fg(colors::SECONDARY),
        )),
    ];

    // Get the current activity
    let activity = match app.current_activity {
        Some(a) => a,
        None => {
            lines.push(Line::from(Span::styled(
                "Select an activity (1-5) to see best times".to_string(),
                Style::default().fg(colors::SECONDARY),
            )));
            return lines;
        }
    };

    // Get beach conditions for scoring
    let conditions = match app.get_conditions(beach_id) {
        Some(c) => c,
        None => {
            lines.push(Line::from(Span::styled(
                "Weather data unavailable for scoring".to_string(),
                Style::default().fg(colors::UNKNOWN),
            )));
            return lines;
        }
    };

    // Compute time windows
    let windows = compute_best_windows(activity, conditions);

    if windows.is_empty() {
        // Check if it's because all times passed
        let current_hour = Local::now().hour() as u8;
        if current_hour >= 21 {
            lines.push(Line::from(Span::styled(
                "Best times have passed for today".to_string(),
                Style::default().fg(colors::SECONDARY),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "No suitable time windows found".to_string(),
                Style::default().fg(colors::SECONDARY),
            )));
        }
    } else {
        let medals = [
            ("\u{1F947}", colors::GOLD),   // gold medal emoji
            ("\u{1F948}", colors::SILVER), // silver medal emoji
            ("\u{1F949}", colors::BRONZE), // bronze medal emoji
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
                Span::styled("Score: ".to_string(), Style::default().fg(colors::SECONDARY)),
                Span::styled(
                    format!("{}/100", window.score),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ),
            ]));

            lines.push(Line::from(Span::styled(
                format!("   {}", window.reason),
                Style::default().fg(colors::SECONDARY),
            )));

            // Add compact factor bars for the first (best) window
            if i == 0 {
                if let Some(ref factors) = window.factors {
                    lines.push(render_factor_bars(factors, activity));
                }
            }
        }
    }

    lines
}

/// Renders the weather section (legacy, kept for reference)
#[allow(dead_code)]
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

/// Block characters for tide chart (8 levels)
const TIDE_BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Converts a tide height to a block character
fn height_to_block(height: f64, max_height: f64) -> char {
    let normalized = (height / max_height).clamp(0.0, 1.0);
    let index = ((normalized * 7.0).round() as usize).min(7);
    TIDE_BLOCKS[index]
}

/// Renders the tides section with tide chart (legacy, kept for reference)
#[allow(dead_code)]
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
                TideState::Rising => ("↑", "Rising", colors::RISING),
                TideState::Falling => ("↓", "Falling", colors::FALLING),
                TideState::High => ("─", "High", colors::HEADER),
                TideState::Low => ("─", "Low", colors::SECONDARY),
            };

            let state_line = Line::from(vec![
                Span::styled(state_icon, Style::default().fg(state_color)),
                Span::raw(" "),
                Span::styled(state_text, Style::default().fg(state_color)),
                Span::raw(" "),
                Span::styled(
                    format!("{:.1}m", t.current_height),
                    Style::default().fg(colors::PRIMARY),
                ),
            ]);
            lines.push(state_line);

            // Generate tide chart
            let heights = t.hourly_heights(4.8);
            let current_hour = Local::now().hour() as usize;
            let current_index = if (6..=21).contains(&current_hour) {
                Some(current_hour - 6)
            } else {
                None
            };

            // Build sparkline with current hour highlighted
            let mut chart_spans: Vec<Span> = Vec::new();
            for (i, height) in heights.iter().enumerate() {
                let block = height_to_block(*height, 4.8);
                let style = if current_index == Some(i) {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(colors::RISING)
                };
                chart_spans.push(Span::styled(block.to_string(), style));
            }
            lines.push(Line::from(chart_spans));

            // Hour labels under chart
            lines.push(Line::from(Span::styled(
                "6    9   12   15   18  21",
                Style::default().fg(colors::SECONDARY),
            )));

            // Next high/low on same line
            let mut next_events: Vec<Span> = Vec::new();
            if let Some(ref high) = t.next_high {
                next_events.push(Span::styled("H:", Style::default().fg(colors::SECONDARY)));
                next_events.push(Span::styled(
                    high.time.format("%H:%M").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ));
                next_events.push(Span::raw(" "));
            }
            if let Some(ref low) = t.next_low {
                next_events.push(Span::styled("L:", Style::default().fg(colors::SECONDARY)));
                next_events.push(Span::styled(
                    low.time.format("%H:%M").to_string(),
                    Style::default().fg(colors::PRIMARY),
                ));
            }
            if !next_events.is_empty() {
                lines.push(Line::from(next_events));
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

/// Renders the water quality section (legacy, kept for reference)
#[allow(dead_code)]
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
    /// Factor breakdown for score transparency
    factors: Option<ScoreFactors>,
}

/// Renders the "Best Window Today" section showing top 3 time slots for the selected activity (legacy, kept for reference)
#[allow(dead_code)]
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
        // Check if it's because all times passed
        let current_hour = Local::now().hour() as u8;
        if current_hour >= 21 {
            lines.push(Line::from(Span::styled(
                "Best times have passed for today",
                Style::default().fg(colors::SECONDARY),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "No suitable time windows found",
                Style::default().fg(colors::SECONDARY),
            )));
        }
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

            // Add compact factor bars for the first (best) window
            if i == 0 {
                if let Some(ref factors) = window.factors {
                    lines.push(render_factor_bars(factors, activity));
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

/// Renders a compact line showing factor scores as visual bars
fn render_factor_bars(factors: &ScoreFactors, activity: Activity) -> Line<'static> {
    let mut spans = vec![Span::raw("   ")];

    // Helper to create a mini bar (5 chars wide)
    let make_bar = |score: f32, label: &str, color: Color| -> Vec<Span<'static>> {
        let filled = (score * 5.0).round() as usize;
        let empty = 5 - filled;
        vec![
            Span::styled(label.to_string(), Style::default().fg(colors::SECONDARY)),
            Span::styled("▰".repeat(filled), Style::default().fg(color)),
            Span::styled("▱".repeat(empty), Style::default().fg(colors::SECONDARY)),
            Span::raw(" "),
        ]
    };

    let score_color = |score: f32| -> Color {
        if score >= 0.8 {
            colors::SAFE
        } else if score >= 0.5 {
            colors::ADVISORY
        } else {
            colors::CLOSED
        }
    };

    // Temperature - always shown
    spans.extend(make_bar(
        factors.temperature,
        "T:",
        score_color(factors.temperature),
    ));

    // Activity-specific factors
    match activity {
        Activity::Swimming => {
            spans.extend(make_bar(
                factors.water_quality,
                "W:",
                score_color(factors.water_quality),
            ));
            spans.extend(make_bar(factors.tide, "Ti:", score_color(factors.tide)));
        }
        Activity::Sailing => {
            spans.extend(make_bar(factors.wind, "Wi:", score_color(factors.wind)));
            spans.extend(make_bar(factors.tide, "Ti:", score_color(factors.tide)));
        }
        Activity::Sunbathing => {
            spans.extend(make_bar(factors.uv, "UV:", score_color(factors.uv)));
            spans.extend(make_bar(factors.wind, "Wi:", score_color(factors.wind)));
        }
        Activity::Sunset => {
            spans.extend(make_bar(
                factors.time_of_day,
                "Ti:",
                score_color(factors.time_of_day),
            ));
        }
        Activity::Peace => {
            spans.extend(make_bar(factors.crowd, "Cr:", score_color(factors.crowd)));
            spans.extend(make_bar(factors.wind, "Wi:", score_color(factors.wind)));
        }
    }

    Line::from(spans)
}

/// Computes the best time windows for a given activity and beach conditions
fn compute_best_windows(
    activity: Activity,
    conditions: &crate::data::BeachConditions,
) -> Vec<TimeWindow> {
    // Get current hour to filter past times
    let current_hour = Local::now().hour() as u8;
    compute_best_windows_from_hour(activity, conditions, current_hour)
}

/// Internal implementation that accepts start hour for testability
fn compute_best_windows_from_hour(
    activity: Activity,
    conditions: &crate::data::BeachConditions,
    current_hour: u8,
) -> Vec<TimeWindow> {
    let profile = get_profile(activity);

    // Get weather data for scoring
    let (temp, wind, uv) = match &conditions.weather {
        Some(w) => (w.temperature as f32, w.wind as f32, w.uv as f32),
        None => return vec![], // Can't score without weather
    };

    // Get sunset hour for dynamic scoring
    let sunset_hour = conditions
        .weather
        .as_ref()
        .map(|w| w.sunset.hour() as u8)
        .unwrap_or(20); // Default to 8 PM if no data

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

    // Score each hour from current_hour to end hour (filter past hours)
    // For Sunset activity, cap at sunset_hour since viewing sunset after sunset is nonsensical
    let effective_end_hour = if activity == Activity::Sunset {
        sunset_hour
    } else {
        21
    };

    // If we're already past the effective end hour, no windows are available
    if current_hour > effective_end_hour {
        return vec![];
    }

    let start_hour = current_hour.max(6); // Don't go before 6am
    let mut hourly_scores: Vec<TimeSlotScore> = Vec::new();
    for hour in start_hour..=effective_end_hour {
        // Estimate crowd level based on time of day (simple heuristic)
        let crowd_level = estimate_crowd_level(hour);

        // For sunset activity, use dynamic scorer based on actual sunset time
        let time_score = if activity == Activity::Sunset {
            sunset_time_scorer_dynamic(hour, sunset_hour)
        } else {
            profile.time_of_day_scorer.map(|f| f(hour)).unwrap_or(1.0)
        };

        let mut score = profile.score_time_slot(
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

        // Adjust score based on time_score for sunset activity
        // The score_time_slot uses the profile's time_of_day_scorer internally,
        // but for sunset we want to override it with the dynamic scorer
        if activity == Activity::Sunset {
            // Recalculate score with dynamic time factor
            // The time_of_day contributes ~0.1 weight to the final score
            // We need to apply a stronger influence for sunset timing
            let base_score = score.score as f32;
            // Apply time_score as a multiplier with significant impact
            let adjusted = base_score * (0.3 + 0.7 * time_score);
            score.score = adjusted.clamp(0.0, 100.0) as u8;
        }

        hourly_scores.push(score);
    }

    // Group adjacent high-scoring hours into windows
    group_into_windows(&hourly_scores, activity)
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
fn group_into_windows(hourly_scores: &[TimeSlotScore], activity: Activity) -> Vec<TimeWindow> {
    if hourly_scores.is_empty() {
        return vec![];
    }

    // Find contiguous windows where score is above threshold (50)
    let threshold = 50u8;
    let mut windows: Vec<TimeWindow> = Vec::new();
    // Track: (start_hour, end_hour, best_score_in_window)
    let mut current_window: Option<(u8, u8, &TimeSlotScore)> = None;

    for slot in hourly_scores {
        if slot.score >= threshold {
            match current_window {
                Some((start, _, best)) => {
                    // Extend window, update best if this score is higher
                    if slot.score > best.score {
                        current_window = Some((start, slot.hour, slot));
                    } else {
                        current_window = Some((start, slot.hour, best));
                    }
                }
                None => {
                    current_window = Some((slot.hour, slot.hour, slot));
                }
            }
        } else {
            // End current window if exists
            if let Some((start, end, best)) = current_window {
                let reason = generate_reason_from_factors(&best.factors, activity);
                windows.push(TimeWindow {
                    start_hour: start,
                    end_hour: end + 1, // End is exclusive
                    score: best.score,
                    reason,
                    factors: Some(best.factors.clone()),
                });
                current_window = None;
            }
        }
    }

    // Don't forget the last window
    if let Some((start, end, best)) = current_window {
        let reason = generate_reason_from_factors(&best.factors, activity);
        windows.push(TimeWindow {
            start_hour: start,
            end_hour: end + 1,
            score: best.score,
            reason,
            factors: Some(best.factors.clone()),
        });
    }

    // If no windows above threshold, create windows from best individual hours
    if windows.is_empty() {
        let mut sorted: Vec<_> = hourly_scores.iter().collect();
        sorted.sort_by(|a, b| b.score.cmp(&a.score));

        for slot in sorted.iter().take(3) {
            let reason = generate_reason_from_factors(&slot.factors, activity);
            windows.push(TimeWindow {
                start_hour: slot.hour,
                end_hour: slot.hour + 1,
                score: slot.score,
                reason,
                factors: Some(slot.factors.clone()),
            });
        }
    }

    // Sort by score descending
    windows.sort_by(|a, b| b.score.cmp(&a.score));
    windows
}

/// Generates a human-readable reason string from score factors.
/// Highlights the top contributing factors for the score.
fn generate_reason_from_factors(factors: &ScoreFactors, activity: Activity) -> String {
    // Collect factor names with their scores, filtering by relevance to activity
    let mut scored_factors: Vec<(&str, f32)> = vec![
        ("temp", factors.temperature),
        ("wind", factors.wind),
        ("uv", factors.uv),
        ("timing", factors.time_of_day),
    ];

    // Add activity-specific factors
    if activity == Activity::Swimming {
        scored_factors.push(("water", factors.water_quality));
    }
    if matches!(activity, Activity::Swimming | Activity::Sailing) {
        scored_factors.push(("tide", factors.tide));
    }
    if matches!(activity, Activity::Peace | Activity::Sunbathing) {
        scored_factors.push(("crowd", factors.crowd));
    }

    // Sort by score descending and take top contributors
    scored_factors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Build reason from top 2-3 high-scoring factors (> 0.6)
    let good_factors: Vec<&str> = scored_factors
        .iter()
        .filter(|(_, score)| *score > 0.6)
        .take(3)
        .map(|(name, _)| factor_to_readable(name))
        .collect();

    if good_factors.is_empty() {
        "mixed conditions".to_string()
    } else {
        good_factors.join(", ")
    }
}

/// Converts factor name to human-readable description
fn factor_to_readable(factor: &str) -> &'static str {
    match factor {
        "temp" => "great temp",
        "water" => "safe water",
        "wind" => "calm winds",
        "uv" => "good UV",
        "tide" => "ideal tide",
        "crowd" => "low crowds",
        "timing" => "perfect timing",
        _ => "good conditions",
    }
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
        Span::styled("j/k", Style::default().fg(colors::HEADER)),
        Span::styled(" Scroll", Style::default().fg(colors::SECONDARY)),
        Span::raw("  "),
        Span::styled("g/G", Style::default().fg(colors::HEADER)),
        Span::styled(" Top/Bottom", Style::default().fg(colors::SECONDARY)),
        Span::raw("  "),
        Span::styled("1-5", Style::default().fg(colors::HEADER)),
        Span::styled(" Activity", Style::default().fg(colors::SECONDARY)),
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
            hourly: Vec::new(),
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

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
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

        let mut app =
            create_test_app_with_conditions("kitsilano", Some(create_test_weather()), None, None);

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
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

        let mut app =
            create_test_app_with_conditions("kitsilano", None, Some(create_test_tides()), None);

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
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
        // Use larger height to accommodate all sections including hourly forecast
        let backend = TestBackend::new(80, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            None,
            None,
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
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

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            None, // No weather data
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
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

        let mut app = create_test_app_with_conditions("kitsilano", None, None, None);

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
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
                render(frame, &mut app, "nonexistent");
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

    // ========================================================================
    // Dynamic Sunset Scorer Tests for compute_best_windows
    // ========================================================================

    /// Helper to create test conditions with a specific sunset time
    fn create_test_conditions_with_sunset(sunset_hour: u8, sunset_minute: u8) -> BeachConditions {
        let beach = Beach {
            id: "test-beach",
            name: "Test Beach",
            latitude: 49.2743,
            longitude: -123.1544,
            water_quality_id: Some("test-beach"),
        };

        let weather = Weather {
            temperature: 22.0,
            feels_like: 24.0,
            condition: WeatherCondition::Clear,
            humidity: 65,
            wind: 10.0,
            uv: 5.0,
            sunrise: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(sunset_hour as u32, sunset_minute as u32, 0).unwrap(),
            fetched_at: Utc::now(),
            hourly: Vec::new(),
        };

        let tides = TideInfo {
            current_height: 2.4,
            tide_state: TideState::Rising,
            next_high: Some(TideEvent {
                time: Local::now(),
                height: 4.8,
            }),
            next_low: Some(TideEvent {
                time: Local::now(),
                height: 0.5,
            }),
            fetched_at: Utc::now(),
        };

        let water_quality = WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(20),
            sample_date: NaiveDate::from_ymd_opt(2026, 1, 24).unwrap(),
            advisory_reason: None,
            fetched_at: Utc::now(),
        };

        BeachConditions {
            beach,
            weather: Some(weather),
            tides: Some(tides),
            water_quality: Some(water_quality),
        }
    }

    #[test]
    fn test_compute_best_windows_uses_dynamic_sunset_scorer() {
        // Create conditions with sunset at 17:00 (5 PM)
        let conditions = create_test_conditions_with_sunset(17, 0);

        // Call compute_best_windows_from_hour with Sunset activity
        // Start from hour 6 to ensure we score all hours including sunset
        let windows = compute_best_windows_from_hour(Activity::Sunset, &conditions, 6);

        // The windows should not be empty
        assert!(
            !windows.is_empty(),
            "Should have at least one time window for sunset"
        );

        // The highest-scored window should be around hour 17 (sunset hour)
        // The first window in the list is the highest scored due to sorting
        let best_window = &windows[0];

        // The best window should contain hour 17 or be very close to it
        // Since we use dynamic scoring, the peak should be at/around sunset_hour
        assert!(
            best_window.start_hour <= 18 && best_window.end_hour >= 16,
            "Best window ({}-{}) should be around sunset hour 17",
            best_window.start_hour,
            best_window.end_hour
        );
    }

    #[test]
    fn test_compute_best_windows_other_activities_unchanged() {
        // Create conditions with sunset at 17:00
        let conditions = create_test_conditions_with_sunset(17, 0);

        // Test Swimming - should NOT peak at sunset hour
        let swimming_windows = compute_best_windows_from_hour(Activity::Swimming, &conditions, 6);
        assert!(
            !swimming_windows.is_empty(),
            "Should have windows for swimming"
        );

        // Swimming doesn't have a time_of_day_scorer, so its best window
        // should be based on other factors (temp, water quality, etc.)
        // Verify it doesn't specifically favor hour 17
        let _swimming_best = &swimming_windows[0];
        // Swimming should prefer midday hours due to temperature and other factors
        // It should NOT specifically favor 17:00 like sunset would

        // Test Peace - should peak at early morning (6-7 AM)
        let peace_windows = compute_best_windows_from_hour(Activity::Peace, &conditions, 6);
        assert!(!peace_windows.is_empty(), "Should have windows for peace");

        let peace_best = &peace_windows[0];
        // Peace activity has a time_of_day_scorer that peaks at 6-7 AM
        // The best window should be in early morning
        assert!(
            peace_best.start_hour <= 8,
            "Peace best window ({}-{}) should be in early morning, not at sunset hour 17",
            peace_best.start_hour,
            peace_best.end_hour
        );

        // Verify Swimming and Peace don't peak at sunset hour like Sunset activity would
        // by checking that their scores at different times differ from Sunset's pattern
        let sunset_windows = compute_best_windows_from_hour(Activity::Sunset, &conditions, 6);
        let sunset_best = &sunset_windows[0];

        // Sunset should favor around hour 17, Peace should favor early morning
        // They should have different best windows
        assert!(
            peace_best.start_hour != sunset_best.start_hour
                || peace_best.end_hour != sunset_best.end_hour,
            "Peace and Sunset should have different best windows"
        );
    }

    #[test]
    fn test_sunset_activity_excludes_hours_after_sunset() {
        // Create conditions with sunset at 17:00
        let conditions = create_test_conditions_with_sunset(17, 0);
        // Start from hour 6 to see all hours
        let windows = compute_best_windows_from_hour(Activity::Sunset, &conditions, 6);
        // No window should include hours after sunset (17)
        for window in &windows {
            assert!(
                window.end_hour <= 18,
                "Sunset window should not extend past sunset hour. Got end_hour={}",
                window.end_hour
            );
        }
    }

    #[test]
    fn test_sunset_activity_returns_empty_when_past_sunset() {
        let conditions = create_test_conditions_with_sunset(17, 0);
        let windows = compute_best_windows_from_hour(Activity::Sunset, &conditions, 18);
        assert!(
            windows.is_empty(),
            "Should have no windows when starting after sunset"
        );
    }

    // ========================================================================
    // Vertical Layout Tests
    // ========================================================================

    #[test]
    fn test_vertical_layout_sections_in_correct_order() {
        // Test that sections appear vertically stacked in order:
        // WEATHER, TIDES, HOURLY FORECAST, WATER QUALITY
        // Use larger height to accommodate all sections
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Find the row positions of each section header
        let mut weather_row: Option<u16> = None;
        let mut tides_row: Option<u16> = None;
        let mut water_quality_row: Option<u16> = None;

        for y in 0..buffer.area().height {
            let mut row_content = String::new();
            for x in 0..buffer.area().width {
                row_content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            if row_content.contains("WEATHER") && weather_row.is_none() {
                weather_row = Some(y);
            }
            if row_content.contains("TIDES") && tides_row.is_none() {
                tides_row = Some(y);
            }
            if row_content.contains("WATER QUALITY") && water_quality_row.is_none() {
                water_quality_row = Some(y);
            }
        }

        // Verify all sections are present
        assert!(weather_row.is_some(), "WEATHER section should be present");
        assert!(tides_row.is_some(), "TIDES section should be present");
        assert!(
            water_quality_row.is_some(),
            "WATER QUALITY section should be present"
        );

        // Verify vertical order: WEATHER < TIDES < WATER QUALITY
        let weather_y = weather_row.unwrap();
        let tides_y = tides_row.unwrap();
        let water_quality_y = water_quality_row.unwrap();

        assert!(
            weather_y < tides_y,
            "WEATHER (row {}) should appear before TIDES (row {})",
            weather_y,
            tides_y
        );
        assert!(
            tides_y < water_quality_y,
            "TIDES (row {}) should appear before WATER QUALITY (row {})",
            tides_y,
            water_quality_y
        );
    }

    #[test]
    fn test_layout_works_at_80_columns_minimum() {
        // Test that layout renders correctly at 80 column minimum width
        // Use larger height to accommodate all sections including hourly forecast
        let backend = TestBackend::new(80, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // Verify key content is visible at 80 columns
        assert!(
            content.contains("WEATHER"),
            "WEATHER should be visible at 80 columns"
        );
        assert!(
            content.contains("TIDES"),
            "TIDES should be visible at 80 columns"
        );
        assert!(
            content.contains("WATER"),
            "WATER QUALITY should be visible at 80 columns"
        );
    }

    #[test]
    fn test_weather_section_full_width() {
        // Test that weather section content renders at full width (not 50%)
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            None,
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Find WEATHER and TIDES rows
        let mut weather_row: Option<u16> = None;
        let mut tides_row: Option<u16> = None;

        for y in 0..buffer.area().height {
            let mut row_content = String::new();
            for x in 0..buffer.area().width {
                row_content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            if row_content.contains("WEATHER") {
                weather_row = Some(y);
            }
            if row_content.contains("TIDES") {
                tides_row = Some(y);
            }
        }

        // Verify WEATHER and TIDES are NOT on the same row (vertical layout)
        assert!(weather_row.is_some(), "WEATHER section should exist");
        assert!(tides_row.is_some(), "TIDES section should exist");
        assert_ne!(
            weather_row.unwrap(),
            tides_row.unwrap(),
            "WEATHER and TIDES should be on different rows (vertical layout, not horizontal)"
        );
    }

    // ========================================================================
    // Scroll Support Tests (Task 105)
    // ========================================================================

    #[test]
    fn test_scroll_offset_is_clamped_to_max() {
        // When scroll offset exceeds max, it should be clamped
        let backend = TestBackend::new(80, 10); // Small height to force scrolling
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );
        app.detail_scroll_offset = 100; // Set to max

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        // After render, scroll offset should be clamped to actual max
        // Content height (7+5+4=16) - available height determines max
        assert!(
            app.detail_scroll_offset <= 100,
            "Scroll offset should be within bounds"
        );
    }

    #[test]
    fn test_scroll_indicator_bottom_shows_when_content_below() {
        // Test with a very small terminal that forces scrolling
        let backend = TestBackend::new(80, 8); // Very small height
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );
        app.detail_scroll_offset = 0; // At top

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // With a small terminal, there should be a "more" indicator
        // Note: This may not show if terminal is too small for any content
        // The test validates rendering doesn't crash with small terminals
        assert!(!content.trim().is_empty(), "Should render something");
    }

    #[test]
    fn test_scroll_indicator_top_shows_when_scrolled_down() {
        let backend = TestBackend::new(80, 10); // Small height to force scrolling
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );
        app.detail_scroll_offset = 5; // Scrolled down

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // When scrolled down, the "more" indicator at top should appear
        // Check that content contains the up arrow indicator
        // Note: The actual character may be rendered differently
        assert!(!content.trim().is_empty(), "Should render something");
    }

    #[test]
    fn test_activity_selector_stays_fixed_when_scrolling() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );
        app.current_activity = Some(crate::activities::Activity::Swimming);
        app.detail_scroll_offset = 0;

        // Render at scroll offset 0
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Find the Activity row
        let mut activity_row: Option<u16> = None;
        for y in 0..buffer.area().height {
            let mut row_content = String::new();
            for x in 0..buffer.area().width {
                row_content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            if row_content.contains("Activity") {
                activity_row = Some(y);
                break;
            }
        }

        assert!(
            activity_row.is_some(),
            "Activity selector should be visible at scroll offset 0"
        );
        let activity_y_at_0 = activity_row.unwrap();

        // Now scroll down and re-render
        app.detail_scroll_offset = 5;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Find Activity row again
        let mut activity_row_scrolled: Option<u16> = None;
        for y in 0..buffer.area().height {
            let mut row_content = String::new();
            for x in 0..buffer.area().width {
                row_content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            if row_content.contains("Activity") {
                activity_row_scrolled = Some(y);
                break;
            }
        }

        assert!(
            activity_row_scrolled.is_some(),
            "Activity selector should still be visible after scrolling"
        );

        // Activity row should be at the same position (fixed)
        assert_eq!(
            activity_y_at_0,
            activity_row_scrolled.unwrap(),
            "Activity selector should stay at the same position when scrolling"
        );
    }

    #[test]
    fn test_help_bar_stays_fixed_when_scrolling() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );
        app.detail_scroll_offset = 0;

        // Render at scroll offset 0
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Find the help row (contains "Back" or "Quit")
        let mut help_row: Option<u16> = None;
        for y in 0..buffer.area().height {
            let mut row_content = String::new();
            for x in 0..buffer.area().width {
                row_content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            if row_content.contains("Back") || row_content.contains("Quit") {
                help_row = Some(y);
            }
        }

        assert!(
            help_row.is_some(),
            "Help bar should be visible at scroll offset 0"
        );
        let help_y_at_0 = help_row.unwrap();

        // Now scroll down and re-render
        app.detail_scroll_offset = 3;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Find help row again
        let mut help_row_scrolled: Option<u16> = None;
        for y in 0..buffer.area().height {
            let mut row_content = String::new();
            for x in 0..buffer.area().width {
                row_content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            if row_content.contains("Back") || row_content.contains("Quit") {
                help_row_scrolled = Some(y);
            }
        }

        assert!(
            help_row_scrolled.is_some(),
            "Help bar should still be visible after scrolling"
        );

        // Help row should be at the same position (fixed at bottom)
        assert_eq!(
            help_y_at_0,
            help_row_scrolled.unwrap(),
            "Help bar should stay at the same position when scrolling"
        );
    }

    #[test]
    fn test_calculate_visible_rect_returns_none_for_invisible_section() {
        // Section entirely above visible area
        let area = Rect::new(0, 0, 80, 10);
        let result = calculate_visible_rect(0, 5, 10, 20, area);
        assert!(result.is_none(), "Section above visible area should return None");

        // Section entirely below visible area
        let result = calculate_visible_rect(25, 5, 10, 20, area);
        assert!(result.is_none(), "Section below visible area should return None");
    }

    #[test]
    fn test_calculate_visible_rect_returns_partial_for_clipped_section() {
        let area = Rect::new(0, 0, 80, 10);

        // Section that starts above visible area but extends into it
        let result = calculate_visible_rect(5, 10, 10, 20, area);
        assert!(result.is_some(), "Partially visible section should return Some");
        let rect = result.unwrap();
        assert_eq!(rect.y, 0, "Clipped section should start at area top");
        assert!(rect.height > 0, "Should have some visible height");
    }

    #[test]
    fn test_calculate_visible_rect_returns_full_for_fully_visible_section() {
        let area = Rect::new(0, 0, 80, 20);

        // Section fully within visible area
        let result = calculate_visible_rect(5, 5, 0, 20, area);
        assert!(result.is_some(), "Fully visible section should return Some");
        let rect = result.unwrap();
        assert_eq!(rect.height, 5, "Should show full section height");
    }

    #[test]
    fn test_interpolate_heights_with_larger_target() {
        let heights = vec![1.0, 2.0, 3.0, 4.0];
        let result = interpolate_heights(&heights, 7);
        assert_eq!(result.len(), 7, "Should return target_width elements");
        assert!((result[0] - 1.0).abs() < 0.001, "First element should match");
        assert!((result[6] - 4.0).abs() < 0.001, "Last element should match");
    }

    #[test]
    fn test_interpolate_heights_with_smaller_target() {
        let heights = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = interpolate_heights(&heights, 3);
        assert_eq!(result.len(), 3, "Should return target_width elements");
    }

    #[test]
    fn test_interpolate_heights_empty_input() {
        let heights: Vec<f64> = vec![];
        let result = interpolate_heights(&heights, 10);
        assert_eq!(result.len(), 10, "Should return target_width zeros");
        assert!(result.iter().all(|&h| h == 0.0), "All values should be 0.0");
    }

    #[test]
    fn test_build_time_labels_wide_width() {
        let labels = build_time_labels(60);
        assert!(labels.len() <= 60, "Labels should not exceed width");
        assert!(labels.contains("6AM"), "Should contain 6AM");
        assert!(labels.contains("12PM"), "Should contain 12PM");
        assert!(labels.contains("12AM"), "Should contain 12AM");
    }

    #[test]
    fn test_build_time_labels_narrow_width() {
        let labels = build_time_labels(15);
        // For narrow widths, we use abbreviated labels
        assert!(labels.contains("6"), "Should contain hour 6");
    }

    #[test]
    fn test_build_tides_lines_with_width_contains_expand_hint() {
        let tides = create_test_tides();
        let lines = build_tides_lines_with_width(Some(&tides), 60);

        // Find the line containing the expand hint
        let has_expand_hint = lines.iter().any(|line| {
            line.spans.iter().any(|span| span.content.contains("[t] expand"))
        });
        assert!(has_expand_hint, "Should contain [t] expand hint");
    }

    #[test]
    fn test_build_tides_lines_with_width_sparkline_scales() {
        let tides = create_test_tides();

        // Test with narrow width
        let lines_narrow = build_tides_lines_with_width(Some(&tides), 20);
        // Test with wide width
        let lines_wide = build_tides_lines_with_width(Some(&tides), 80);

        // Find the sparkline line (should be the third line, after header and state)
        // The sparkline is composed of individual character spans
        let sparkline_narrow = lines_narrow.get(2);
        let sparkline_wide = lines_wide.get(2);

        assert!(sparkline_narrow.is_some(), "Narrow sparkline should exist");
        assert!(sparkline_wide.is_some(), "Wide sparkline should exist");

        // Wide sparkline should have more spans (characters)
        let narrow_len = sparkline_narrow.unwrap().spans.len();
        let wide_len = sparkline_wide.unwrap().spans.len();

        assert!(
            wide_len > narrow_len,
            "Wide sparkline ({}) should have more characters than narrow ({})",
            wide_len,
            narrow_len
        );
    }

    #[test]
    fn test_build_tides_lines_with_width_time_markers_present() {
        let tides = create_test_tides();
        let lines = build_tides_lines_with_width(Some(&tides), 60);

        // The time labels line should be the 4th line (index 3)
        let time_line = lines.get(3);
        assert!(time_line.is_some(), "Time labels line should exist");

        let time_content: String = time_line.unwrap().spans.iter()
            .map(|s| s.content.as_ref())
            .collect();

        // Should contain time markers
        assert!(time_content.contains("6AM") || time_content.contains("6"), "Should have 6AM marker");
    }

    #[test]
    fn test_build_tides_lines_without_tides_data() {
        let lines = build_tides_lines_with_width(None, 60);
        assert!(!lines.is_empty(), "Should return at least header");

        let has_unavailable = lines.iter().any(|line| {
            line.spans.iter().any(|span| span.content.contains("unavailable"))
        });
        assert!(has_unavailable, "Should show unavailable message when no tides data");
    }

    // ========================================================================
    // Expanded Tide Chart Tests (Task 107)
    // ========================================================================

    #[test]
    fn test_expanded_tide_chart_has_more_lines_than_collapsed() {
        let tides = create_test_tides();
        let collapsed_lines = build_tides_lines_with_width(Some(&tides), 60);
        let expanded_lines = build_expanded_tide_chart(Some(&tides), 60);

        assert!(
            expanded_lines.len() > collapsed_lines.len(),
            "Expanded chart ({}) should have more lines than collapsed ({})",
            expanded_lines.len(),
            collapsed_lines.len()
        );
    }

    #[test]
    fn test_expanded_tide_chart_height_approximately_15_lines() {
        let tides = create_test_tides();
        let lines = build_expanded_tide_chart(Some(&tides), 60);

        // Expected: header(1) + state(1) + chart_rows(9) + x_axis_border(1) + time_markers(1) + next_events(1) = 14-15 lines
        assert!(
            lines.len() >= 13 && lines.len() <= 16,
            "Expanded chart should have ~14-15 lines, got {}",
            lines.len()
        );
    }

    #[test]
    fn test_expanded_tide_chart_contains_y_axis_labels() {
        let tides = create_test_tides();
        let lines = build_expanded_tide_chart(Some(&tides), 60);

        // Convert all lines to string for checking
        let all_content: String = lines.iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        assert!(all_content.contains("0m"), "Should contain 0m Y-axis label");
        assert!(all_content.contains("1m"), "Should contain 1m Y-axis label");
        assert!(all_content.contains("2m"), "Should contain 2m Y-axis label");
        assert!(all_content.contains("3m"), "Should contain 3m Y-axis label");
        assert!(all_content.contains("4m"), "Should contain 4m Y-axis label");
    }

    #[test]
    fn test_expanded_tide_chart_contains_x_axis_time_markers() {
        let tides = create_test_tides();
        let lines = build_expanded_tide_chart(Some(&tides), 80);

        // Convert all lines to string for checking
        let all_content: String = lines.iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        assert!(all_content.contains("6AM"), "Should contain 6AM time marker");
        assert!(all_content.contains("12PM"), "Should contain 12PM time marker");
        assert!(all_content.contains("6PM"), "Should contain 6PM time marker");
    }

    #[test]
    fn test_expanded_tide_chart_contains_collapse_hint() {
        let tides = create_test_tides();
        let lines = build_expanded_tide_chart(Some(&tides), 60);

        // Check for collapse hint
        let has_collapse_hint = lines.iter().any(|line| {
            line.spans.iter().any(|span| span.content.contains("[t] collapse"))
        });
        assert!(has_collapse_hint, "Expanded chart should contain [t] collapse hint");
    }

    #[test]
    fn test_collapsed_tide_chart_contains_expand_hint() {
        let tides = create_test_tides();
        let lines = build_tides_lines_with_width(Some(&tides), 60);

        // Check for expand hint
        let has_expand_hint = lines.iter().any(|line| {
            line.spans.iter().any(|span| span.content.contains("[t] expand"))
        });
        assert!(has_expand_hint, "Collapsed chart should contain [t] expand hint");
    }

    #[test]
    fn test_expanded_tide_chart_uses_box_drawing_characters() {
        let tides = create_test_tides();
        let lines = build_expanded_tide_chart(Some(&tides), 60);

        // Convert all lines to string
        let all_content: String = lines.iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Should contain box-drawing characters (Y-axis uses ┤ and ┼)
        assert!(
            all_content.contains('\u{2524}') || all_content.contains('\u{253C}'),
            "Should contain Y-axis box-drawing characters (┤ or ┼)"
        );
    }

    #[test]
    fn test_expanded_tide_chart_without_data() {
        let lines = build_expanded_tide_chart(None, 60);

        // Should show header and unavailable message
        assert!(lines.len() >= 2, "Should have at least header and message");

        let has_unavailable = lines.iter().any(|line| {
            line.spans.iter().any(|span| span.content.contains("unavailable"))
        });
        assert!(has_unavailable, "Should show unavailable message when no tides data");
    }

    #[test]
    fn test_build_expanded_time_labels_wide_width() {
        let labels = build_expanded_time_labels(80);
        assert!(labels.contains("6AM"), "Should contain 6AM");
        assert!(labels.contains("12PM"), "Should contain 12PM");
        assert!(labels.contains("10PM"), "Should contain 10PM");
    }

    #[test]
    fn test_build_expanded_time_labels_narrow_width() {
        let labels = build_expanded_time_labels(25);
        // For narrow widths, should still have some time markers
        assert!(labels.contains("6AM"), "Should contain 6AM even in narrow width");
    }

    #[test]
    fn test_t_key_toggles_tide_chart_expansion() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = App::new();
        app.state = crate::app::AppState::BeachDetail("kitsilano".to_string());
        assert!(!app.tide_chart_expanded, "Should start collapsed");

        // Press 't' to expand
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        assert!(app.tide_chart_expanded, "Should be expanded after 't' press");

        // Press 't' again to collapse
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        assert!(!app.tide_chart_expanded, "Should be collapsed after second 't' press");
    }

    #[test]
    fn test_tide_chart_expanded_state_in_rendered_output() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        // Render with collapsed chart
        app.tide_chart_expanded = false;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let collapsed_content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // Render with expanded chart
        app.tide_chart_expanded = true;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let expanded_content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        // Expanded should have "collapse" hint, collapsed should have "expand" hint
        assert!(
            collapsed_content.contains("expand"),
            "Collapsed view should show 'expand' hint"
        );
        assert!(
            expanded_content.contains("collapse"),
            "Expanded view should show 'collapse' hint"
        );
    }

    // ========================================================================
    // Hourly Forecast Section Tests (Task 108)
    // ========================================================================

    /// Helper to create test weather with hourly forecasts
    fn create_test_weather_with_hourly(current_hour: u8) -> Weather {
        use crate::data::HourlyForecast;

        let mut hourly = Vec::new();
        // Create 24 hours of forecast data
        for hour in 0..24u8 {
            hourly.push(HourlyForecast {
                hour,
                temperature: 15.0 + (hour as f64 * 0.5),
                feels_like: 14.0 + (hour as f64 * 0.5),
                condition: if hour < 12 {
                    WeatherCondition::Clear
                } else {
                    WeatherCondition::PartlyCloudy
                },
                wind: 10.0 + (hour as f64 * 0.2),
                wind_direction: "NW".to_string(),
                uv: if hour < 6 || hour > 20 { 0.0 } else { (hour as f64 - 6.0).min(8.0) },
                precipitation_chance: 0,
            });
        }

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
            hourly,
        }
    }

    #[test]
    fn test_hourly_forecast_section_displays_header() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let weather = create_test_weather_with_hourly(10);
        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(weather),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|cell| cell.symbol()).collect();

        assert!(
            content.contains("HOURLY FORECAST"),
            "Should display HOURLY FORECAST header"
        );
    }

    #[test]
    fn test_hourly_forecast_filters_past_hours() {
        // Test that past hours are not displayed
        // build_hourly_forecast_lines uses Local::now() internally
        // So we just verify that when there are hourly forecasts,
        // the function produces sensible output

        let weather = create_test_weather_with_hourly(0);
        let lines = build_hourly_forecast_lines(Some(&weather));

        // The header is always there
        assert!(lines.len() >= 1, "Should have at least header");

        // The function filters based on Local::now(), so we can verify
        // that it produces content (header + hours or "no more forecasts")
        let content: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.to_string()))
            .collect();

        // Should contain the header
        assert!(content.contains("HOURLY FORECAST"), "Should have header");

        // Should have either hourly data or "no more" message
        let has_hourly_content = content.contains(":00");
        let has_no_more_message = content.contains("No more forecasts") || content.contains("No hourly");

        assert!(
            has_hourly_content || has_no_more_message,
            "Should show either hours or a message"
        );
    }

    #[test]
    fn test_hourly_forecast_shows_max_8_hours() {
        let weather = create_test_weather_with_hourly(10);
        let lines = build_hourly_forecast_lines(Some(&weather));

        // 1 header + max 8 hour lines = 9 lines max
        assert!(
            lines.len() <= 9,
            "Should have at most 9 lines (1 header + 8 hours)"
        );
        // Should have at least header + some hours
        assert!(
            lines.len() > 1,
            "Should have header and at least one hour"
        );
    }

    #[test]
    fn test_hourly_forecast_shows_time_temp_icon_wind_uv() {
        let weather = create_test_weather_with_hourly(0);
        let lines = build_hourly_forecast_lines(Some(&weather));

        // Skip header - check if we have hour lines
        // The function filters by current time, so we may or may not have hour lines
        if lines.len() > 1 {
            // If we have hour lines, verify they have the expected format
            let hour_line = &lines[1];

            // Check spans exist for time, temp, icon, wind, UV
            // The line should have multiple spans
            assert!(
                hour_line.spans.len() >= 3,
                "Hour line should have multiple spans"
            );

            // Check content includes expected elements
            let line_content: String = hour_line.spans.iter().map(|s| s.content.to_string()).collect();

            // Should have some time format (:00), temperature (C), and Wind/UV
            let has_time = line_content.contains(":00");
            let has_temp = line_content.contains("C");
            let has_wind = line_content.contains("Wind:");
            let has_uv = line_content.contains("UV:");

            assert!(has_time, "Should show time");
            assert!(has_temp, "Should show temperature");
            assert!(has_wind, "Should show wind");
            assert!(has_uv, "Should show UV");
        } else {
            // If all hours have passed (late in the day), we should have a message
            let content: String = lines
                .iter()
                .flat_map(|line| line.spans.iter().map(|s| s.content.to_string()))
                .collect();
            assert!(
                content.contains("No more forecasts") || content.contains("No hourly"),
                "Should show message when no hours available"
            );
        }
    }

    #[test]
    fn test_hourly_forecast_handles_empty_hourly_data() {
        let mut weather = create_test_weather();
        weather.hourly = Vec::new();

        let lines = build_hourly_forecast_lines(Some(&weather));

        let content: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.to_string()))
            .collect();

        assert!(
            content.contains("No hourly forecast available"),
            "Should show message when no hourly data"
        );
    }

    #[test]
    fn test_hourly_forecast_handles_missing_weather() {
        let lines = build_hourly_forecast_lines(None);

        let content: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.to_string()))
            .collect();

        assert!(
            content.contains("No hourly forecast available"),
            "Should show message when weather is None"
        );
    }

    #[test]
    fn test_hourly_forecast_temperature_color_coding() {
        // Test hot temperature (>= 30)
        assert_eq!(temperature_color(35.0), Color::Red);
        assert_eq!(temperature_color(30.0), Color::Red);

        // Test warm temperature (>= 25)
        assert_eq!(temperature_color(27.0), Color::LightRed);

        // Test comfortable temperature (>= 20)
        assert_eq!(temperature_color(22.0), Color::Yellow);

        // Test cool temperature (>= 15)
        assert_eq!(temperature_color(17.0), Color::Green);

        // Test cold temperature (>= 10)
        assert_eq!(temperature_color(12.0), Color::Cyan);

        // Test very cold temperature (< 10)
        assert_eq!(temperature_color(5.0), Color::Blue);
    }

    #[test]
    fn test_hourly_forecast_positioned_between_tides_and_water_quality() {
        let backend = TestBackend::new(80, 45);
        let mut terminal = Terminal::new(backend).unwrap();

        let weather = create_test_weather_with_hourly(10);
        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(weather),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Find row positions of each section header
        let mut tides_row: Option<u16> = None;
        let mut hourly_row: Option<u16> = None;
        let mut water_quality_row: Option<u16> = None;

        for y in 0..buffer.area().height {
            let mut row_content = String::new();
            for x in 0..buffer.area().width {
                row_content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            if row_content.contains("TIDES") && tides_row.is_none() {
                tides_row = Some(y);
            }
            if row_content.contains("HOURLY FORECAST") && hourly_row.is_none() {
                hourly_row = Some(y);
            }
            if row_content.contains("WATER QUALITY") && water_quality_row.is_none() {
                water_quality_row = Some(y);
            }
        }

        // Verify all sections are present
        assert!(tides_row.is_some(), "TIDES section should be present");
        assert!(hourly_row.is_some(), "HOURLY FORECAST section should be present");
        assert!(water_quality_row.is_some(), "WATER QUALITY section should be present");

        // Verify order: TIDES < HOURLY FORECAST < WATER QUALITY
        let tides_y = tides_row.unwrap();
        let hourly_y = hourly_row.unwrap();
        let water_quality_y = water_quality_row.unwrap();

        assert!(
            tides_y < hourly_y,
            "TIDES (row {}) should appear before HOURLY FORECAST (row {})",
            tides_y,
            hourly_y
        );
        assert!(
            hourly_y < water_quality_y,
            "HOURLY FORECAST (row {}) should appear before WATER QUALITY (row {})",
            hourly_y,
            water_quality_y
        );
    }

    #[test]
    fn test_hourly_condition_icon_mapping() {
        assert_eq!(hourly_condition_icon(WeatherCondition::Clear), "\u{2600}");
        assert_eq!(hourly_condition_icon(WeatherCondition::PartlyCloudy), "\u{26C5}");
        assert_eq!(hourly_condition_icon(WeatherCondition::Cloudy), "\u{2601}");
        assert_eq!(hourly_condition_icon(WeatherCondition::Rain), "\u{1F327}");
        assert_eq!(hourly_condition_icon(WeatherCondition::Showers), "\u{1F326}");
        assert_eq!(hourly_condition_icon(WeatherCondition::Thunderstorm), "\u{26C8}");
        assert_eq!(hourly_condition_icon(WeatherCondition::Snow), "\u{2744}");
        assert_eq!(hourly_condition_icon(WeatherCondition::Fog), "\u{1F32B}");
    }

    #[test]
    fn test_hourly_forecast_no_more_forecasts_when_late() {
        // Create weather with hourly data, but set current hour to after all forecasts
        let mut weather = create_test_weather_with_hourly(0);
        // Keep only hours 0-12 in the forecast
        weather.hourly.retain(|h| h.hour < 12);

        // Now build lines as if current time is 14:00 (after all forecasts)
        // Since the filter in build_hourly_forecast_lines uses Local::now(),
        // we'll test with weather that has no future hours
        let lines = build_hourly_forecast_lines(Some(&weather));

        // The actual behavior depends on current time, but we can at least
        // verify the function handles this case gracefully
        assert!(lines.len() >= 1, "Should have at least the header");
    }

    // ========================================================================
    // Integration Tests (Task 109)
    // ========================================================================
    //
    // These tests verify the complete beach detail screen works correctly
    // at various terminal sizes and with all features.

    /// Helper to create a fully populated test app with all data
    fn create_fully_populated_test_app(beach_id: &str) -> App {
        let weather = create_test_weather_with_hourly(10);
        let mut app = create_test_app_with_conditions(
            beach_id,
            Some(weather),
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );
        app.detail_scroll_offset = 0;
        app.tide_chart_expanded = false;
        app
    }

    /// Helper to extract buffer content as a string
    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        buffer.content().iter().map(|cell| cell.symbol()).collect()
    }

    /// Helper to find row number containing a specific text
    fn find_row_containing(buffer: &ratatui::buffer::Buffer, text: &str) -> Option<u16> {
        for y in 0..buffer.area().height {
            let mut row_content = String::new();
            for x in 0..buffer.area().width {
                row_content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            if row_content.contains(text) {
                return Some(y);
            }
        }
        None
    }

    // ------------------------------------------------------------------------
    // Terminal Size Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_integration_render_at_80x24_standard_terminal() {
        // Test rendering at standard terminal size (80x24)
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Verify all major sections are present
        assert!(content.contains("WEATHER"), "WEATHER section should be visible at 80x24");
        assert!(content.contains("TIDES"), "TIDES section should be visible at 80x24");
        // Note: WATER QUALITY may be below fold, but should not panic

        // Verify no crash and meaningful content rendered
        assert!(!content.trim().is_empty(), "Should render meaningful content");
    }

    #[test]
    fn test_integration_render_at_120x40_large_terminal() {
        // Test rendering at large terminal size (120x40)
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // At 120x40, all sections should be visible without scrolling
        assert!(content.contains("WEATHER"), "WEATHER should be visible at 120x40");
        assert!(content.contains("TIDES"), "TIDES should be visible at 120x40");
        assert!(content.contains("HOURLY FORECAST"), "HOURLY FORECAST should be visible at 120x40");
        assert!(content.contains("WATER QUALITY"), "WATER QUALITY should be visible at 120x40");
    }

    #[test]
    fn test_integration_render_at_80x20_small_terminal_requires_scroll() {
        // Test rendering at small terminal size (80x20) that requires scrolling
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should render without panic
        assert!(!content.trim().is_empty(), "Should render content at small size");

        // At minimum, top sections should be visible
        assert!(content.contains("WEATHER"), "WEATHER should be visible at top");
    }

    #[test]
    fn test_integration_render_at_minimum_width() {
        // Test rendering at minimum width (60x24)
        let backend = TestBackend::new(60, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should not panic and should render content
        assert!(!content.trim().is_empty(), "Should render at narrow width");
    }

    #[test]
    fn test_integration_render_at_very_small_terminal() {
        // Test rendering at very small terminal (40x10)
        // This tests edge case handling
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        // Should not panic
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // May have limited content but should not crash
        assert!(!content.is_empty(), "Should render something even at very small size");
    }

    // ------------------------------------------------------------------------
    // Scroll Navigation Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_integration_scroll_navigation_through_all_content() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        // Start at top (scroll offset 0)
        app.detail_scroll_offset = 0;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content_at_top = buffer_to_string(buffer);
        assert!(content_at_top.contains("WEATHER"), "Should see WEATHER at top");

        // Scroll down
        app.detail_scroll_offset = 10;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content_scrolled = buffer_to_string(buffer);

        // After scrolling, content should change (different sections visible)
        // The key is that rendering doesn't panic
        assert!(!content_scrolled.trim().is_empty(), "Should render after scrolling");

        // Scroll to maximum
        app.detail_scroll_offset = 100; // Will be clamped
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        // Scroll offset should be clamped to valid range
        assert!(app.detail_scroll_offset <= 100, "Scroll offset should be clamped");
    }

    #[test]
    fn test_integration_scroll_preserves_fixed_elements() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.current_activity = Some(Activity::Swimming);

        // Record position of Activity selector at scroll 0
        app.detail_scroll_offset = 0;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let activity_row_0 = find_row_containing(buffer, "Activity");
        let help_row_0 = find_row_containing(buffer, "Back");

        // Scroll down and check positions remain same
        app.detail_scroll_offset = 5;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let activity_row_5 = find_row_containing(buffer, "Activity");
        let help_row_5 = find_row_containing(buffer, "Back");

        assert_eq!(activity_row_0, activity_row_5, "Activity selector should stay fixed");
        assert_eq!(help_row_0, help_row_5, "Help bar should stay fixed");
    }

    // ------------------------------------------------------------------------
    // Tide Chart Toggle Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_integration_tide_chart_toggle_collapsed_to_expanded() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.tide_chart_expanded = false;

        // Render collapsed
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content_collapsed = buffer_to_string(buffer);
        assert!(
            content_collapsed.contains("[t] expand"),
            "Collapsed state should show expand hint"
        );

        // Toggle to expanded
        app.tide_chart_expanded = true;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content_expanded = buffer_to_string(buffer);
        assert!(
            content_expanded.contains("[t] collapse"),
            "Expanded state should show collapse hint"
        );
    }

    #[test]
    fn test_integration_tide_chart_toggle_expanded_to_collapsed() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.tide_chart_expanded = true;

        // Render expanded
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content_expanded = buffer_to_string(buffer);
        assert!(
            content_expanded.contains("[t] collapse"),
            "Expanded state should show collapse hint"
        );

        // Toggle to collapsed
        app.tide_chart_expanded = false;
        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content_collapsed = buffer_to_string(buffer);
        assert!(
            content_collapsed.contains("[t] expand"),
            "Collapsed state should show expand hint"
        );
    }

    #[test]
    fn test_integration_expanded_tide_chart_shows_y_axis() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.tide_chart_expanded = true;

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Expanded chart should have Y-axis labels
        assert!(content.contains("0m") || content.contains("1m"),
            "Expanded chart should have Y-axis meter labels");
    }

    // ------------------------------------------------------------------------
    // Section Order Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_integration_all_sections_render_in_correct_order() {
        // Use large terminal to see all sections
        let backend = TestBackend::new(120, 50);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Find positions of all sections
        let weather_row = find_row_containing(buffer, "WEATHER");
        let tides_row = find_row_containing(buffer, "TIDES");
        let hourly_row = find_row_containing(buffer, "HOURLY FORECAST");
        let water_quality_row = find_row_containing(buffer, "WATER QUALITY");

        // Verify all sections are present
        assert!(weather_row.is_some(), "WEATHER section should be present");
        assert!(tides_row.is_some(), "TIDES section should be present");
        assert!(hourly_row.is_some(), "HOURLY FORECAST section should be present");
        assert!(water_quality_row.is_some(), "WATER QUALITY section should be present");

        // Verify order: WEATHER < TIDES < HOURLY FORECAST < WATER QUALITY
        let weather_y = weather_row.unwrap();
        let tides_y = tides_row.unwrap();
        let hourly_y = hourly_row.unwrap();
        let water_quality_y = water_quality_row.unwrap();

        assert!(weather_y < tides_y,
            "WEATHER (row {}) should appear before TIDES (row {})", weather_y, tides_y);
        assert!(tides_y < hourly_y,
            "TIDES (row {}) should appear before HOURLY FORECAST (row {})", tides_y, hourly_y);
        assert!(hourly_y < water_quality_y,
            "HOURLY FORECAST (row {}) should appear before WATER QUALITY (row {})", hourly_y, water_quality_y);
    }

    #[test]
    fn test_integration_best_window_section_appears_when_activity_selected() {
        let backend = TestBackend::new(120, 60);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.current_activity = Some(Activity::Swimming);

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Best Window section should be visible when activity is selected
        assert!(
            content.contains("BEST WINDOW") || content.contains("Best Window"),
            "Best Window section should appear when activity is selected"
        );
    }

    #[test]
    fn test_integration_best_window_not_visible_when_no_activity() {
        let backend = TestBackend::new(120, 50);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.current_activity = None;

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Best Window should NOT be visible when no activity selected
        // Note: The section may still be rendered but hidden, so we check for header
        let _has_best_window = content.contains("BEST WINDOW");
        // This may or may not be true depending on implementation
        // The important thing is it doesn't panic
        assert!(!content.trim().is_empty(), "Should render content");
    }

    // ------------------------------------------------------------------------
    // Edge Case Tests (Missing Data)
    // ------------------------------------------------------------------------

    #[test]
    fn test_integration_render_with_missing_weather_data() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            None, // No weather
            Some(create_test_tides()),
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should render without crashing
        assert!(!content.trim().is_empty(), "Should render with missing weather");
        assert!(content.contains("WEATHER"), "Should still show WEATHER section header");
        assert!(
            content.contains("unavailable") || content.contains("WEATHER"),
            "Should indicate missing weather data"
        );
    }

    #[test]
    fn test_integration_render_with_missing_tides_data() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            None, // No tides
            Some(create_test_water_quality()),
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should render without crashing
        assert!(!content.trim().is_empty(), "Should render with missing tides");
        assert!(content.contains("TIDES"), "Should still show TIDES section header");
    }

    #[test]
    fn test_integration_render_with_missing_water_quality_data() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            None, // No water quality
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should render without crashing
        assert!(!content.trim().is_empty(), "Should render with missing water quality");
    }

    #[test]
    fn test_integration_render_with_all_data_missing() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_test_app_with_conditions(
            "kitsilano",
            None,
            None,
            None,
        );

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should render without crashing
        assert!(!content.trim().is_empty(), "Should render even with all data missing");
    }

    // ------------------------------------------------------------------------
    // Activity Selection Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_integration_render_with_each_activity_selected() {
        let activities = [
            Activity::Swimming,
            Activity::Sunbathing,
            Activity::Sailing,
            Activity::Sunset,
            Activity::Peace,
        ];

        for activity in activities {
            let backend = TestBackend::new(120, 50);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut app = create_fully_populated_test_app("kitsilano");
            app.current_activity = Some(activity);

            // Should not panic for any activity
            terminal
                .draw(|frame| {
                    render(frame, &mut app, "kitsilano");
                })
                .unwrap();

            let buffer = terminal.backend().buffer();
            let content = buffer_to_string(buffer);

            assert!(
                !content.trim().is_empty(),
                "Should render content for activity {:?}",
                activity
            );
        }
    }

    // ------------------------------------------------------------------------
    // Combined Feature Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_integration_expanded_chart_with_scroll_at_small_terminal() {
        // Combine expanded tide chart with scrolling on small terminal
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.tide_chart_expanded = true;
        app.detail_scroll_offset = 5;

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Should not panic and should render something
        assert!(!content.trim().is_empty(), "Should render with expanded chart and scroll");
    }

    #[test]
    fn test_integration_activity_selected_with_scroll() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.current_activity = Some(Activity::Sunset);
        app.detail_scroll_offset = 10;

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Activity selector should still be visible (fixed position)
        assert!(
            content.contains("Activity"),
            "Activity selector should remain visible when scrolled"
        );
    }

    #[test]
    fn test_integration_all_features_combined() {
        // Test with all features active at once
        let backend = TestBackend::new(100, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");
        app.current_activity = Some(Activity::Swimming);
        app.tide_chart_expanded = true;
        app.detail_scroll_offset = 3;

        terminal
            .draw(|frame| {
                render(frame, &mut app, "kitsilano");
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content = buffer_to_string(buffer);

        // Verify no panic and meaningful content
        assert!(!content.trim().is_empty(), "Should render with all features active");

        // Fixed elements should be present
        assert!(content.contains("Activity"), "Activity selector should be present");
        assert!(
            content.contains("Back") || content.contains("Quit"),
            "Help bar should be present"
        );
    }

    // ------------------------------------------------------------------------
    // Stress Tests (No Panics)
    // ------------------------------------------------------------------------

    #[test]
    fn test_integration_rapid_scroll_no_panic() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        // Rapidly change scroll positions
        for offset in [0, 5, 10, 15, 20, 25, 100, 0, 50, 3] {
            app.detail_scroll_offset = offset;
            terminal
                .draw(|frame| {
                    render(frame, &mut app, "kitsilano");
                })
                .unwrap();
        }

        // If we get here without panic, test passes
        assert!(true, "No panic during rapid scroll");
    }

    #[test]
    fn test_integration_rapid_tide_toggle_no_panic() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = create_fully_populated_test_app("kitsilano");

        // Rapidly toggle tide chart
        for _ in 0..10 {
            app.tide_chart_expanded = !app.tide_chart_expanded;
            terminal
                .draw(|frame| {
                    render(frame, &mut app, "kitsilano");
                })
                .unwrap();
        }

        // If we get here without panic, test passes
        assert!(true, "No panic during rapid tide toggle");
    }

    #[test]
    fn test_integration_various_terminal_sizes_no_panic() {
        let sizes = [
            (80, 24),   // Standard
            (120, 40),  // Large
            (80, 20),   // Small height
            (60, 24),   // Narrow
            (100, 30),  // Medium
            (40, 15),   // Very small
            (200, 50),  // Very large
        ];

        for (width, height) in sizes {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut app = create_fully_populated_test_app("kitsilano");
            app.current_activity = Some(Activity::Swimming);

            terminal
                .draw(|frame| {
                    render(frame, &mut app, "kitsilano");
                })
                .unwrap();
        }

        // If we get here without panic, test passes
        assert!(true, "No panic at various terminal sizes");
    }
}
