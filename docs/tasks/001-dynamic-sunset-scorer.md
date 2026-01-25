---
id: "001"
title: "Add dynamic sunset time scorer function"
status: pending
depends_on: []
test_file: src/activities.rs (inline tests)
---

# 001: Add dynamic sunset time scorer function

## Objective

Create a new `sunset_time_scorer_dynamic` function in `src/activities.rs` that scores hours based on the actual sunset time rather than hard-coded hours (18-20).

## Acceptance Criteria

- [ ] New function `sunset_time_scorer_dynamic(hour: u8, sunset_hour: u8) -> f32` added
- [ ] Score is 1.0 at sunset_hour (peak)
- [ ] Score is 0.9 at sunset_hour ± 1 (golden hour before, twilight after)
- [ ] Score decreases further from sunset (0.5 at ±2, 0.2 at ±3, 0.1 beyond)
- [ ] Function is public and documented
- [ ] Existing `sunset_time_scorer` function remains unchanged (backward compat)

## Technical Notes

From TECHNICAL_DESIGN.md:
```rust
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

## Test Requirements

Add to existing `#[cfg(test)] mod tests` in `src/activities.rs`:

1. `test_sunset_time_scorer_dynamic_peaks_at_sunset_hour`
   - Verify score is 1.0 when hour == sunset_hour
   - Test with sunset_hour = 17, 20, 21

2. `test_sunset_time_scorer_dynamic_scores_decrease_with_distance`
   - Verify hour ±1 from sunset scores 0.9
   - Verify hour ±2 from sunset scores 0.5
   - Verify hour ±3 from sunset scores 0.2
   - Verify hour ±4+ from sunset scores 0.1

3. `test_sunset_time_scorer_dynamic_with_early_sunset`
   - Test with sunset_hour = 17 (winter)
   - Verify hour 17 scores 1.0, hour 18 scores 0.9, etc.

4. `test_sunset_time_scorer_dynamic_with_late_sunset`
   - Test with sunset_hour = 21 (summer)
   - Verify hour 21 scores 1.0, hour 20 scores 0.9, etc.
