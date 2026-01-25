//! Cache module for storing API responses to disk
//!
//! This module provides a cache manager that persists API responses to the filesystem
//! with configurable TTL (time-to-live) values. It supports graceful degradation by
//! returning expired cache entries with an `is_expired` flag, allowing the application
//! to use stale data when APIs are unavailable.

mod manager;

pub use manager::{CacheManager, CachedData};
