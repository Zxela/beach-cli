# Architecture Decision Record: Best Time to Go Feature

## Context

The Vanbeach CLI shows current beach conditions but doesn't help users decide when to visit. We need to add activity-based time recommendations that:

1. Work with five different activities (swimming, sunbathing, sailing, sunset, peace)
2. Provide recommendations through multiple UI entry points
3. Score hourly time slots across 12 beaches
4. Stay fast and responsive

## Decision Drivers

- **Personal use focus** - Feature is primarily for the developer's own beach trips
- **Technical exploration** - Opportunity to learn algorithm design, async data pipelines, advanced TUI, and state management
- **Simplicity** - No external services for crowd data; keep dependencies minimal
- **Performance** - Must not slow down existing functionality

## Options Considered

### Option A: Activity Profiles with Weighted Scoring (Selected)

Define each activity as a profile with weighted factors:

```rust
struct ActivityProfile {
    name: &'static str,
    temp_weight: f32,
    water_quality_weight: f32,
    tide_preference: TidePref,
    wind_range: (f32, f32),
    uv_preference: UvPref,
    crowd_aversion: f32,
}
```

Each time slot gets a 0-100 score by:
1. Evaluating each condition factor (0.0-1.0 "goodness")
2. Multiplying by activity-specific weights
3. Summing and normalizing

**Pros:**
- Transparent, debuggable scoring
- Easy to tune weights
- New activities can be added easily
- Produces nuanced "pretty good" vs "perfect" distinctions

**Cons:**
- Weights need manual tuning
- May need iteration to feel right

### Option B: Rule-Based Recommendations

Hard-coded rules like "swimming is good when temp > 20C AND water quality is safe AND tide is not extreme low".

**Pros:**
- Simple to implement
- Easy to understand

**Cons:**
- Binary good/bad, no nuance
- Explosion of rules for edge cases
- Hard to maintain

### Option C: ML-Style Learned Preferences

Track which recommendations the user actually followed and learn personal weights over time.

**Pros:**
- Personalized recommendations
- Gets better over time

**Cons:**
- Massive overkill for personal tool
- Needs significant usage data
- Complex to implement and debug

## Decision

**Option A: Activity Profiles with Weighted Scoring**

This approach provides:
- Clear algorithm to learn from
- Testable scoring logic
- Good balance of nuance and simplicity
- Extensibility for future activities

## Consequences

### Positive

- Scoring engine is isolated and testable
- Activity profiles are data, not code - easy to adjust
- Can show score breakdowns to explain recommendations
- Natural fit for heatmap visualization

### Negative

- Initial weights may need tuning based on real-world use
- More complex than rule-based approach
- Need to fetch hourly forecast data (more API calls)

## Implementation Notes

### Crowd Estimation

No free API for real-time crowd data. Decision: use **heuristics based on time/day/season**.

```rust
fn estimate_crowd(month: u32, weekday: Weekday, hour: u32) -> f32
```

Lookup table approach:
- July + Saturday + 2pm = 0.9 (packed)
- March + Tuesday + 7am = 0.1 (empty)

Can add manual calibration later if heuristics are wrong.

### Hourly Weather Data

Open-Meteo already supports hourly forecasts - same endpoint, different parameters:

```
&hourly=temperature_2m,weathercode,windspeed_10m,uv_index
```

Cache hourly data for 1 hour (weather changes).

### State Persistence

Activity selection persists across screens within a session. Stored at `App` level:

```rust
struct App {
    current_activity: Option<Activity>,
    // ...
}
```

## Related Decisions

- Uses existing caching infrastructure from `CacheManager`
- Follows existing state machine pattern from `AppState`
- Maintains vim-style navigation conventions
