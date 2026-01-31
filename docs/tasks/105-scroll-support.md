---
id: "105"
title: "Add scroll support with indicators"
status: pending
depends_on: ["103", "104"]
test_file: src/ui/beach_detail.rs
---

# 105: Add scroll support with indicators

## Objective

Implement vertical scrolling for the beach detail view to support smaller terminals and the increased content height from the new layout.

## Acceptance Criteria

- [ ] Content scrolls when total height exceeds visible area
- [ ] `j` / `↓` scrolls down one line
- [ ] `k` / `↑` scrolls up one line
- [ ] `g` jumps to top
- [ ] `G` jumps to bottom
- [ ] Scroll indicator "▲ more" shown when content above
- [ ] Scroll indicator "▼ more" shown when content below
- [ ] Activity selector and help bar remain fixed (not scrolled)

## Technical Notes

From TECHNICAL_DESIGN.md:
- Track `scroll_offset` in app state (task 103)
- Calculate `max_scroll = total_height - visible_height`
- Clamp scroll offset to valid range
- Render scroll indicators at top/bottom of scrollable area

Key handling in main.rs or wherever key events are processed.

## Test Requirements

Add tests to `src/ui/beach_detail.rs`:
- Test scroll offset stays within bounds
- Test scroll indicators appear when needed
- Test scroll indicators hidden when at top/bottom
