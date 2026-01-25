# PRD: Fix Sunset Suggestions

## Problem Statement

The Vancouver Beach CLI tool currently suggests sunset viewing activities at inappropriate times. Specifically:

1. **Static time scoring**: The `sunset_time_scorer` function uses hard-coded hours (18-20) to score sunset viewing suitability, ignoring the actual sunset time from weather data. This means if sunset is at 5 PM in winter, the system still recommends 6-8 PM as best for sunset viewing.

2. **Past time recommendations**: The "Best Window Today" section displays time windows that have already passed. If the user checks the app at 9 PM after an 8 PM sunset, they still see recommendations for 6-8 PM sunset viewing.

## Goals

1. Make sunset activity scoring use the actual sunset time from weather API data
2. Filter out time windows that have already passed from recommendations
3. Maintain backward compatibility with the scoring engine for all other activities

## Non-Goals

1. Changing how other activities (Swimming, Sunbathing, Sailing, Peace) are scored
2. Adding sunset prediction for future days
3. Modifying the beach list hint system (already correctly handles sunset timing)

## User Stories

### US-1: Accurate Sunset Time Scoring
**As a** beach-goer looking for sunset viewing spots
**I want** the app to recommend times based on the actual sunset time
**So that** I arrive at the beach when the sunset is actually visible

**Acceptance Criteria:**
- When sunset is at 5:30 PM, the best window should center around 5:30 PM
- When sunset is at 9:00 PM, the best window should center around 9:00 PM
- Scoring should peak 30-60 minutes before actual sunset (golden hour)

### US-2: No Past Time Recommendations
**As a** user checking beach conditions in the evening
**I want** to only see time windows I can still use
**So that** I don't get confused by suggestions for times that already passed

**Acceptance Criteria:**
- Time windows where end_hour <= current_hour should be filtered out
- If all windows have passed, show a message like "Best times have passed for today"
- Current hour should still be shown if it's within a good window

## Success Metrics

- Sunset activity scores correlate with actual sunset time (can verify in tests)
- No past-time recommendations appear in the UI after those times pass
- All existing tests continue to pass
