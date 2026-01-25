---
id: "003"
title: "Add 'Best times have passed' message for empty windows"
status: pending
depends_on: ["002"]
test_file: src/ui/beach_detail.rs (inline tests)
---

# 003: Add "Best times have passed" message

## Objective

Update `render_best_window_section` in `src/ui/beach_detail.rs` to show a contextual message when no time windows are available, distinguishing between "no suitable windows" and "all times have passed for today".

## Acceptance Criteria

- [ ] When `windows.is_empty()` and `current_hour >= 21`, show "Best times have passed for today"
- [ ] When `windows.is_empty()` and `current_hour < 21`, show "No suitable time windows found"
- [ ] Message uses `colors::SECONDARY` style (consistent with existing UI)
- [ ] Existing "No suitable time windows found" case still works

## Technical Notes

From TECHNICAL_DESIGN.md, update the empty windows handling:

```rust
if windows.is_empty() {
    // Check if it's because all times passed
    let current_hour = Local::now().hour() as u8;
    if current_hour >= 21 {
        lines.push(Line::from(Span::styled(
            "Best times have passed for today",
            Style::default().fg(colors::SECONDARY),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "No suitable time windows found",
            Style::default().fg(colors::SECONDARY),
        )));
    }
}
```

## Test Requirements

Add test to `src/ui/beach_detail.rs`:

1. `test_render_best_window_section_shows_passed_message`
   - This is difficult to unit test without time mocking
   - Verify manually that the logic compiles and the message strings are correct
   - The existing render tests provide coverage for the happy path

Note: Since this is primarily a UI message change and time-dependent, mark as verified by:
- Code review of the conditional logic
- Manual testing at different times of day
- Existing render tests still pass
