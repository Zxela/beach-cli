---
id: "013"
title: "Add hourly weather forecast fetching"
status: pending
depends_on: ["012"]
test_file: src/data/weather.rs (inline tests)
---

# 013: Add hourly weather forecast fetching

## Objective

Extend the WeatherClient to fetch 48-hour hourly forecasts from Open-Meteo in addition to current conditions. This provides the time-series data needed for scoring future time slots.

## Acceptance Criteria

- [ ] `HourlyForecast` struct with: time, temperature, weather_code, wind_speed, uv_index
- [ ] `WeatherData` struct containing both current weather and hourly forecasts
- [ ] `WeatherClient::fetch_weather_with_hourly()` method returns `WeatherData`
- [ ] Hourly data covers next 48 hours
- [ ] Existing `fetch_weather()` continues to work unchanged
- [ ] Parse hourly arrays from Open-Meteo response

## Technical Notes

From TECHNICAL_DESIGN.md:
- Open-Meteo query adds: `&hourly=temperature_2m,weathercode,windspeed_10m,uv_index`
- Same endpoint, just different parameters
- Cache hourly data for 1 hour (weather changes frequently)

Open-Meteo hourly response structure:
```json
{
  "hourly": {
    "time": ["2026-01-25T00:00", "2026-01-25T01:00", ...],
    "temperature_2m": [5.2, 4.8, ...],
    "weathercode": [3, 3, ...],
    "windspeed_10m": [12.5, 11.2, ...],
    "uv_index": [0, 0, ...]
  }
}
```

## Files to Modify

- `src/data/weather.rs` - Add structs and fetch method
- `src/data/mod.rs` - Export new types

## Test Requirements

Inline tests in `src/data/weather.rs`:
- Test parsing valid hourly response JSON
- Test `HourlyForecast` fields are correctly extracted
- Test hourly array has expected length (~48 entries)
- Test existing `fetch_weather()` still works (backward compatibility)
- Test handling of missing hourly data gracefully
