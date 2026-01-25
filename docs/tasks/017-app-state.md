---
id: "017"
title: "Add PlanTrip state and activity tracking to App"
status: pending
depends_on: ["016"]
test_file: src/app.rs (inline tests)
---

# 017: Add PlanTrip state and activity tracking to App

## Objective

Extend the App struct and AppState enum to support the new PlanTrip screen and track the currently selected activity across screens.

## Acceptance Criteria

- [ ] `AppState::PlanTrip` variant added to enum
- [ ] `App.current_activity: Option<Activity>` field
- [ ] `App.plan_cursor: (usize, usize)` for grid navigation (beach, hour)
- [ ] `App.plan_time_range: (u8, u8)` for visible hour range (default 6, 21)
- [ ] Key binding 'p' in BeachList transitions to PlanTrip
- [ ] Key binding 'p' in BeachDetail transitions to PlanTrip
- [ ] Key bindings '1'-'5' in BeachDetail set current_activity
- [ ] Esc in PlanTrip returns to BeachList
- [ ] Activity persists when navigating between screens

## Technical Notes

From TECHNICAL_DESIGN.md:
```rust
pub enum AppState {
    Loading,
    BeachList,
    BeachDetail(String),
    PlanTrip,  // New
}

pub struct App {
    // Existing fields...
    pub current_activity: Option<Activity>,  // New
    pub plan_cursor: (usize, usize),         // New
    pub plan_time_range: (u8, u8),           // New
}
```

## Files to Modify

- `src/app.rs` - Add state variant, fields, key handlers

## Test Requirements

Inline tests in `src/app.rs`:
- Test 'p' key in BeachList transitions to PlanTrip
- Test 'p' key in BeachDetail transitions to PlanTrip
- Test '1'-'5' keys set current_activity correctly
- Test Esc in PlanTrip transitions to BeachList
- Test current_activity persists after screen transitions
- Test plan_cursor defaults to (0, 0)
- Test plan_time_range defaults to (6, 21)
