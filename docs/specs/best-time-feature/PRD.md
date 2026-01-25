# Product Requirements Document: Best Time to Go Feature

## Problem Statement

The Vanbeach CLI currently shows current conditions for Vancouver beaches, but doesn't help users decide **when** to visit. Beach-goers have different goals - swimming, sunbathing, sailing, sunset walks, or finding solitude - and each activity has optimal conditions that vary throughout the day.

Users must mentally cross-reference weather forecasts, tide tables, and crowd patterns to figure out the best time for their preferred activity. This feature solves that by automatically recommending optimal time windows.

## Goals

1. **Activity-aware recommendations** - Score time slots based on user's intended activity (swimming, sunbathing, sailing, sunset, peace & quiet)
2. **Multiple entry points** - Accessible from detail view, dedicated planning screen, list view hints, and CLI flags
3. **Visual time planning** - Heatmap visualization showing quality across hours and beaches
4. **Smart defaults** - Contextual hints without requiring explicit activity selection

## Non-Goals

- Real-time crowd data (use heuristics instead)
- Notifications or alerts for optimal windows
- Multi-day trip planning (focus on today/tomorrow)
- Weather accuracy improvements (rely on existing Open-Meteo data)
- User preference learning/ML (static activity profiles)

## User Stories

### US-1: Quick activity recommendation on beach detail
**As a** beach-goer viewing a specific beach
**I want to** see the best time today for my activity
**So that** I can plan my visit without manual calculation

**Acceptance Criteria:**
- Activity selector chips visible on detail screen (Swimming, Sunbathing, Sailing, Sunset, Peace)
- Keyboard shortcuts 1-5 for quick selection
- "Best window" section shows top 2-3 time slots with scores
- Each slot shows time range and key factors (e.g., "11am-1pm: 87/100 - warm, safe water, moderate crowd")

### US-2: Compare times across all beaches
**As a** beach-goer with flexibility on location
**I want to** see which beach and time combination is best for my activity
**So that** I can choose both where and when to go

**Acceptance Criteria:**
- Dedicated "Plan Trip" screen accessible via 'p' key
- Heatmap grid with beaches on Y-axis, hours on X-axis
- Color-coded cells (green = excellent, yellow = good, red = poor)
- Activity selector changes entire grid
- Cursor navigation to select specific cell
- Summary shows best overall recommendation

### US-3: Contextual hints without explicit selection
**As a** beach-goer browsing the list
**I want to** see relevant hints based on current time
**So that** I get useful info without extra interaction

**Acceptance Criteria:**
- Each beach row shows contextual hint based on time of day
- Morning: "Good for peace" or "Warming up"
- Midday: "Peak swimming" or "Crowded"
- Evening: "Sunset in Xh Ym"
- Hints are subtle and don't clutter the existing layout

### US-4: Launch directly into planning mode
**As a** power user
**I want to** launch directly into activity planning from the command line
**So that** I skip navigation when I know what I want

**Acceptance Criteria:**
- `vanbeach --plan` opens Plan Trip screen
- `vanbeach --plan swim` opens Plan Trip with swimming pre-selected
- `vanbeach --plan sunset` opens with sunset pre-selected
- Invalid activity names show error message

## Activity Definitions

| Activity | Primary Focus | Optimal Conditions |
|----------|--------------|-------------------|
| Swimming | Water safety & comfort | Safe water quality, temp >18C, mid-tide depth |
| Sunbathing | Sun exposure & comfort | High temp (>22C), high UV, low wind (<10 km/h) |
| Sailing | Wind conditions | Steady 15-25 km/h wind, manageable gusts |
| Sunset | Timing & visibility | Clear skies, within 2 hours of sunset |
| Peace & Quiet | Avoiding crowds | Early morning (6-8am), weekdays, off-peak season |

## Success Metrics

- Plan Trip screen renders heatmap within 500ms of data load
- Recommendations align with common sense (swimming bad when water unsafe, sailing bad when calm)
- Activity switching updates grid within 100ms
- Feature doesn't slow down existing beach list/detail views
