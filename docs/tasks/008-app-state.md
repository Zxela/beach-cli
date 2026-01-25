---
id: "008"
title: "Implement application state management"
status: pending
depends_on: ["003", "005", "006", "007"]
test_file: src/app.rs
---

# 008: Implement application state management

## Objective

Create the central App struct that manages UI state, coordinates data fetching, and handles user input.

## Acceptance Criteria

- [ ] Create `src/app.rs` with App struct
- [ ] `AppState` enum: Loading, BeachList, BeachDetail(beach_id)
- [ ] Track currently selected beach index in list
- [ ] Store fetched BeachConditions for all beaches
- [ ] `async fn load_all_data(&mut self)` - fetches all data concurrently
- [ ] `async fn refresh_beach(&mut self, beach_id: &str)` - refreshes single beach
- [ ] Handle keyboard input: Up/Down navigation, Enter to select, Esc to go back, q to quit
- [ ] `should_quit` flag for exit handling

## Technical Notes

Data loading strategy:
1. On startup, show Loading state
2. Fetch weather for all beaches concurrently (tokio::join!)
3. Fetch tides once (same for all beaches)
4. Fetch water quality for each beach concurrently
5. Transition to BeachList state

Use `tokio::select!` or similar for concurrent fetching with error handling per-source.

## Test Requirements

Add tests in `src/app.rs`:
- Test initial state is Loading
- Test state transitions: Loading -> BeachList -> BeachDetail -> BeachList
- Test navigation updates selected_index correctly
- Test selected_index wraps at boundaries
