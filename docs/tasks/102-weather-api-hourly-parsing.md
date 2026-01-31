---
id: "102"
title: "Parse hourly data from weather API"
status: pending
depends_on: ["101"]
test_file: src/data/weather.rs
---

# 102: Parse hourly data from weather API

## Objective

Update the weather API client to parse hourly forecast data from the Open-Meteo API response and populate the `Weather.hourly` field.

## Acceptance Criteria

- [ ] `fetch_weather()` parses hourly data from API response
- [ ] Hourly data includes temperature, feels_like, condition, wind, UV for each hour
- [ ] Only today's hours are included (filter by date)
- [ ] Graceful handling if hourly data is missing (default to empty vec)
- [ ] Cache includes hourly data

## Technical Notes

Open-Meteo API already provides hourly data in the response. The current implementation ignores it. Need to:
1. Parse `response["hourly"]["time"]` array
2. Parse corresponding data arrays (temperature_2m, apparent_temperature, weathercode, etc.)
3. Map weathercode to WeatherCondition enum
4. Filter to today's date only

## Test Requirements

Add tests to `src/data/weather.rs`:
- Test parsing of hourly data from mock API response
- Test empty hourly array handling
- Test date filtering (only today's hours)
