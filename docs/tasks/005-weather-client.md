---
id: "005"
title: "Implement Open-Meteo weather API client"
status: pending
depends_on: ["002", "004"]
test_file: src/data/weather.rs
---

# 005: Implement Open-Meteo weather API client

## Objective

Create an async client to fetch current weather data from Open-Meteo API, returning typed Weather structs.

## Acceptance Criteria

- [ ] Create `src/data/weather.rs` with WeatherClient struct
- [ ] `fetch_weather(lat: f64, lon: f64) -> Result<Weather, WeatherError>`
- [ ] Parse Open-Meteo JSON response into Weather struct
- [ ] Map weather_code to WeatherCondition enum
- [ ] Handle API errors gracefully with custom WeatherError type
- [ ] Extract UV index from daily forecast
- [ ] Extract sunrise/sunset times

## Technical Notes

From TECHNICAL_DESIGN.md API contract:
```
GET https://api.open-meteo.com/v1/forecast
    ?latitude={lat}
    &longitude={lon}
    &current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m
    &daily=sunrise,sunset,uv_index_max
    &timezone=America/Vancouver
```

Weather code mapping (WMO codes):
- 0: Clear
- 1-3: PartlyCloudy
- 45, 48: Fog
- 51-55, 61-65, 80-82: Rain
- 56-57, 66-67: Showers
- 71-77, 85-86: Snow
- 95-99: Thunderstorm

## Test Requirements

Add tests in `src/data/weather.rs`:
- Test parsing valid Open-Meteo JSON response
- Test weather_code mapping to WeatherCondition
- Test error handling for malformed JSON
- Test error handling for missing fields
