# ADR: TUI Layout Redesign Architecture

## Status

Proposed

## Context

The beach detail screen currently uses a horizontal split layout:
- Weather (50% width) | Tides (50% width)
- Water Quality (full width)
- Best Window Today (full width, when activity selected)

This layout wastes vertical space in the Tides column and limits the width available for the tide sparkline visualization. Users have requested:
1. Better use of space
2. Expanded tide visualization
3. Hourly weather forecast

We need to decide on the overall layout strategy and how to handle the expanded content.

## Options Considered

### Option 1: Keep Horizontal Split, Add Tabs
Keep Weather|Tides side-by-side but add tabs or modes to show additional content (forecast, expanded tide).

**Pros:**
- Minimal layout change
- Compact default view

**Cons:**
- Doesn't solve the space inefficiency
- Hidden content requires discovery
- More complex navigation

### Option 2: Vertical Stack with Collapsible Sections
Stack all sections vertically (full width), allow sections to expand/collapse.

**Pros:**
- Maximum width for each section
- Clear information hierarchy
- Flexible — users control what's expanded

**Cons:**
- More vertical space needed
- Requires scroll support for smaller terminals

### Option 3: Adaptive Layout Based on Terminal Size
Dynamically switch between horizontal (wide terminals) and vertical (narrow terminals) layouts.

**Pros:**
- Optimal for each terminal size

**Cons:**
- Complex implementation
- Inconsistent UX across sizes
- Harder to test

## Decision

**Option 2: Vertical Stack with Collapsible Sections**

Rationale:
1. Provides consistent, predictable layout
2. Full-width sections enable richer visualizations (tide graph, hourly forecast)
3. Scrolling is a well-understood interaction pattern
4. Expansion toggle (for tide chart) is simple to implement and discover
5. Future features (beach info, etc.) fit naturally as additional sections

## Consequences

### Positive
- Tide sparkline can be 60+ characters wide (vs ~35 currently)
- Room for 6-8 hour forecast display
- Cleaner visual hierarchy
- Easier to add future sections

### Negative
- Detail view will be taller — may exceed 24-line terminals
- Need to implement scroll support
- Users accustomed to current layout need to adjust

### Mitigations
- Keep section order logical: Weather → Tides → Hourly Forecast → Water Quality → Best Window
- Implement smooth scrolling with clear position indicator
- Consider keyboard shortcuts to jump between sections

## Implementation Notes

### Section Order (top to bottom)
1. Activity selector (1 line)
2. Weather (6-7 lines)
3. Tides — compact (5 lines) or expanded (12 lines)
4. Hourly Forecast (8-10 lines)
5. Water Quality (4 lines)
6. Best Window Today (6 lines, when activity selected)
7. Help text (2 lines)

### Scroll Implementation
- Use ratatui's built-in scroll support or manual offset tracking
- Track `scroll_offset` in app state
- Render visible portion of content based on offset
- Show scroll indicator (e.g., "▲ more above" / "▼ more below")

### Tide Chart Toggle
- Add `tide_expanded: bool` to app state
- `t` key toggles expansion
- Expanded view uses box-drawing characters for graph

### Hourly Forecast Data
- Weather API already fetched — need to parse hourly data
- Cache hourly forecasts alongside current conditions
- Filter to show only future hours
