---
id: "015"
title: "Create crowd estimation heuristics module"
status: pending
depends_on: ["012"]
test_file: src/crowd.rs (inline tests)
---

# 015: Create crowd estimation heuristics module

## Objective

Create a new module with heuristic functions to estimate beach crowd levels based on time of day, day of week, and season. This replaces the need for real-time crowd data APIs.

## Acceptance Criteria

- [ ] New `src/crowd.rs` module
- [ ] `estimate_crowd(month: u32, weekday: Weekday, hour: u32) -> f32` function
- [ ] Returns value from 0.0 (empty) to 1.0 (packed)
- [ ] Summer weekends at peak hours return high values (>0.8)
- [ ] Winter weekday mornings return low values (<0.2)
- [ ] All returned values are clamped to 0.0-1.0 range

## Technical Notes

From TECHNICAL_DESIGN.md:
```rust
let season_factor = match month {
    6..=8 => 1.0,       // Summer - busiest
    5 | 9 => 0.6,       // Shoulder season
    4 | 10 => 0.3,      // Spring/fall
    _ => 0.1,           // Winter - minimal
};

let day_factor = match weekday {
    Weekday::Sat | Weekday::Sun => 1.0,
    Weekday::Fri => 0.7,
    _ => 0.4,
};

let hour_factor = match hour {
    12..=16 => 1.0,     // Peak afternoon
    10..=11 | 17..=18 => 0.7,
    8..=9 | 19..=20 => 0.4,
    6..=7 | 21 => 0.2,
    _ => 0.1,
};
```

Final crowd = season_factor * day_factor * hour_factor

## Files to Create/Modify

- `src/crowd.rs` (new)
- `src/main.rs` (add mod declaration)

## Test Requirements

Inline tests in `src/crowd.rs`:
- Test July Saturday 2pm returns > 0.8
- Test January Tuesday 7am returns < 0.2
- Test output is always in 0.0-1.0 range
- Test boundary conditions (month 1, 12; hour 0, 23)
- Test all weekdays produce valid output
