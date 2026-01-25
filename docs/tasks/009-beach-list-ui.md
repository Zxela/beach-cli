---
id: "009"
title: "Implement beach list screen UI"
status: pending
depends_on: ["008"]
test_file: src/ui/beach_list.rs
---

# 009: Implement beach list screen UI

## Objective

Create the main beach list screen showing all beaches with temperature, weather icon, and water quality status.

## Acceptance Criteria

- [ ] Create `src/ui/mod.rs` with module structure
- [ ] Create `src/ui/beach_list.rs` with render function
- [ ] Display bordered box with title "Vancouver Beaches"
- [ ] List all beaches with: name, temperature, weather icon, water status icon
- [ ] Highlight currently selected beach (cursor indicator + different color)
- [ ] Show help text at bottom: "↑/↓ Navigate  Enter Select  r Refresh  q Quit"
- [ ] Handle missing data gracefully (show "--" or "?" for unavailable)
- [ ] Color coding: temp by warmth, water status (green/yellow/red/gray)

## Technical Notes

From WIREFRAMES.md beach list layout:
```
╭─ Vancouver Beaches ─────────────────────────────╮
│ ▸ Kitsilano Beach         22°C  🌤️   🟢       │
│   English Bay Beach       21°C  ☀️   🟢       │
```

Weather icons (Unicode):
- Clear: ☀️
- PartlyCloudy: 🌤️
- Cloudy: ☁️
- Rain: 🌧️
- etc.

Water status icons:
- Safe: 🟢
- Advisory: 🟡
- Closed: 🔴
- Unknown: ⚪

## Test Requirements

Add tests in `src/ui/beach_list.rs`:
- Test render produces non-empty buffer
- Test selected item is highlighted
- Test missing weather shows placeholder
- Test all beaches are rendered
