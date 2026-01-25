---
id: "010"
title: "Implement beach detail screen UI"
status: pending
depends_on: ["008"]
test_file: src/ui/beach_detail.rs
---

# 010: Implement beach detail screen UI

## Objective

Create the detailed beach view showing full weather, tide, and water quality information.

## Acceptance Criteria

- [ ] Create `src/ui/beach_detail.rs` with render function
- [ ] Display bordered box with beach name as title
- [ ] WEATHER section: condition icon, temperature, feels-like, wind, humidity, UV index, sunrise/sunset
- [ ] TIDES section: current state (Rising/Falling), current height, next high time/height, next low time/height
- [ ] WATER QUALITY section: status with icon, last test date, E. coli count
- [ ] Show help text: "← Back  r Refresh  q Quit"
- [ ] Color coding matching WIREFRAMES.md color scheme
- [ ] Handle missing data sections gracefully

## Technical Notes

From WIREFRAMES.md detail layout:
```
╭─ Kitsilano Beach ───────────────────────────────╮
│  WEATHER              │  TIDES                  │
│  🌤️  22°C (feels 24°) │  ↗ Rising              │
│  💨 12 km/h W         │  High: 2:34 PM         │
│  ☀️  UV: 6 (High)     │  Low:  8:45 PM         │
│                       │                         │
│  WATER QUALITY                                  │
│  🟢 Safe to swim                               │
│  Last tested: Jan 24  E.coli: 45 CFU/100mL    │
```

Layout using ratatui:
- Use Layout::horizontal for weather/tides split
- Use Block widgets for sections
- Use Paragraph for content

## Test Requirements

Add tests in `src/ui/beach_detail.rs`:
- Test render produces non-empty buffer
- Test weather section renders temperature
- Test tides section renders tide state
- Test water quality section renders status
- Test handles missing weather gracefully
