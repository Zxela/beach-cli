---
id: "004"
title: "Implement cache manager for API responses"
status: pending
depends_on: ["002"]
test_file: src/cache/manager.rs
---

# 004: Implement cache manager for API responses

## Objective

Create a cache manager that stores API responses to disk with expiry timestamps, supporting the graceful degradation requirement.

## Acceptance Criteria

- [ ] Create `src/cache/mod.rs` and `src/cache/manager.rs`
- [ ] `CacheManager` struct with configurable cache directory
- [ ] Use XDG-compliant cache path (`~/.cache/vanbeach/` via `directories` crate)
- [ ] `write<T: Serialize>(key: &str, data: &T, ttl_hours: u64)` - stores data with expiry
- [ ] `read<T: DeserializeOwned>(key: &str) -> Option<CachedData<T>>` - returns data with age info
- [ ] `CachedData<T>` struct containing data, cached_at timestamp, is_expired flag
- [ ] Cache files stored as JSON with `.json` extension
- [ ] Automatic directory creation if missing

## Technical Notes

From ADR.md caching strategy:
- Weather: No caching (always fetch fresh)
- Tides: Cache for 24 hours
- Water Quality: Cache for 24 hours

Cache key format: `{data_type}_{beach_id}.json` (e.g., `tides_kitsilano.json`)

## Test Requirements

Add tests in `src/cache/manager.rs`:
- Test write creates file in cache directory
- Test read returns None for missing key
- Test read returns data with is_expired=false for fresh cache
- Test read returns data with is_expired=true for expired cache
- Test cache survives serialization round-trip
