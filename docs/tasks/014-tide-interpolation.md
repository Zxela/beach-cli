---
id: "014"
title: "Add tide height interpolation function"
status: pending
depends_on: ["012"]
test_file: src/data/tides.rs (inline tests)
---

# 014: Add tide height interpolation function

## Objective

Add a function to estimate tide height at any given hour by interpolating between known high/low tide points. This enables scoring time slots based on tide conditions.

## Acceptance Criteria

- [ ] `TidesClient::get_height_at_hour(hour: u8) -> Option<f32>` method
- [ ] Uses existing tide prediction data to interpolate
- [ ] Returns height in meters
- [ ] Works for hours between known tide events
- [ ] Returns None if no tide data available
- [ ] `TidesClient::get_max_tide_height() -> f32` for normalization

## Technical Notes

From TECHNICAL_DESIGN.md:
- Use linear interpolation between known high/low points
- Existing code already has cosine interpolation logic in `calculate_tide_state_and_height`
- Can reuse/adapt that logic for arbitrary hour queries
- Max tide height needed for normalizing scores (0.0-1.0 range)

The existing `TidePrediction` struct has:
- `date: NaiveDate`
- `time: NaiveTime`
- `height: f64`
- `is_high: bool`

## Files to Modify

- `src/data/tides.rs` - Add interpolation methods

## Test Requirements

Inline tests in `src/data/tides.rs`:
- Test interpolation at exact high tide time returns high tide height
- Test interpolation at exact low tide time returns low tide height
- Test interpolation at midpoint returns value between high and low
- Test returns None for dates without data
- Test `get_max_tide_height()` returns reasonable value (around 4.8m for Point Atkinson)
