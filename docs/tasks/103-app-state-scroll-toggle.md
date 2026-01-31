---
id: "103"
title: "Add app state for scroll and toggle"
status: pending
depends_on: []
test_file: src/app.rs
---

# 103: Add app state for scroll and toggle

## Objective

Add new fields to the `App` struct to track scroll position in beach detail view and tide chart expansion state.

## Acceptance Criteria

- [ ] `detail_scroll_offset: u16` field added to App (default 0)
- [ ] `tide_chart_expanded: bool` field added to App (default false)
- [ ] Fields initialized correctly in `App::new()`
- [ ] State resets appropriately when navigating away from detail view

## Technical Notes

From TECHNICAL_DESIGN.md:
```rust
pub struct App {
    // ... existing fields ...
    pub detail_scroll_offset: u16,
    pub tide_chart_expanded: bool,
}
```

Consider adding helper methods:
- `scroll_up()` / `scroll_down()` with bounds checking
- `toggle_tide_chart()`
- `reset_detail_view_state()` called when leaving detail view

## Test Requirements

Add tests to `src/app.rs`:
- Test initial state values
- Test scroll offset bounds (can't go negative, capped at max)
- Test tide chart toggle
- Test state reset on navigation
