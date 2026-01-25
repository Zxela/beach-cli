---
id: "006"
title: "Implement tides API client"
status: pending
depends_on: ["002", "004"]
test_file: src/data/tides.rs
---

# 006: Implement tides API client

## Objective

Create an async client to fetch tide data for the Vancouver area, with caching support.

## Acceptance Criteria

- [ ] Create `src/data/tides.rs` with TidesClient struct
- [ ] `fetch_tides() -> Result<TideInfo, TidesError>` - fetches today's tide data
- [ ] Determine current tide state (Rising/Falling) based on time and tide events
- [ ] Calculate approximate current height based on interpolation
- [ ] Return next high and next low tide events
- [ ] Integrate with CacheManager (24-hour cache)
- [ ] Return cached data on API failure

## Technical Notes

From TECHNICAL_DESIGN.md:
- Station ID for Vancouver area: 7735 (Point Atkinson)
- DFO site may require HTML parsing or finding alternative JSON endpoint

Alternative approaches if DFO is difficult:
1. Use WorldTides API (requires API key - avoid per ADR)
2. Parse DFO HTML tables
3. Use pre-computed tide tables for 2026 (static data approach)

Consider using a tides calculation library or simple static data for MVP.

## Test Requirements

Add tests in `src/data/tides.rs`:
- Test parsing tide response into TideInfo
- Test tide state calculation (Rising when between low and high)
- Test cache integration (returns cached when fresh)
- Test returns stale cache on API failure
