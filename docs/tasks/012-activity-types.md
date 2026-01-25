---
id: "012"
title: "Define Activity enum and preference types"
status: pending
depends_on: []
test_file: src/activities.rs (inline tests)
---

# 012: Define Activity enum and preference types

## Objective

Create the foundational `Activity` enum and related preference types that will be used throughout the scoring engine and UI. This establishes the vocabulary for the entire feature.

## Acceptance Criteria

- [ ] `Activity` enum with 5 variants: Swimming, Sunbathing, Sailing, Sunset, Peace
- [ ] `Activity::all()` returns slice of all activities
- [ ] `Activity::label()` returns display string
- [ ] `Activity::from_str()` parses user input (case-insensitive, with aliases)
- [ ] `TidePreference` enum: High, Mid, Low, Any
- [ ] `UvPreference` enum: High, Moderate, Low, Any
- [ ] All types derive necessary traits (Debug, Clone, Copy, PartialEq, Eq)

## Technical Notes

From TECHNICAL_DESIGN.md:
- Create new file `src/activities.rs`
- Add `mod activities;` to `src/main.rs`
- String parsing should handle aliases: "swim" -> Swimming, "sun" -> Sunbathing, etc.

## Files to Create/Modify

- `src/activities.rs` (new)
- `src/main.rs` (add mod declaration)

## Test Requirements

Inline tests in `src/activities.rs`:
- Test `Activity::all()` returns 5 activities
- Test `Activity::label()` for each variant
- Test `Activity::from_str()` with valid inputs and aliases
- Test `Activity::from_str()` returns None for invalid input
- Test TidePreference and UvPreference derive traits
