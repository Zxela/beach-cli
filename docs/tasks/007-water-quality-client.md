---
id: "007"
title: "Implement Vancouver water quality API client"
status: pending
depends_on: ["002", "004"]
test_file: src/data/water_quality.rs
---

# 007: Implement Vancouver water quality API client

## Objective

Create an async client to fetch beach water quality data from Vancouver Open Data API, with caching support.

## Acceptance Criteria

- [ ] Create `src/data/water_quality.rs` with WaterQualityClient struct
- [ ] `fetch_water_quality(beach_name: &str) -> Result<WaterQuality, WaterQualityError>`
- [ ] Parse Vancouver Open Data JSON response
- [ ] Map E. coli levels to WaterStatus (Safe: <200, Advisory: 200-400, Closed: >400)
- [ ] Integrate with CacheManager (24-hour cache)
- [ ] Return cached data on API failure
- [ ] Handle missing beach data (return Unknown status)

## Technical Notes

From TECHNICAL_DESIGN.md API contract:
```
GET https://opendata.vancouver.ca/api/explore/v2.1/catalog/datasets/beach-water-quality/records
    ?where=beach_name='{beach_name}'
    &order_by=sample_date desc
    &limit=1
```

E. coli thresholds (CFU/100mL):
- Safe: < 200 (green)
- Advisory: 200-400 (yellow)
- Closed: > 400 or explicit closure (red)
- Unknown: No data in last 7 days (gray)

## Test Requirements

Add tests in `src/data/water_quality.rs`:
- Test parsing valid Vancouver Open Data response
- Test E. coli threshold mapping (Safe/Advisory/Closed)
- Test Unknown status when no recent data
- Test cache integration
- Test returns stale cache on API failure
