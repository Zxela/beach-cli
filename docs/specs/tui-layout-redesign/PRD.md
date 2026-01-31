# PRD: TUI Layout Redesign

## Problem Statement

The current beach detail screen uses a side-by-side layout for Weather and Tides sections (50%/50% horizontal split). This creates several issues:

1. **Wasted space** — The Tides section is vertically shorter than Weather, leaving empty space
2. **Cramped readability** — Narrow columns make text harder to scan
3. **Limited visualization** — The tide sparkline is constrained to half-width
4. **No room for expansion** — Cannot add hourly forecast or richer data displays

## Goals

1. Improve space efficiency by using full-width vertical stacking
2. Enhance readability with wider, more breathable sections
3. Enable richer tide visualization (expandable ASCII graph)
4. Add hourly weather forecast display
5. Support smaller terminals via scrollable content

## Non-Goals

- Beach-specific info (amenities, parking, accessibility) — future iteration
- Enhanced activity recommendation displays — future iteration
- Responsive/adaptive layouts that hide content — prefer scrolling instead
- Changes to the beach list screen layout

## User Stories

### US-1: Vertical Layout
**As a** user viewing beach details
**I want** Weather and Tides displayed in full-width stacked sections
**So that** I can read information more easily without cramped columns

**Acceptance Criteria:**
- Weather section spans full width of the detail view
- Tides section appears below Weather, also full width
- All existing information is preserved
- No horizontal scrolling required at 80-column width

### US-2: Expandable Tide Chart
**As a** user planning around tides
**I want** to expand the tide sparkline into a detailed ASCII graph
**So that** I can see the full tide curve with precise height and time markers

**Acceptance Criteria:**
- Default view shows compact sparkline (current behavior, but full width)
- Pressing a key (e.g., `t`) expands to full ASCII graph
- Expanded graph shows Y-axis with height labels (0m-4m)
- Expanded graph shows X-axis with time markers (6AM to 12AM)
- Current time/height is marked on the graph
- Pressing the key again or Esc collapses back to sparkline

### US-3: Hourly Forecast
**As a** user planning a beach visit
**I want** to see hourly weather conditions for the upcoming hours
**So that** I can choose the best time to visit

**Acceptance Criteria:**
- Shows forecast for next 6-8 hours (or until end of day)
- Each hour displays: time, temperature, condition icon, wind speed, UV index
- Vertical list format for easy scanning
- Clearly indicates which hours have passed vs. upcoming

### US-4: Scrollable Content
**As a** user with a smaller terminal
**I want** the detail view to scroll vertically
**So that** I can access all information without needing a larger terminal

**Acceptance Criteria:**
- Content scrolls when it exceeds available screen height
- Scroll position indicator shows where user is in content
- Keyboard navigation (j/k or arrow keys) scrolls content
- Beach name header and help footer remain fixed (optional)

## Success Metrics

1. All existing beach detail information remains accessible
2. Tide chart can display at 60+ characters wide (vs current ~35)
3. Hourly forecast shows at least 6 upcoming hours
4. Detail view works on terminals as small as 80x20
