---
id: "020"
title: "Implement Plan Trip screen with heatmap grid"
status: pending
depends_on: ["016", "017"]
test_file: null
no_test_reason: "UI rendering - verified by manual testing"
---

# 020: Implement Plan Trip screen with heatmap grid

## Objective

Create the new Plan Trip screen with a heatmap grid showing scores across beaches (rows) and hours (columns). This is the main visual planning interface.

## Acceptance Criteria

- [ ] New `src/ui/plan_trip.rs` module
- [ ] Activity selector bar at top (same as detail screen)
- [ ] Heatmap grid with beaches as rows, hours as columns
- [ ] Cells show block characters colored by score (██ green, ▓▓ yellow, ░░ red)
- [ ] Cursor highlights current cell with brackets [ ]
- [ ] Navigation: h/←, l/→ for hours; j/↓, k/↑ for beaches
- [ ] "Best Recommendation" section at bottom
- [ ] "Selected Cell" section shows details of cursor position
- [ ] Legend showing score ranges
- [ ] Help bar with key bindings
- [ ] Enter on cell navigates to that beach's detail view

## Technical Notes

From WIREFRAMES.md:
```
╭─ Plan Your Trip ────────────────────────────────────────────────────╮
│  Activity: [●Swimming] [○Sunbathing] [○Sailing] [○Sunset] [○Peace] │
├─────────────────────────────────────────────────────────────────────┤
│              6am   8am  10am  12pm   2pm   4pm   6pm   8pm          │
│            ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐        │
│ Kitsilano  │ ░░  │ ▒▒  │ ▓▓  │[██] │ ██  │ ▓▓  │ ▒▒  │ ░░  │        │
│ English B  │ ░░  │ ▒▒  │ ▓▓  │ ██  │ ▓▓  │ ▓▓  │ ▒▒  │ ░░  │        │
...
│  Legend: ██ Excellent (80+)  ▓▓ Good (60-79)  ▒▒ Fair (40-59)      │
│          ░░ Poor (<40)       [ ] Cursor                             │
├─────────────────────────────────────────────────────────────────────┤
│  BEST RECOMMENDATION                                                │
│  Jericho Beach @ 10:00 AM  Score: 94/100                           │
│  SELECTED: Kitsilano @ 12:00 PM  Score: 88/100                     │
├─────────────────────────────────────────────────────────────────────┤
│  ←/h →/l Hours  ↑/k ↓/j Beaches  1-5 Activity  Enter Go  Esc Back  │
╰─────────────────────────────────────────────────────────────────────╯
```

Color mapping:
- 80-100: Green (Color::Green)
- 60-79: Light Green (Color::LightGreen)
- 40-59: Yellow (Color::Yellow)
- 20-39: Light Red (Color::LightRed)
- 0-19: Red (Color::Red)

## Files to Create/Modify

- `src/ui/plan_trip.rs` (new)
- `src/ui/mod.rs` - Export plan_trip module
- `src/app.rs` - Wire up rendering for PlanTrip state

## Verification

Manual testing:
1. Press 'p' from beach list
2. Verify grid renders with all beaches and hours 6am-8pm
3. Verify cursor moves with arrow keys / hjkl
4. Verify activity changes with 1-5 keys
5. Verify grid colors update when activity changes
6. Verify "Best" and "Selected" sections update
7. Verify Enter navigates to selected beach
8. Verify Esc returns to beach list
