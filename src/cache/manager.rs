//! Cache manager for persisting API responses to disk
//!
//! Provides a `CacheManager` that stores serializable data to JSON files with
//! expiry timestamps, supporting graceful degradation when APIs are unavailable.

use chrono::{DateTime, Duration, Utc};
use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Wrapper struct for cached data stored on disk
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry<T> {
    /// The cached data
    data: T,
    /// When the data was cached
    cached_at: DateTime<Utc>,
    /// When the cache entry expires
    expires_at: DateTime<Utc>,
}

/// Result of reading from cache, including metadata about cache freshness
#[derive(Debug)]
pub struct CachedData<T> {
    /// The cached data
    pub data: T,
    /// When the data was originally cached
    #[allow(dead_code)]
    pub cached_at: DateTime<Utc>,
    /// Whether the cache entry has expired
    pub is_expired: bool,
}

/// Manages reading and writing cached data to disk
///
/// The cache manager stores data as JSON files in an XDG-compliant cache directory
/// (`~/.cache/vanbeach/` on Linux). Each cache entry includes an expiry timestamp,
/// and expired entries are still returned (with `is_expired = true`) to support
/// graceful degradation.
#[derive(Debug, Clone)]
pub struct CacheManager {
    /// Directory where cache files are stored
    cache_dir: PathBuf,
}

impl CacheManager {
    /// Creates a new CacheManager using XDG-compliant cache directory
    ///
    /// Uses `~/.cache/vanbeach/` on Linux, or equivalent XDG path on other platforms.
    /// Returns `None` if the cache directory cannot be determined (e.g., no home directory).
    pub fn new() -> Option<Self> {
        let project_dirs = ProjectDirs::from("", "", "vanbeach")?;
        let cache_dir = project_dirs.cache_dir().to_path_buf();
        Some(Self { cache_dir })
    }

