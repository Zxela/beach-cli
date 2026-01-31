---
id: "109"
title: "Add integration tests for layout"
status: pending
depends_on: ["105", "107", "108"]
test_file: src/ui/beach_detail.rs
---

# 109: Add integration tests for layout

## Objective

Add comprehensive integration tests verifying the complete beach detail screen works correctly at various terminal sizes and with all new features.

## Acceptance Criteria

- [ ] Test at 80x24 (standard terminal)
- [ ] Test at 120x40 (large terminal)
- [ ] Test at 80x20 (small terminal requiring scroll)
- [ ] Test scroll navigation through all content
- [ ] Test tide chart toggle in both directions
- [ ] Test all sections render in correct order
- [ ] Verify no panics or rendering issues

## Technical Notes

Use ratatui's `TestBackend` for rendering tests:
```rust
let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|frame| render(frame, &app, "kitsilano")).unwrap();
```

Test scenarios:
1. Full render with all data
2. Render with missing weather data
3. Render with tide chart expanded
4. Render at small terminal size with scroll
5. Render with activity selected (Best Window visible)

## Test Requirements

This task IS the test task. Create comprehensive tests covering:
- All terminal sizes mentioned
- All feature combinations
- Edge cases (missing data, end of day, etc.)
