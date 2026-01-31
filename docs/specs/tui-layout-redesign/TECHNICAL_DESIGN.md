# Technical Design: TUI Layout Redesign

## Overview

Restructure the beach detail screen from horizontal Weather|Tides split to vertical stacking, add expandable tide chart, hourly forecast display, and scrollable content support.

## Architecture

### Component Structure

```
BeachDetailScreen
├── ActivitySelector (fixed, 1 line)
├── ScrollableContent
│   ├── WeatherSection (full width)
│   ├── TidesSection (full width, expandable)
│   ├── HourlyForecastSection (new)
│   ├── WaterQualitySection (full width)
│   └── BestWindowSection (conditional)
└── HelpBar (fixed, 2 lines)
```

### State Changes

```rust
// In app.rs - Add to App struct
pub struct App {
    // ... existing fields ...

    /// Scroll offset for beach detail view
    pub detail_scroll_offset: u16,

    /// Whether tide chart is expanded
    pub tide_chart_expanded: bool,
}
```

## Data Models

### Hourly Forecast

```rust
// In data/weather.rs - Add hourly forecast support

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyForecast {
    pub hour: u8,  // 0-23
    pub temperature: f64,
    pub feels_like: f64,
    pub condition: WeatherCondition,
    pub wind: f64,
    pub wind_direction: String,
    pub uv: f64,
    pub precipitation_chance: u8,
}

// Extend Weather struct
pub struct Weather {
    // ... existing fields ...

    /// Hourly forecasts for today
    pub hourly: Vec<HourlyForecast>,
}
```

### API Integration

The Open-Meteo API already provides hourly data. Update `fetch_weather()` to parse and store hourly forecasts:

```rust
// Parse hourly data from API response
let hourly = response["hourly"]["time"]
    .as_array()
    .zip(response["hourly"]["temperature_2m"].as_array())
    // ... map to HourlyForecast structs
```

## UI Implementation

### Layout Structure

```rust
// In ui/beach_detail.rs

pub fn render(frame: &mut Frame, app: &App, beach_id: &str) {
    let area = frame.area();

    // Fixed header and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Activity selector
            Constraint::Min(10),    // Scrollable content area
            Constraint::Length(2),  // Help bar
        ])
        .split(area);

    render_activity_selector(frame, chunks[0], app.current_activity);
    render_scrollable_content(frame, chunks[1], app, beach_id);
    render_help_bar(frame, chunks[2]);
}

fn render_scrollable_content(frame: &mut Frame, area: Rect, app: &App, beach_id: &str) {
    // Calculate total content height
    let weather_height = 7;
    let tides_height = if app.tide_chart_expanded { 12 } else { 5 };
    let forecast_height = 9;  // Header + 6-8 hours
    let water_quality_height = 4;
    let best_window_height = if app.current_activity.is_some() { 6 } else { 0 };

    let total_height = weather_height + tides_height + forecast_height
                     + water_quality_height + best_window_height;

    // Render with scroll offset
    let visible_height = area.height;
    let max_scroll = total_height.saturating_sub(visible_height);
    let scroll_offset = app.detail_scroll_offset.min(max_scroll);

    // Create virtual canvas and render sections
    // ... render each section at calculated Y positions ...

    // Show scroll indicators if content exceeds view
    if scroll_offset > 0 {
        render_scroll_indicator_top(frame, area);
    }
    if scroll_offset < max_scroll {
        render_scroll_indicator_bottom(frame, area);
    }
}
```

### Expanded Tide Chart

```rust
fn render_tides_section(
    frame: &mut Frame,
    area: Rect,
    tides: Option<&TideInfo>,
    expanded: bool,
) {
    if expanded {
        render_tide_graph(frame, area, tides);
    } else {
        render_tide_sparkline(frame, area, tides);
    }
}

fn render_tide_graph(frame: &mut Frame, area: Rect, tides: Option<&TideInfo>) {
    // ASCII graph with box-drawing characters
    //
    // TIDES                              [t] collapse
    // 4m ┤            ╭───╮
    // 3m ┤          ╭─╯   ╰─╮
    // 2m ┤        ╭─╯       ╰─╮
    // 1m ┤  ●───╭─╯           ╰───
    // 0m ┼──┴────┴────┴────┴────┴──
    //     6AM   9AM  12PM  3PM  6PM  9PM
    //
    // ● = current time, H:14:32 (3.2m)  L:20:15 (0.8m)

    let heights = tides.map(|t| t.hourly_heights(4.8)).unwrap_or_default();
    let max_height = 4.8;
    let graph_height = 5;  // 0m to 4m
    let graph_width = area.width.saturating_sub(6) as usize;  // Leave room for Y-axis

    // Build graph lines from heights data
    // ... implementation details ...
}
```

### Hourly Forecast Section

```rust
fn render_hourly_forecast(
    frame: &mut Frame,
    area: Rect,
    hourly: &[HourlyForecast],
) {
    let current_hour = Local::now().hour() as u8;

    let mut lines = vec![Line::from(Span::styled(
        "HOURLY FORECAST",
        Style::default().fg(Color::Cyan).bold(),
    ))];

    // Filter to upcoming hours, take 6-8
    let upcoming: Vec<_> = hourly
        .iter()
        .filter(|h| h.hour >= current_hour)
        .take(8)
        .collect();

    for forecast in upcoming {
        let icon = weather_icon(forecast.condition);
        let line = Line::from(vec![
            Span::styled(format!("{:02}:00", forecast.hour), Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled(format!("{:>3}°C", forecast.temperature as i32), Style::default().fg(temperature_color(forecast.temperature))),
            Span::raw("  "),
            Span::raw(icon),
            Span::raw("  "),
            Span::styled(format!("Wind: {:>2}km/h", forecast.wind as i32), Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(format!("UV: {}", forecast.uv as i32), Style::default().fg(uv_color(forecast.uv))),
        ]);
        lines.push(line);
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}
```

## Key Bindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down (in detail view) |
| `k` / `↑` | Scroll up (in detail view) |
| `t` | Toggle tide chart expansion |
| `g` | Jump to top of content |
| `G` | Jump to bottom of content |

## Testing Strategy

### Unit Tests
- `test_scroll_offset_bounds` — Verify scroll doesn't exceed content
- `test_tide_chart_toggle` — Verify expansion state toggles correctly
- `test_hourly_forecast_filtering` — Verify past hours are excluded

### Integration Tests
- Render beach detail at various terminal sizes (80x24, 120x40, 80x20)
- Verify all sections visible via scrolling
- Verify tide chart renders correctly in both modes

### Manual Testing
- Test on actual terminal with real API data
- Verify smooth scrolling experience
- Test keyboard navigation flow

## Migration

1. Update `Weather` struct with `hourly` field (default to empty vec for backwards compat)
2. Update weather API client to parse hourly data
3. Add new app state fields (`detail_scroll_offset`, `tide_chart_expanded`)
4. Refactor `beach_detail.rs` layout from horizontal to vertical
5. Add new render functions for expanded tide and hourly forecast
6. Update key handling for scroll and toggle
7. Update tests

## Dependencies

No new crate dependencies required. Uses existing:
- `ratatui` for TUI rendering
- `chrono` for time handling
- Existing weather/tide data structures
