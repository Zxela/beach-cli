---
id: "002"
title: "Define core data models and types"
status: pending
depends_on: ["001"]
test_file: src/data/mod.rs
---

# 002: Define core data models and types

## Objective

Create the data module with all core types: Beach, Weather, WeatherCondition, TideInfo, TideEvent, TideState, WaterQuality, WaterStatus, and BeachConditions.

## Acceptance Criteria

- [ ] Create `src/data/mod.rs` with module declarations
- [ ] `Beach` struct with id, name, latitude, longitude, water_quality_id
- [ ] `Weather` struct with temperature, feels_like, condition, humidity, wind, UV, sunrise/sunset, fetched_at
- [ ] `WeatherCondition` enum (Clear, PartlyCloudy, Cloudy, Rain, Showers, Thunderstorm, Snow, Fog)
- [ ] `TideInfo` struct with current_height, tide_state, next_high, next_low, fetched_at
- [ ] `TideEvent` struct with time, height
- [ ] `TideState` enum (Rising, Falling, High, Low)
- [ ] `WaterQuality` struct with status, ecoli_count, sample_date, advisory_reason, fetched_at
- [ ] `WaterStatus` enum (Safe, Advisory, Closed, Unknown)
- [ ] `BeachConditions` struct combining beach with optional weather/tides/water_quality
- [ ] All structs derive Serialize, Deserialize, Debug, Clone

## Technical Notes

From TECHNICAL_DESIGN.md data models section. Use:
- `chrono::NaiveTime` for sunrise/sunset
- `chrono::DateTime<Utc>` for fetched_at timestamps
- `chrono::DateTime<Local>` for tide event times
- `chrono::NaiveDate` for sample_date

## Test Requirements

Add tests in `src/data/mod.rs`:
- Test structs can be created with valid data
- Test serialization/deserialization round-trip for Weather
- Test enum variants are accessible
