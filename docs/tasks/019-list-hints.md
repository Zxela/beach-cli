---
id: "019"
title: "Add contextual hints to beach list view"
status: pending
depends_on: ["016"]
test_file: null
no_test_reason: "UI rendering - verified by manual testing"
---

# 019: Add contextual hints to beach list view

## Objective

Add subtle contextual hints next to each beach in the list view, based on current time of day and conditions. These help users quickly identify relevant information without explicitly selecting an activity.

## Acceptance Criteria

- [ ] Each beach row shows a contextual hint in a muted color
- [ ] Hints vary based on time of day (morning/midday/evening)
- [ ] Hints consider current conditions (wind, water quality, sunset time)
- [ ] Hints are short (max ~20 characters)
- [ ] Hints don't clutter the existing layout
- [ ] Examples: "Good for peace", "Peak swimming", "Sunset in 2h", "Windy - good sailing"

## Technical Notes

From WIREFRAMES.md:
```
│  ▸ Kitsilano Beach         22°C  🌤️   🟢   Good for swimming      │
│    English Bay Beach       21°C  ☀️   🟢   Peak sun hours          │
│    Jericho Beach           20°C  🌤️   🟡   Quieter than Kits      │
│    Spanish Banks East      19°C  ☁️   🟢   Windy - good sailing    │
```

Hint logic (priority order):
1. Water quality issue → "Water advisory"
2. Within 2h of sunset → "Sunset in Xh Ym"
3. High wind (>15 km/h) → "Windy - good sailing"
4. Early morning (6-9am) → "Good for peace" or "Warming up"
5. Peak hours (12-4pm) + weekend → "Crowded now"
6. Peak hours + good weather → "Peak swimming" or "Peak sun hours"
7. Default based on temp/conditions

## Files to Modify

- `src/ui/beach_list.rs` - Add hint column to each beach row

## Verification

Manual testing:
1. Run app at different times of day
2. Verify hints make sense for current conditions
3. Verify hints are visible but not distracting
4. Verify layout doesn't break with long beach names
