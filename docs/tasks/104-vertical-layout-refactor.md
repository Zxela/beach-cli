---
id: "104"
title: "Refactor beach detail to vertical layout"
status: pending
depends_on: ["103"]
test_file: src/ui/beach_detail.rs
---

# 104: Refactor beach detail to vertical layout

## Objective

Change the beach detail screen layout from horizontal Weather|Tides split to vertical stacking. This is the core layout change that enables all other improvements.

## Acceptance Criteria

- [ ] Weather section renders at full width (not 50%)
- [ ] Tides section renders below Weather at full width
- [ ] Water Quality section remains full width
- [ ] Best Window section remains full width (when activity selected)
- [ ] All existing information is preserved
- [ ] Layout works at 80 columns minimum width
- [ ] Existing tests pass (may need adjustment for new structure)

## Technical Notes

From TECHNICAL_DESIGN.md, new layout structure:
```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(1),  // Activity selector
        Constraint::Min(10),    // Scrollable content area
        Constraint::Length(2),  // Help bar
    ])
    .split(area);
```

Key changes:
1. Remove horizontal split between Weather and Tides
2. Stack sections vertically in scrollable content area
3. Calculate section heights: weather(7), tides(5), water_quality(4), best_window(6)

## Test Requirements

Update tests in `src/ui/beach_detail.rs`:
- Test weather section renders (check for WEATHER header)
- Test tides section renders below weather
- Test all sections present in correct order
- Test layout at 80x24 terminal size
