---
id: "106"
title: "Update tide sparkline to full width"
status: pending
depends_on: ["104"]
test_file: src/ui/beach_detail.rs
---

# 106: Update tide sparkline to full width

## Objective

Expand the tide sparkline visualization to use the full available width now that tides have their own row instead of sharing with weather.

## Acceptance Criteria

- [ ] Sparkline uses full section width (minus padding)
- [ ] Time markers span full width: 6AM to 12AM
- [ ] Current hour highlighting still works
- [ ] Next high/low times displayed on same line or below
- [ ] Hint "[t] expand" shown to indicate expandability

## Technical Notes

Current sparkline is ~16 characters (half width). New sparkline should be 50-60+ characters depending on terminal width.

Update `render_tides_section()` to:
1. Calculate available width from area.width
2. Generate more granular hourly heights
3. Render wider sparkline with proportional time markers

## Test Requirements

Add tests to `src/ui/beach_detail.rs`:
- Test sparkline width scales with terminal width
- Test time markers are evenly distributed
- Test "[t] expand" hint is visible
