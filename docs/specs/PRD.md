# Product Requirements Document: Vancouver Beach CLI

## Problem Statement

Deciding whether to visit a Vancouver beach requires checking multiple sources: weather forecasts, tide tables, and water quality advisories. This information is scattered across different websites and apps, making it tedious to get a quick overview before heading out.

## Goals

1. **Single source of truth** — Aggregate weather, tides, and water quality for all major Vancouver beaches in one tool
2. **Fast decision-making** — Display information in a scannable, colorful format with icons
3. **Easy browsing** — Interactive menu to explore beaches without memorizing commands
4. **Reliable** — Graceful degradation when data sources are unavailable; sensible caching

## Non-Goals

- Mobile app or web interface (CLI only)
- Beaches outside Vancouver proper (no North Van, West Van, Richmond)
- Real-time crowd data or parking availability
- Historical data or analytics
- Notifications or alerts

## User Stories

### US-1: Check conditions for a specific beach
**As a** beach-goer
**I want to** see current weather, tide times, and water quality for a specific beach
**So that** I can decide if conditions are good for my visit

**Acceptance Criteria:**
- Select beach from interactive menu
- See temperature, conditions, UV index, wind
- See today's high/low tide times and current tide status
- See water quality status (safe/advisory/closed)
- Information displayed with colors and icons for quick scanning

### US-2: Compare multiple beaches
**As a** beach-goer
**I want to** browse through different beaches
**So that** I can find the one with the best conditions

**Acceptance Criteria:**
- Navigate between beaches using keyboard
- Return to beach list easily
- See overview of all beaches with key stats

### US-3: Handle unavailable data gracefully
**As a** user
**I want** the CLI to show available data even when some sources fail
**So that** I still get useful information

**Acceptance Criteria:**
- If weather API fails, still show tides and water quality
- Clear indication when data is unavailable or stale
- Cached water quality data shown with age indicator

## Target Beaches

The CLI will cover these major Vancouver beaches:

1. Kitsilano Beach
2. English Bay Beach
3. Jericho Beach
4. Spanish Banks (East, West)
5. Locarno Beach
6. Wreck Beach
7. Second Beach (Stanley Park)
8. Third Beach (Stanley Park)
9. Sunset Beach
10. Trout Lake Beach
11. New Brighton Beach

## Success Metrics

- Launch to usable output in under 2 seconds
- All major beaches covered with accurate coordinates
- Water quality data no more than 24 hours stale
