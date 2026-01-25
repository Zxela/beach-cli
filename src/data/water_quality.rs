//! Water quality API client for Vancouver Open Data
//!
//! Fetches beach water quality data from Vancouver Open Data API and maps
//! E. coli levels to water quality status.

use super::{WaterQuality, WaterStatus};
use crate::cache::CacheManager;
use chrono::{NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

/// Time-to-live for water quality cache entries in hours
const CACHE_TTL_HOURS: u64 = 24;

/// Number of days after which data is considered stale
const STALE_DATA_DAYS: i64 = 7;

/// E. coli threshold for safe water (CFU/100mL)
const ECOLI_SAFE_THRESHOLD: u32 = 200;

/// E. coli threshold for advisory (CFU/100mL)
const ECOLI_ADVISORY_THRESHOLD: u32 = 400;

/// Errors that can occur when fetching water quality data
#[derive(Debug, Error)]
pub enum WaterQualityError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// Failed to parse API response
    #[error("Failed to parse API response: {0}")]
    ParseError(String),
}

/// Response from Vancouver Open Data API
#[derive(Debug, Deserialize)]
struct ApiResponse {
    results: Vec<WaterQualityRecord>,
}

/// A single water quality record from the API
#[derive(Debug, Deserialize)]
struct WaterQualityRecord {
    /// Beach name from the API
    #[allow(dead_code)]
    beach_name: Option<String>,
    /// E. coli count (CFU per 100mL)
    e_coli: Option<f64>,
    /// Date the sample was taken (YYYY-MM-DD format)
    sample_date: Option<String>,
    /// Whether there's an advisory or closure
    #[serde(default)]
    advisory: Option<String>,
}

/// Client for fetching water quality data from Vancouver Open Data API
#[derive(Debug, Clone)]
pub struct WaterQualityClient {
    /// HTTP client for making requests
    http_client: Client,
    /// Cache manager for persisting responses
    cache_manager: Option<CacheManager>,
    /// Base URL for the API (allows override for testing)
    base_url: String,
}

impl WaterQualityClient {
    /// Creates a new WaterQualityClient with default configuration
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            cache_manager: CacheManager::new(),
            base_url: "https://opendata.vancouver.ca/api/explore/v2.1/catalog/datasets/beach-water-quality/records".to_string(),
        }
    }

    /// Creates a new WaterQualityClient with a custom cache manager
    pub fn with_cache(cache_manager: CacheManager) -> Self {
        Self {
            http_client: Client::new(),
            cache_manager: Some(cache_manager),
            base_url: "https://opendata.vancouver.ca/api/explore/v2.1/catalog/datasets/beach-water-quality/records".to_string(),
        }
    }

    /// Creates a new WaterQualityClient with a custom base URL (for testing)
    #[cfg(test)]
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            http_client: Client::new(),
            cache_manager: None,
            base_url,
        }
    }

    /// Generates a cache key for a beach
    fn cache_key(beach_name: &str) -> String {
        format!(
            "water_quality_{}",
            beach_name.replace(' ', "_").to_lowercase()
        )
    }

    /// Fetches water quality data for a specific beach
    ///
    /// # Arguments
    /// * `beach_name` - The name of the beach to fetch data for
    ///
    /// # Returns
    /// * `Ok(WaterQuality)` - Water quality data for the beach
    /// * `Err(WaterQualityError)` - If the request fails and no cached data is available
    ///
    /// # Behavior
    /// - First checks cache for fresh data
    /// - If cache is fresh, returns cached data
    /// - If cache is expired or missing, fetches from API
    /// - On API failure, returns expired cache data if available
    /// - Returns Unknown status if no data is available or data is older than 7 days
    pub async fn fetch_water_quality(
        &self,
        beach_name: &str,
    ) -> Result<WaterQuality, WaterQualityError> {
        let cache_key = Self::cache_key(beach_name);

        // Check cache first
        if let Some(ref cache_manager) = self.cache_manager {
            if let Some(cached) = cache_manager.read::<WaterQuality>(&cache_key) {
                if !cached.is_expired {
                    return Ok(cached.data);
                }
            }
        }

        // Try to fetch from API
        match self.fetch_from_api(beach_name).await {
            Ok(water_quality) => {
                // Cache the result
                if let Some(ref cache_manager) = self.cache_manager {
                    let _ = cache_manager.write(&cache_key, &water_quality, CACHE_TTL_HOURS);
                }
                Ok(water_quality)
            }
            Err(api_error) => {
                // Try to return expired cache data on API failure
                if let Some(ref cache_manager) = self.cache_manager {
                    if let Some(cached) = cache_manager.read::<WaterQuality>(&cache_key) {
                        return Ok(cached.data);
                    }
                }
                Err(api_error)
            }
        }
    }

    /// Fetches water quality data directly from the API
    async fn fetch_from_api(&self, beach_name: &str) -> Result<WaterQuality, WaterQualityError> {
        let url = format!(
            "{}?where=beach_name='{}'&order_by=sample_date desc&limit=1",
            self.base_url,
            urlencoded(beach_name)
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await?
            .json::<ApiResponse>()
            .await?;

        if response.results.is_empty() {
            return Ok(self.create_unknown_status(beach_name));
        }

        let record = &response.results[0];
        self.parse_record(record, beach_name)
    }

    /// Parses an API record into WaterQuality
    fn parse_record(
        &self,
        record: &WaterQualityRecord,
        beach_name: &str,
    ) -> Result<WaterQuality, WaterQualityError> {
        // Parse sample date
        let sample_date = match &record.sample_date {
            Some(date_str) => NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|e| {
                WaterQualityError::ParseError(format!("Invalid date format: {}", e))
            })?,
            None => return Ok(self.create_unknown_status(beach_name)),
        };

        // Check if data is stale (older than 7 days)
        let today = Utc::now().date_naive();
        let days_old = (today - sample_date).num_days();
        if days_old > STALE_DATA_DAYS {
            return Ok(self.create_unknown_status_with_date(sample_date));
        }

        // Parse E. coli count
        let ecoli_count = record.e_coli.map(|v| v as u32);

        // Determine status based on E. coli levels and advisory
        let status = self.determine_status(ecoli_count, record.advisory.as_deref());

        // Extract advisory reason if present
        let advisory_reason = if status == WaterStatus::Advisory || status == WaterStatus::Closed {
            record.advisory.clone()
        } else {
            None
        };

        Ok(WaterQuality {
            status,
            ecoli_count,
            sample_date,
            advisory_reason,
            fetched_at: Utc::now(),
        })
    }

    /// Determines water quality status based on E. coli count and advisory
    fn determine_status(&self, ecoli_count: Option<u32>, advisory: Option<&str>) -> WaterStatus {
        // Check for explicit closure in advisory
        if let Some(adv) = advisory {
            let adv_lower = adv.to_lowercase();
            if adv_lower.contains("closed") || adv_lower.contains("closure") {
                return WaterStatus::Closed;
            }
        }

        // Determine status based on E. coli thresholds
        match ecoli_count {
            Some(count) if count > ECOLI_ADVISORY_THRESHOLD => WaterStatus::Closed,
            Some(count) if count >= ECOLI_SAFE_THRESHOLD => WaterStatus::Advisory,
            Some(_) => WaterStatus::Safe,
            None => WaterStatus::Unknown,
        }
    }

    /// Creates an Unknown status WaterQuality with today's date
    fn create_unknown_status(&self, _beach_name: &str) -> WaterQuality {
        WaterQuality {
            status: WaterStatus::Unknown,
            ecoli_count: None,
            sample_date: Utc::now().date_naive(),
            advisory_reason: None,
            fetched_at: Utc::now(),
        }
    }

    /// Creates an Unknown status WaterQuality with a specific date
    fn create_unknown_status_with_date(&self, sample_date: NaiveDate) -> WaterQuality {
        WaterQuality {
            status: WaterStatus::Unknown,
            ecoli_count: None,
            sample_date,
            advisory_reason: None,
            fetched_at: Utc::now(),
        }
    }
}

