---
id: "108"
title: "Add hourly forecast section"
status: pending
depends_on: ["102", "104"]
test_file: src/ui/beach_detail.rs
---

# 108: Add hourly forecast section

## Objective

Add a new UI section displaying hourly weather forecasts for the upcoming 6-8 hours, positioned between Tides and Water Quality sections.

## Acceptance Criteria

- [ ] "HOURLY FORECAST" header displayed
- [ ] Shows next 6-8 hours (until end of day)
- [ ] Each hour shows: time, temperature, condition icon, wind, UV
- [ ] Past hours are filtered out
- [ ] Temperature color-coded (existing temperature_color function)
- [ ] UV color-coded (existing uv_color function)
- [ ] Graceful handling when no hourly data available

## Technical Notes

From WIREFRAMES.md:
```
HOURLY FORECAST
14:00   23°C  ☀  Wind: 10km/h  UV: 6
15:00   24°C  ☀  Wind: 12km/h  UV: 5
16:00   23°C  ⛅  Wind: 15km/h  UV: 4
...
```

Section height: 1 (header) + 6-8 (hours) = 7-9 lines

Reuse existing helpers:
- `weather_icon()` for condition
- `temperature_color()` for temp coloring
- `uv_color()` / `uv_index_color()` for UV coloring

## Test Requirements

Add tests to `src/ui/beach_detail.rs`:
- Test section renders with header
- Test past hours are excluded
- Test shows up to 8 upcoming hours
- Test handles empty hourly data gracefully