    /// Creates a new CacheManager with a custom cache directory
    ///
    /// Useful for testing or when a specific cache location is needed.
    #[allow(dead_code)]
    pub fn with_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Returns the path to a cache file for the given key
    fn cache_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", key))
    }

    /// Ensures the cache directory exists
    fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.cache_dir)
    }

    /// Writes data to the cache with a specified TTL (time-to-live) in hours
    ///
    /// # Arguments
    /// * `key` - Unique identifier for the cache entry (e.g., "tides_kitsilano")
    /// * `data` - The data to cache (must implement Serialize)
    /// * `ttl_hours` - How long the cache entry should be considered fresh
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if directory creation or file writing fails
    pub fn write<T: Serialize>(&self, key: &str, data: &T, ttl_hours: u64) -> std::io::Result<()> {
        self.ensure_dir()?;

        let now = Utc::now();
        let entry = CacheEntry {
            data,
            cached_at: now,
            expires_at: now + Duration::hours(ttl_hours as i64),
        };

        let json = serde_json::to_string_pretty(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        fs::write(self.cache_path(key), json)
    }

    /// Reads data from the cache
    ///
    /// Returns `None` if the cache entry doesn't exist or cannot be parsed.
    /// Returns `Some(CachedData)` with `is_expired = true` if the entry exists but has expired,
    /// allowing for graceful degradation when APIs are unavailable.
    ///
    /// # Arguments
    /// * `key` - The cache key to read
    ///
    /// # Returns
    /// * `Some(CachedData<T>)` if the entry exists and can be parsed
    /// * `None` if the entry doesn't exist or parsing fails
    pub fn read<T: DeserializeOwned>(&self, key: &str) -> Option<CachedData<T>> {
        let path = self.cache_path(key);
        let content = fs::read_to_string(path).ok()?;
        let entry: CacheEntry<T> = serde_json::from_str(&content).ok()?;

        let now = Utc::now();
        let is_expired = now > entry.expires_at;

        Some(CachedData {
            data: entry.data,
            cached_at: entry.cached_at,
            is_expired,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::thread;
    use std::time::Duration as StdDuration;
    use tempfile::TempDir;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        name: String,
        value: i32,
    }

    fn create_test_cache() -> (CacheManager, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache = CacheManager::with_dir(temp_dir.path().to_path_buf());
        (cache, temp_dir)
    }

    #[test]
    fn test_write_creates_file_in_cache_directory() {
        let (cache, temp_dir) = create_test_cache();
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        cache.write("test_key", &data, 24).expect("Write should succeed");

        let expected_path = temp_dir.path().join("test_key.json");
        assert!(expected_path.exists(), "Cache file should exist");

        // Verify the file contains valid JSON
        let content = fs::read_to_string(&expected_path).expect("Should read file");
        assert!(content.contains("\"name\""));
        assert!(content.contains("\"test\""));
        assert!(content.contains("\"value\""));
        assert!(content.contains("42"));
    }

    #[test]
    fn test_read_returns_none_for_missing_key() {
        let (cache, _temp_dir) = create_test_cache();

        let result: Option<CachedData<TestData>> = cache.read("nonexistent_key");

        assert!(result.is_none(), "Should return None for missing key");
    }

    #[test]
    fn test_read_returns_data_with_is_expired_false_for_fresh_cache() {
        let (cache, _temp_dir) = create_test_cache();
        let data = TestData {
            name: "fresh".to_string(),
            value: 100,
        };

        cache.write("fresh_key", &data, 24).expect("Write should succeed");

        let result: CachedData<TestData> = cache.read("fresh_key").expect("Should read fresh cache");

        assert_eq!(result.data, data);
        assert!(!result.is_expired, "Fresh cache should not be expired");
    }

    #[test]
    fn test_read_returns_data_with_is_expired_true_for_expired_cache() {
        let (cache, _temp_dir) = create_test_cache();
        let data = TestData {
            name: "expired".to_string(),
            value: 0,
        };

        // Write with 0 hour TTL - should expire immediately
        cache.write("expired_key", &data, 0).expect("Write should succeed");

        // Small delay to ensure expiry
        thread::sleep(StdDuration::from_millis(10));

        let result: CachedData<TestData> = cache.read("expired_key").expect("Should read expired cache");

        assert_eq!(result.data, data);
        assert!(result.is_expired, "Cache with 0 TTL should be expired");
    }

    #[test]
    fn test_cache_survives_serialization_roundtrip() {
        let (cache, _temp_dir) = create_test_cache();
        let original = TestData {
            name: "roundtrip".to_string(),
            value: 12345,
        };

        cache.write("roundtrip_key", &original, 24).expect("Write should succeed");

        let result: CachedData<TestData> = cache.read("roundtrip_key").expect("Should read cache");

        assert_eq!(result.data, original, "Data should survive roundtrip");
    }

    #[test]
    fn test_write_creates_directory_if_missing() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let nested_path = temp_dir.path().join("nested").join("cache").join("dir");
        let cache = CacheManager::with_dir(nested_path.clone());

        let data = TestData {
            name: "nested".to_string(),
            value: 1,
        };

        cache.write("nested_key", &data, 24).expect("Write should succeed");

        assert!(nested_path.exists(), "Nested directory should be created");
        assert!(nested_path.join("nested_key.json").exists(), "Cache file should exist");
    }

    #[test]
    fn test_cached_at_timestamp_is_recorded() {
        let (cache, _temp_dir) = create_test_cache();
        let data = TestData {
            name: "timestamp".to_string(),
            value: 999,
        };

        let before = Utc::now();
        cache.write("timestamp_key", &data, 24).expect("Write should succeed");
        let after = Utc::now();

        let result: CachedData<TestData> = cache.read("timestamp_key").expect("Should read cache");

        assert!(result.cached_at >= before, "cached_at should be after write started");
        assert!(result.cached_at <= after, "cached_at should be before write finished");
    }

    #[test]
    fn test_new_creates_xdg_compliant_path() {
        if let Some(cache) = CacheManager::new() {
            let path_str = cache.cache_dir.to_string_lossy();
            assert!(
                path_str.contains("vanbeach"),
                "Cache path should contain project name"
            );
        }
        // Test passes if new() returns None (e.g., no home directory in CI)
    }

    #[test]
    fn test_overwrite_existing_cache() {
        let (cache, _temp_dir) = create_test_cache();
        let data1 = TestData {
            name: "first".to_string(),
            value: 1,
        };
        let data2 = TestData {
            name: "second".to_string(),
            value: 2,
        };

        cache.write("overwrite_key", &data1, 24).expect("First write should succeed");
        cache.write("overwrite_key", &data2, 24).expect("Second write should succeed");

        let result: CachedData<TestData> = cache.read("overwrite_key").expect("Should read cache");

        assert_eq!(result.data, data2, "Cache should contain latest data");
    }
}
