# Technical Design: Fix Sunset Suggestions

## Overview

This document describes the technical changes needed to fix sunset activity suggestions so they:
1. Use the actual sunset time from weather data
2. Don't recommend past time windows

## Architecture

### Current Flow

```
User selects Sunset activity
        |
        v
render_best_window_section()
        |
        v
compute_best_windows(activity, conditions)
        |
        v
For hour in 6..=21:
    profile.score_time_slot(hour, ...)
        |
        v
    sunset_time_scorer(hour)  <-- Static hours (18-20)
        |
        v
group_into_windows()
        |
        v
Display top 3 windows  <-- No filtering of past hours
```

### Proposed Flow

```
User selects Sunset activity
        |
        v
render_best_window_section()
        |
        v
compute_best_windows(activity, conditions)
        |
        v
Extract current_hour and sunset_hour from conditions
        |
        v
For hour in current_hour..=21:  <-- Filter past hours
    profile.score_time_slot(hour, ..., sunset_hour)
        |
        v
    sunset_time_scorer_dynamic(hour, sunset_hour)  <-- Dynamic!
        |
        v
group_into_windows()
        |
        v
Display top 3 windows (or "Best times passed" message)
```

## Code Changes

### 1. src/activities.rs

Add a new dynamic sunset scorer:

```rust
/// Dynamic time-of-day scorer for sunset activities.
/// Peaks at sunset_hour - 1 (golden hour) and sunset_hour.
///
/// # Arguments
/// * `hour` - The hour being scored (0-23)
/// * `sunset_hour` - The hour when sunset occurs
pub fn sunset_time_scorer_dynamic(hour: u8, sunset_hour: u8) -> f32 {
    let diff = (hour as i16 - sunset_hour as i16).abs();
    match diff {
        0 => 1.0,      // Sunset hour
        1 => 0.9,      // 1 hour before/after
        2 => 0.5,      // 2 hours before/after
        3 => 0.2,      // 3 hours before/after
        _ => 0.1,      // Too far from sunset
    }
}
```

### 2. src/ui/beach_detail.rs

Modify `compute_best_windows`:

```rust
fn compute_best_windows(
    activity: Activity,
    conditions: &crate::data::BeachConditions,
) -> Vec<TimeWindow> {
    // ... existing setup code ...

    // Get current hour to filter past times
    let current_hour = Local::now().hour() as u8;

    // Get sunset hour for dynamic scoring
    let sunset_hour = conditions.weather.as_ref()
        .map(|w| w.sunset.hour() as u8)
        .unwrap_or(20); // Default to 8 PM if no data

    // Score each hour from current_hour to 9pm (filter past hours)
    let mut hourly_scores: Vec<(u8, u8)> = Vec::new();
    let start_hour = current_hour.max(6); // Don't go before 6am

    for hour in start_hour..=21 {
        let crowd_level = estimate_crowd_level(hour);

        // For sunset activity, use dynamic scorer
        let time_score = if activity == Activity::Sunset {
            sunset_time_scorer_dynamic(hour, sunset_hour)
        } else {
            profile.time_of_day_scorer.map(|f| f(hour)).unwrap_or(1.0)
        };

        // ... rest of scoring with time_score factored in ...
    }

    // ... rest of function ...
}
```

Add handling for "no windows" case in `render_best_window_section`:

```rust
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
}
```

## Data Models

No changes to existing data models. The `Weather` struct already contains:

```rust
pub struct Weather {
    // ...
    pub sunset: NaiveTime,
    // ...
}
```

## API Contracts

No external API changes. This is an internal fix.

## Dependencies

- `chrono::Local` - Already imported for time comparisons
- No new dependencies required

## Security Considerations

None. This change only affects display logic.

## Testing Strategy

### Unit Tests

1. **test_sunset_time_scorer_dynamic_peaks_at_sunset_hour**
   - Verify score is 1.0 at sunset hour
   - Verify score decreases as hours move away from sunset

2. **test_sunset_time_scorer_dynamic_with_early_sunset**
   - Test with sunset at 17:00 (winter)
   - Verify hour 17 scores highest

3. **test_sunset_time_scorer_dynamic_with_late_sunset**
   - Test with sunset at 21:00 (summer)
   - Verify hour 21 scores highest

4. **test_compute_best_windows_filters_past_hours**
   - Mock current time to 19:00
   - Verify no windows returned for hours < 19

5. **test_compute_best_windows_shows_all_passed_message**
   - Mock current time to 22:00
   - Verify appropriate message displayed

### Integration Tests

1. Test full render of beach detail with sunset activity at various times
2. Verify backward compatibility: other activities still score correctly
