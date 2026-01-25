---
id: "002"
title: "Filter past hours and use dynamic sunset scorer in compute_best_windows"
status: pending
depends_on: ["001"]
test_file: src/ui/beach_detail.rs (inline tests)
---

# 002: Filter past hours and use dynamic sunset scorer

## Objective

Modify `compute_best_windows` in `src/ui/beach_detail.rs` to:
1. Start scoring from the current hour (filter out past hours)
2. Use the dynamic sunset scorer for Sunset activity

## Acceptance Criteria

- [ ] `compute_best_windows` extracts current hour from `Local::now()`
- [ ] `compute_best_windows` extracts sunset hour from `conditions.weather.sunset`
- [ ] Scoring loop starts at `current_hour.max(6)` instead of fixed `6`
- [ ] For Sunset activity, uses `sunset_time_scorer_dynamic(hour, sunset_hour)`
- [ ] Other activities continue using their existing time_of_day_scorer
- [ ] All existing tests pass

## Technical Notes

From TECHNICAL_DESIGN.md, modify the scoring loop:

```rust
// Get current hour to filter past times
let current_hour = Local::now().hour() as u8;

// Get sunset hour for dynamic scoring
let sunset_hour = conditions.weather.as_ref()
    .map(|w| w.sunset.hour() as u8)
    .unwrap_or(20); // Default to 8 PM if no data

// Score each hour from current_hour to 9pm (filter past hours)
let start_hour = current_hour.max(6); // Don't go before 6am

for hour in start_hour..=21 {
    // ... existing code ...

    // For sunset activity, use dynamic scorer
    let time_score = if activity == Activity::Sunset {
        sunset_time_scorer_dynamic(hour, sunset_hour)
    } else {
        profile.time_of_day_scorer.map(|f| f(hour)).unwrap_or(1.0)
    };

    // ... factor time_score into the scoring ...
}
```

Need to import `sunset_time_scorer_dynamic` from `crate::activities`.

## Test Requirements

Add tests to `src/ui/beach_detail.rs`:

1. `test_compute_best_windows_uses_dynamic_sunset_scorer`
   - Create conditions with sunset at 17:00
   - Call compute_best_windows with Sunset activity
   - Verify the highest-scored window is around hour 17

2. `test_compute_best_windows_other_activities_unchanged`
   - Test Swimming, Peace activities still use their original scorers
   - Verify Swimming doesn't peak at sunset hour
   - Verify Peace still peaks at early morning

Note: Testing past-hour filtering is difficult without time mocking. Focus on the sunset scorer integration. The past-hour filtering is implicitly tested by the "all passed" message in task 003.
