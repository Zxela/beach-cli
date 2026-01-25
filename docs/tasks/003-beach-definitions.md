---
id: "003"
title: "Define Vancouver beach data with coordinates"
status: pending
depends_on: ["002"]
test_file: src/data/beach.rs
---

# 003: Define Vancouver beach data with coordinates

## Objective

Create a static list of all Vancouver beaches with their geographic coordinates and water quality IDs for API lookups.

## Acceptance Criteria

- [ ] Create `src/data/beach.rs`
- [ ] Define `BEACHES` static array with all 12 beaches from PRD
- [ ] Each beach has accurate latitude/longitude coordinates
- [ ] Each beach has water_quality_id matching Vancouver Open Data naming
- [ ] Helper function `get_beach_by_id(id: &str) -> Option<&Beach>`
- [ ] Helper function `all_beaches() -> &[Beach]`

## Technical Notes

Target beaches from PRD.md:
1. Kitsilano Beach (49.2743, -123.1544)
2. English Bay Beach (49.2863, -123.1432)
3. Jericho Beach (49.2726, -123.1967)
4. Spanish Banks East (49.2756, -123.2089)
5. Spanish Banks West (49.2769, -123.2244)
6. Locarno Beach (49.2768, -123.2167)
7. Wreck Beach (49.2621, -123.2617)
8. Second Beach (49.2912, -123.1513)
9. Third Beach (49.2989, -123.1588)
10. Sunset Beach (49.2799, -123.1339)
11. Trout Lake Beach (49.2555, -123.0644)
12. New Brighton Beach (49.2930, -123.0365)

Water quality IDs should match the beach_name field in Vancouver Open Data API.

## Test Requirements

Add tests in `src/data/beach.rs`:
- Test BEACHES array has exactly 12 entries
- Test each beach has non-zero coordinates in Vancouver area
- Test get_beach_by_id returns correct beach
- Test get_beach_by_id returns None for invalid id