impl Default for WaterQualityClient {
    fn default() -> Self {
        Self::new()
    }
}

/// URL-encodes a string for use in query parameters
fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20").replace('\'', "%27")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper to create a test cache manager
    fn create_test_cache() -> (CacheManager, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache = CacheManager::with_dir(temp_dir.path().to_path_buf());
        (cache, temp_dir)
    }

    #[test]
    fn test_ecoli_safe_threshold() {
        let client = WaterQualityClient::new();

        // Below 200 is Safe
        assert_eq!(client.determine_status(Some(0), None), WaterStatus::Safe);
        assert_eq!(client.determine_status(Some(50), None), WaterStatus::Safe);
        assert_eq!(client.determine_status(Some(199), None), WaterStatus::Safe);
    }

    #[test]
    fn test_ecoli_advisory_threshold() {
        let client = WaterQualityClient::new();

        // 200-400 is Advisory
        assert_eq!(
            client.determine_status(Some(200), None),
            WaterStatus::Advisory
        );
        assert_eq!(
            client.determine_status(Some(300), None),
            WaterStatus::Advisory
        );
        assert_eq!(
            client.determine_status(Some(400), None),
            WaterStatus::Advisory
        );
    }

    #[test]
    fn test_ecoli_closed_threshold() {
        let client = WaterQualityClient::new();

        // Above 400 is Closed
        assert_eq!(
            client.determine_status(Some(401), None),
            WaterStatus::Closed
        );
        assert_eq!(
            client.determine_status(Some(500), None),
            WaterStatus::Closed
        );
        assert_eq!(
            client.determine_status(Some(1000), None),
            WaterStatus::Closed
        );
    }

    #[test]
    fn test_explicit_closure_in_advisory() {
        let client = WaterQualityClient::new();

        // Explicit closure overrides E. coli reading
        assert_eq!(
            client.determine_status(Some(50), Some("Beach closed due to storm")),
            WaterStatus::Closed
        );
        assert_eq!(
            client.determine_status(Some(100), Some("Closure in effect")),
            WaterStatus::Closed
        );
        assert_eq!(
            client.determine_status(None, Some("Beach CLOSED")),
            WaterStatus::Closed
        );
    }

    #[test]
    fn test_unknown_status_when_no_ecoli_data() {
        let client = WaterQualityClient::new();

        assert_eq!(client.determine_status(None, None), WaterStatus::Unknown);
        assert_eq!(
            client.determine_status(None, Some("Some advisory")),
            WaterStatus::Unknown
        );
    }

    #[test]
    fn test_parse_valid_record() {
        let client = WaterQualityClient::new();
        let today = Utc::now().date_naive();

        let record = WaterQualityRecord {
            beach_name: Some("Kitsilano Beach".to_string()),
            e_coli: Some(50.0),
            sample_date: Some(today.format("%Y-%m-%d").to_string()),
            advisory: None,
        };

        let result = client.parse_record(&record, "Kitsilano Beach").unwrap();
        assert_eq!(result.status, WaterStatus::Safe);
        assert_eq!(result.ecoli_count, Some(50));
        assert_eq!(result.sample_date, today);
        assert!(result.advisory_reason.is_none());
    }

    #[test]
    fn test_parse_record_with_advisory() {
        let client = WaterQualityClient::new();
        let today = Utc::now().date_naive();

        let record = WaterQualityRecord {
            beach_name: Some("English Bay".to_string()),
            e_coli: Some(250.0),
            sample_date: Some(today.format("%Y-%m-%d").to_string()),
            advisory: Some("High bacteria levels".to_string()),
        };

        let result = client.parse_record(&record, "English Bay").unwrap();
        assert_eq!(result.status, WaterStatus::Advisory);
        assert_eq!(result.ecoli_count, Some(250));
        assert_eq!(
            result.advisory_reason,
            Some("High bacteria levels".to_string())
        );
    }

    #[test]
    fn test_stale_data_returns_unknown() {
        let client = WaterQualityClient::new();
        let old_date = Utc::now().date_naive() - chrono::Duration::days(10);

        let record = WaterQualityRecord {
            beach_name: Some("Kitsilano Beach".to_string()),
            e_coli: Some(50.0),
            sample_date: Some(old_date.format("%Y-%m-%d").to_string()),
            advisory: None,
        };

        let result = client.parse_record(&record, "Kitsilano Beach").unwrap();
        assert_eq!(result.status, WaterStatus::Unknown);
    }

    #[test]
    fn test_cache_key_generation() {
        assert_eq!(
            WaterQualityClient::cache_key("Kitsilano Beach"),
            "water_quality_kitsilano_beach"
        );
        assert_eq!(
            WaterQualityClient::cache_key("English Bay"),
            "water_quality_english_bay"
        );
        assert_eq!(
            WaterQualityClient::cache_key("Spanish Banks East"),
            "water_quality_spanish_banks_east"
        );
    }

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoded("Kitsilano Beach"), "Kitsilano%20Beach");
        assert_eq!(urlencoded("O'Brien's Beach"), "O%27Brien%27s%20Beach");
    }

    #[test]
    fn test_cache_integration_write_and_read() {
        let (cache, _temp_dir) = create_test_cache();
        let client = WaterQualityClient::with_cache(cache.clone());

        let water_quality = WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(50),
            sample_date: Utc::now().date_naive(),
            advisory_reason: None,
            fetched_at: Utc::now(),
        };

        let cache_key = WaterQualityClient::cache_key("test-beach");
        cache
            .write(&cache_key, &water_quality, CACHE_TTL_HOURS)
            .unwrap();

        let cached = cache.read::<WaterQuality>(&cache_key).unwrap();
        assert_eq!(cached.data.status, WaterStatus::Safe);
        assert_eq!(cached.data.ecoli_count, Some(50));
        assert!(!cached.is_expired);
    }

    #[test]
    fn test_default_implementation() {
        let client = WaterQualityClient::default();
        assert!(client.base_url.contains("opendata.vancouver.ca"));
    }

    #[test]
    fn test_create_unknown_status() {
        let client = WaterQualityClient::new();
        let unknown = client.create_unknown_status("Test Beach");

        assert_eq!(unknown.status, WaterStatus::Unknown);
        assert!(unknown.ecoli_count.is_none());
        assert!(unknown.advisory_reason.is_none());
    }

    #[tokio::test]
    async fn test_fetch_returns_cached_on_fresh_cache() {
        let (cache, _temp_dir) = create_test_cache();

        // Pre-populate cache with fresh data
        let water_quality = WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(75),
            sample_date: Utc::now().date_naive(),
            advisory_reason: None,
            fetched_at: Utc::now(),
        };

        let cache_key = WaterQualityClient::cache_key("cached-beach");
        cache
            .write(&cache_key, &water_quality, CACHE_TTL_HOURS)
            .unwrap();

        // Create client with cache - it should return cached data without hitting API
        let client = WaterQualityClient::with_cache(cache);
        let result = client.fetch_water_quality("cached-beach").await.unwrap();

        assert_eq!(result.status, WaterStatus::Safe);
        assert_eq!(result.ecoli_count, Some(75));
    }
}
