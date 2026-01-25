---
id: "016"
title: "Implement activity scoring engine"
status: pending
depends_on: ["012", "013", "014", "015"]
test_file: src/activities.rs (inline tests)
---

# 016: Implement activity scoring engine

## Objective

Build the core scoring engine that evaluates time slots for each activity. This combines weather, tide, water quality, and crowd data with activity-specific weights to produce 0-100 scores.

## Acceptance Criteria

- [ ] `ActivityProfile` struct with all weight fields
- [ ] `ScoreFactors` struct capturing individual factor scores (0.0-1.0)
- [ ] `TimeSlotScore` struct with time, beach_id, activity, score (0-100), factors
- [ ] `get_profile(activity: Activity) -> ActivityProfile` returns preset profiles
- [ ] Individual scoring methods: score_temperature, score_wind, score_water_quality, score_uv, score_tide, score_crowd
- [ ] `ActivityProfile::score_time_slot()` combines all factors with weights
- [ ] Custom time_of_day scorers for Sunset and Peace activities

## Technical Notes

From TECHNICAL_DESIGN.md - Activity profile weights:
- Swimming: water_quality=0.4 (critical), temp=0.3, tide=0.15, wind=0.1
- Sunbathing: temp=0.35, wind=0.25, uv=0.25, crowd=0.15
- Sailing: wind=0.6, tide=0.2, temp=0.1, crowd=0.1
- Sunset: time_of_day custom scorer, temp=0.15, crowd=0.15
- Peace: crowd=0.7 (dominant), time_of_day custom scorer

Scoring formula:
```
weighted_sum = sum(factor_score * weight for each factor)
total_weight = sum(all weights)
final_score = (weighted_sum / total_weight) * 100
```

## Files to Modify

- `src/activities.rs` - Add profiles and scoring logic
- `src/data/mod.rs` - May need to expose WaterStatus

## Test Requirements

Inline tests in `src/activities.rs`:
- Test score_temperature returns 1.0 when in ideal range
- Test score_temperature returns 0.0 when far outside range
- Test score_water_quality returns 0.0 for Closed, 1.0 for Safe
- Test score_wind returns 1.0 when in ideal range
- Test Swimming profile penalizes unsafe water heavily
- Test Sailing profile rewards high wind
- Test Peace profile heavily weights crowd_aversion
- Test sunset_time_scorer peaks at evening hours
- Test peace_time_scorer peaks at early morning
- Test full score_time_slot produces score in 0-100 range
