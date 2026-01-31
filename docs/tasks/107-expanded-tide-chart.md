---
id: "107"
title: "Add expanded ASCII tide chart"
status: pending
depends_on: ["103", "106"]
test_file: src/ui/beach_detail.rs
---

# 107: Add expanded ASCII tide chart

## Objective

Implement the expanded tide chart view with full ASCII graph showing the tide curve, Y-axis labels, X-axis time markers, and current position indicator.

## Acceptance Criteria

- [ ] `t` key toggles between sparkline and expanded chart
- [ ] Expanded chart shows Y-axis with height labels (0m-4m)
- [ ] Expanded chart shows X-axis with time markers (6AM to 12AM)
- [ ] Tide curve rendered using box-drawing characters
- [ ] Current time/height marked with "●" on the curve
- [ ] "[t] collapse" hint shown in expanded mode
- [ ] Chart height ~12 lines when expanded (vs ~5 collapsed)

## Technical Notes

From WIREFRAMES.md:
```
4m ┤                    ╭────╮
   │                  ╭─╯    ╰─╮
3m ┤                ╭─╯        ╰─╮
   │              ╭─╯            ╰─╮
2m ┤            ╭─╯                ╰─╮
   │      ●───╭─╯                    ╰─╮
1m ┤        ╭─╯                        ╰─╮
   │      ╭─╯                            ╰─╮
0m ┼──────╯                                ╰──╯
   └────┬────┬────┬────┬────┬────┬────┬────┬────
       6AM  8AM 10AM 12PM  2PM  4PM  6PM  8PM 10PM
```

Implementation approach:
1. Sample tide heights at regular intervals
2. For each row (height level), determine which characters to draw
3. Use box-drawing characters: ─ │ ╭ ╮ ╰ ╯ ┤ ┼

## Test Requirements

Add tests to `src/ui/beach_detail.rs`:
- Test toggle state switches correctly
- Test expanded view height is greater than collapsed
- Test Y-axis labels present (0m, 1m, 2m, 3m, 4m)
- Test X-axis time markers present
