---
id: "101"
title: "Add HourlyForecast data model"
status: pending
depends_on: []
test_file: src/data/weather.rs
---

# 101: Add HourlyForecast data model

## Objective

Create the `HourlyForecast` struct and extend the `Weather` struct to include hourly forecast data. This provides the data foundation for the hourly forecast UI section.

## Acceptance Criteria

- [ ] `HourlyForecast` struct created with fields: hour, temperature, feels_like, condition, wind, wind_direction, uv, precipitation_chance
- [ ] `Weather` struct extended with `hourly: Vec<HourlyForecast>` field
- [ ] Default value for `hourly` is empty vec (backwards compatibility)
- [ ] Serde serialization/deserialization works correctly
- [ ] Existing tests pass without modification

## Technical Notes

From TECHNICAL_DESIGN.md:
```rust
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
```

## Test Requirements

Add to existing weather tests in `src/data/weather.rs`:
- Test `HourlyForecast` can be created and serialized
- Test `Weather` with empty hourly vec deserializes correctly
- Test `Weather` with populated hourly vec deserializes correctly
