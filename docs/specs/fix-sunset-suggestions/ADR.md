# ADR: Dynamic Sunset Scoring

## Status
Proposed

## Context

The current sunset scoring system has two issues:

1. **Static time scorer**: The `sunset_time_scorer` function in `src/activities.rs` uses hard-coded hours:
   ```rust
   pub fn sunset_time_scorer(hour: u8) -> f32 {
       match hour {
           18..=20 => 1.0,
           17 | 21 => 0.7,
           16 | 22 => 0.3,
           _ => 0.1,
       }
   }
   ```
   This ignores the actual sunset time which varies from ~4:30 PM in December to ~9:15 PM in June in Vancouver.

2. **Past time recommendations**: The `compute_best_windows` function in `src/ui/beach_detail.rs` scores hours 6-21 without filtering out past hours.

The current architecture stores sunset time in `Weather.sunset` as a `NaiveTime`, and this data is accessible in `BeachConditions.weather`.

## Decision Drivers

- Minimal changes to existing architecture
- Type safety and compile-time guarantees
- Testability
- Backward compatibility with other activities

## Options Considered

### Option A: Pass sunset time to scorer function (Selected)
Change `time_of_day_scorer` from `fn(u8) -> f32` to a closure or pass additional context containing sunset time.

**Pros:**
- Direct solution to the problem
- Scoring function gets exact sunset time
- Clean separation of concerns

**Cons:**
- Requires changing the `ActivityProfile` struct signature
- Breaks existing function pointer pattern

### Option B: Pre-compute sunset-aware hours in compute_best_windows
Calculate which hours are good for sunset in `compute_best_windows` based on sunset time, before calling the scorer.

**Pros:**
- No changes to `ActivityProfile` or scoring functions
- All sunset logic contained in one place

**Cons:**
- Special-casing sunset activity in the scoring loop
- Less clean separation

### Option C: Create SunsetActivityProfile subtype
Create a specialized profile type for sunset that carries the sunset time.

**Pros:**
- Type-safe
- Clear intent

**Cons:**
- Over-engineered for this problem
- Requires significant refactoring

## Decision

**Option A with modification**: Instead of changing the function signature globally, we will:

1. Add an optional `sunset_hour` parameter to `score_time_slot` and `compute_best_windows`
2. Create a new `sunset_time_scorer_dynamic(hour: u8, sunset_hour: u8) -> f32` function
3. Use the dynamic scorer when sunset_hour is provided and the activity is Sunset
4. Filter past hours in `compute_best_windows` based on current time

This provides:
- Backward compatibility (existing tests pass)
- Dynamic sunset scoring
- Past-hour filtering
- Minimal changes to existing code

## Consequences

### Positive
- Sunset recommendations will be accurate year-round
- Users won't see past time recommendations
- Existing tests continue to pass
- Other activities unaffected

### Negative
- Slight increase in complexity in `compute_best_windows`
- Need to pass weather data deeper into scoring (already available via conditions)

### Neutral
- Tests need to be added for dynamic sunset scoring
- Documentation should be updated
