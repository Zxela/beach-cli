---
id: "018"
title: "Add activity filter and best window to beach detail UI"
status: pending
depends_on: ["016", "017"]
test_file: null
no_test_reason: "UI rendering - verified by manual testing"
---

# 018: Add activity filter and best window to beach detail UI

## Objective

Enhance the beach detail screen with activity selector chips and a "Best Window Today" section that shows the top time slots for the selected activity.

## Acceptance Criteria

- [ ] Activity selector row at top: [Swimming] [Sunbathing] [Sailing] [Sunset] [Peace]
- [ ] Selected activity shown with filled indicator (●), others with empty (○)
- [ ] "Best Window Today" section appears when activity is selected
- [ ] Shows top 3 time slots with scores and key factors
- [ ] Each slot shows: time range, score/100, brief reason
- [ ] Section hidden when no activity selected
- [ ] Keyboard shortcuts 1-5 visible in help bar

## Technical Notes

From WIREFRAMES.md:
```
│  Activity: [●Swimming] [○Sunbathing] [○Sailing] [○Sunset] [○Peace] │
│                                                                     │
│  BEST WINDOW TODAY                                                  │
│  ─────────────────                                                  │
│  🥇 11:00 AM - 1:00 PM    Score: 92/100                            │
│     Warm (24°C), safe water, mid-tide, moderate crowds             │
│                                                                     │
│  🥈 2:00 PM - 4:00 PM     Score: 85/100                            │
│     Hot (26°C), safe water, high tide, busier                      │
```

Need to:
1. Call scoring engine for each hour
2. Group adjacent high-scoring hours into windows
3. Sort by score, take top 3
4. Format factors into human-readable string

## Files to Modify

- `src/ui/beach_detail.rs` - Add activity chips and best window section

## Verification

Manual testing:
1. Navigate to beach detail
2. Press 1-5 to select activities
3. Verify activity chip highlights correctly
4. Verify "Best Window" section shows relevant recommendations
5. Verify recommendations change when activity changes
