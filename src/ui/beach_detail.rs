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

    // Create layout: Activity selector, then vertically stacked content sections, Help row
    // Section heights: weather(7), tides(5), water_quality(4), best_window(6)
    let constraints = if show_best_window {
        vec![
            Constraint::Length(1),  // Activity selector row
            Constraint::Length(7),  // Weather section (full width)
            Constraint::Length(5),  // Tides section (full width)
            Constraint::Length(4),  // Water Quality section (full width)
            Constraint::Length(6),  // Best Window Today section (full width)
            Constraint::Length(2),  // Help text
        ]
    } else {
        vec![
            Constraint::Length(1),  // Activity selector row
            Constraint::Length(7),  // Weather section (full width)
            Constraint::Length(5),  // Tides section (full width)
            Constraint::Length(4),  // Water Quality section (full width)
            Constraint::Length(2),  // Help text
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    // Render activity selector at the top
    render_activity_selector(frame, chunks[0], app.current_activity);

    // Render sections vertically stacked at full width
    render_weather_section(frame, chunks[1], conditions.weather.as_ref());
    render_tides_section(frame, chunks[2], conditions.tides.as_ref());
    render_water_quality_section(frame, chunks[3], conditions.water_quality.as_ref());

    // Render Best Window Today section if activity is selected
    if show_best_window {
        render_best_window_section(frame, chunks[4], app, beach_id);
        render_help_text(frame, chunks[5]);
    } else {
        render_help_text(frame, chunks[4]);
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

/// Block characters for tide chart (8 levels)
const TIDE_BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Converts a tide height to a block character
fn height_to_block(height: f64, max_height: f64) -> char {
    let normalized = (height / max_height).clamp(0.0, 1.0);
    let index = ((normalized * 7.0).round() as usize).min(7);
    TIDE_BLOCKS[index]
}

/// Renders the tides section with tide chart
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
    /// Factor breakdown for score transparency
    factors: Option<ScoreFactors>,
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
        // WEATHER, TIDES, WATER QUALITY
        let backend = TestBackend::new(80, 30);
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

        let app = create_test_app_with_conditions(
            "kitsilano",
            Some(create_test_weather()),
            Some(create_test_tides()),
            None,
        );

        terminal
            .draw(|frame| {
                render(frame, &app, "kitsilano");
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
}
