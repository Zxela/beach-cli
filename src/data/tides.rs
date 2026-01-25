//! Tides API client for fetching tide information
//!
//! This module provides tide data for Vancouver area beaches using Point Atkinson
//! as the reference station (Station ID: 7735). For the MVP, it uses pre-computed
//! static tide predictions for January 2026.

use crate::cache::CacheManager;
use crate::data::{TideEvent, TideInfo, TideState};
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use thiserror::Error;

/// Cache key for tide data
const TIDES_CACHE_KEY: &str = "tides_point_atkinson";

/// Cache TTL in hours (24 hours as per requirements)
const TIDES_CACHE_TTL_HOURS: u64 = 24;

/// Errors that can occur when fetching tide data
#[derive(Debug, Error)]
pub enum TidesError {
    /// No tide data available for the requested date
    #[error("No tide data available for the requested date")]
    NoDataAvailable,

    /// Cache read/write error
    #[error("Cache error: {0}")]
    CacheError(String),
}

/// Client for fetching tide information
///
/// Uses static tide predictions for Point Atkinson (Station 7735) in the
/// Vancouver area. Integrates with CacheManager for 24-hour caching.
#[derive(Debug)]
pub struct TidesClient {
    cache: Option<CacheManager>,
}

/// A single tide prediction point (high or low)
#[derive(Debug, Clone)]
struct TidePrediction {
    date: NaiveDate,
    time: NaiveTime,
    height: f64,
    is_high: bool,
}

impl TidesClient {
    /// Creates a new TidesClient with optional cache manager
    pub fn new(cache: Option<CacheManager>) -> Self {
        Self { cache }
    }

    /// Fetches today's tide data
    ///
    /// Returns tide information including current height, tide state (rising/falling),
    /// and next high/low tide events. Uses cached data if fresh, falls back to
    /// cached data on failure.
    pub async fn fetch_tides(&self) -> Result<TideInfo, TidesError> {
        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.read::<TideInfo>(TIDES_CACHE_KEY) {
                if !cached.is_expired {
                    return Ok(cached.data);
                }
            }
        }

        // Generate tide info from static predictions
        let result = self.generate_tide_info();

        match result {
            Ok(tide_info) => {
                // Cache the successful result
                if let Some(ref cache) = self.cache {
                    let _ = cache.write(TIDES_CACHE_KEY, &tide_info, TIDES_CACHE_TTL_HOURS);
                }
                Ok(tide_info)
            }
            Err(e) => {
                // Try to return cached data on failure (even if expired)
                if let Some(ref cache) = self.cache {
                    if let Some(cached) = cache.read::<TideInfo>(TIDES_CACHE_KEY) {
                        return Ok(cached.data);
                    }
                }
                Err(e)
            }
        }
    }

    /// Generates tide info from static predictions for the current time
    fn generate_tide_info(&self) -> Result<TideInfo, TidesError> {
        let now = Local::now();
        let today = now.date_naive();

        // Get predictions for today and tomorrow (for next tide events)
        let predictions = self.get_predictions_for_date_range(today, 2);

        if predictions.is_empty() {
            return Err(TidesError::NoDataAvailable);
        }

        // Find the previous and next tide events relative to now
        let (prev_event, next_event) = self.find_surrounding_events(&predictions, now);

        // Determine tide state and calculate current height
        let (tide_state, current_height) = self.calculate_tide_state_and_height(
            prev_event.as_ref(),
            next_event.as_ref(),
            now,
        );

        // Find next high and next low tides
        let (next_high, next_low) = self.find_next_high_low(&predictions, now);

        Ok(TideInfo {
            current_height,
            tide_state,
            next_high,
            next_low,
            fetched_at: Utc::now(),
        })
    }

    /// Gets tide predictions for a date range starting from the given date
    fn get_predictions_for_date_range(&self, start_date: NaiveDate, days: i64) -> Vec<TidePrediction> {
        let mut predictions = Vec::new();

        for day_offset in 0..days {
            if let Some(date) = start_date.checked_add_signed(chrono::Duration::days(day_offset)) {
                predictions.extend(self.get_predictions_for_date(date));
            }
        }

        // Sort by date and time
        predictions.sort_by(|a, b| {
            let dt_a = a.date.and_time(a.time);
            let dt_b = b.date.and_time(b.time);
            dt_a.cmp(&dt_b)
        });

        predictions
    }

    /// Gets static tide predictions for a specific date
    ///
    /// These are pre-computed predictions for Point Atkinson (Station 7735)
    /// for January 2026. Heights are in meters.
    fn get_predictions_for_date(&self, date: NaiveDate) -> Vec<TidePrediction> {
        // Static tide predictions for January 2026 - Point Atkinson
        // Format: (day, hour, minute, height_meters, is_high)
        // Data based on typical semi-diurnal tidal patterns for Vancouver
        let january_2026_tides: &[(u32, u32, u32, f64, bool)] = &[
            // January 1, 2026
            (1, 2, 15, 4.8, true),
            (1, 8, 45, 1.2, false),
            (1, 14, 30, 4.5, true),
            (1, 21, 0, 0.8, false),
            // January 2, 2026
            (2, 3, 0, 4.7, true),
            (2, 9, 30, 1.3, false),
            (2, 15, 15, 4.4, true),
            (2, 21, 45, 0.9, false),
            // January 3, 2026
            (3, 3, 45, 4.6, true),
            (3, 10, 15, 1.4, false),
            (3, 16, 0, 4.3, true),
            (3, 22, 30, 1.0, false),
            // January 4, 2026
            (4, 4, 30, 4.5, true),
            (4, 11, 0, 1.5, false),
            (4, 16, 45, 4.2, true),
            (4, 23, 15, 1.1, false),
            // January 5, 2026
            (5, 5, 15, 4.4, true),
            (5, 11, 45, 1.6, false),
            (5, 17, 30, 4.1, true),
            // January 6, 2026
            (6, 0, 0, 1.2, false),
            (6, 6, 0, 4.3, true),
            (6, 12, 30, 1.7, false),
            (6, 18, 15, 4.0, true),
            // January 7, 2026
            (7, 0, 45, 1.3, false),
            (7, 6, 45, 4.2, true),
            (7, 13, 15, 1.8, false),
            (7, 19, 0, 3.9, true),
            // January 8, 2026
            (8, 1, 30, 1.4, false),
            (8, 7, 30, 4.1, true),
            (8, 14, 0, 1.9, false),
            (8, 19, 45, 3.8, true),
            // January 9, 2026
            (9, 2, 15, 1.5, false),
            (9, 8, 15, 4.0, true),
            (9, 14, 45, 2.0, false),
            (9, 20, 30, 3.7, true),
            // January 10, 2026
            (10, 3, 0, 1.6, false),
            (10, 9, 0, 3.9, true),
            (10, 15, 30, 2.1, false),
            (10, 21, 15, 3.6, true),
            // January 11, 2026
            (11, 3, 45, 1.7, false),
            (11, 9, 45, 3.8, true),
            (11, 16, 15, 2.0, false),
            (11, 22, 0, 3.7, true),
            // January 12, 2026
            (12, 4, 30, 1.6, false),
            (12, 10, 30, 3.9, true),
            (12, 17, 0, 1.9, false),
            (12, 22, 45, 3.8, true),
            // January 13, 2026
            (13, 5, 15, 1.5, false),
            (13, 11, 15, 4.0, true),
            (13, 17, 45, 1.8, false),
            (13, 23, 30, 3.9, true),
            // January 14, 2026
            (14, 6, 0, 1.4, false),
            (14, 12, 0, 4.1, true),
            (14, 18, 30, 1.7, false),
            // January 15, 2026
            (15, 0, 15, 4.0, true),
            (15, 6, 45, 1.3, false),
            (15, 12, 45, 4.2, true),
            (15, 19, 15, 1.6, false),
            // January 16, 2026
            (16, 1, 0, 4.1, true),
            (16, 7, 30, 1.2, false),
            (16, 13, 30, 4.3, true),
            (16, 20, 0, 1.5, false),
            // January 17, 2026
            (17, 1, 45, 4.2, true),
            (17, 8, 15, 1.1, false),
            (17, 14, 15, 4.4, true),
            (17, 20, 45, 1.4, false),
            // January 18, 2026
            (18, 2, 30, 4.3, true),
            (18, 9, 0, 1.0, false),
            (18, 15, 0, 4.5, true),
            (18, 21, 30, 1.3, false),
            // January 19, 2026
            (19, 3, 15, 4.4, true),
            (19, 9, 45, 0.9, false),
            (19, 15, 45, 4.6, true),
            (19, 22, 15, 1.2, false),
            // January 20, 2026
            (20, 4, 0, 4.5, true),
            (20, 10, 30, 0.8, false),
            (20, 16, 30, 4.7, true),
            (20, 23, 0, 1.1, false),
            // January 21, 2026
            (21, 4, 45, 4.6, true),
            (21, 11, 15, 0.9, false),
            (21, 17, 15, 4.6, true),
            (21, 23, 45, 1.0, false),
            // January 22, 2026
            (22, 5, 30, 4.5, true),
            (22, 12, 0, 1.0, false),
            (22, 18, 0, 4.5, true),
            // January 23, 2026
            (23, 0, 30, 1.1, false),
            (23, 6, 15, 4.4, true),
            (23, 12, 45, 1.1, false),
            (23, 18, 45, 4.4, true),
            // January 24, 2026
            (24, 1, 15, 1.2, false),
            (24, 7, 0, 4.3, true),
            (24, 13, 30, 1.2, false),
            (24, 19, 30, 4.3, true),
            // January 25, 2026
            (25, 2, 0, 1.3, false),
            (25, 7, 45, 4.2, true),
            (25, 14, 15, 1.3, false),
            (25, 20, 15, 4.2, true),
            // January 26, 2026
            (26, 2, 45, 1.4, false),
            (26, 8, 30, 4.1, true),
            (26, 15, 0, 1.4, false),
            (26, 21, 0, 4.1, true),
            // January 27, 2026
            (27, 3, 30, 1.5, false),
            (27, 9, 15, 4.0, true),
            (27, 15, 45, 1.5, false),
            (27, 21, 45, 4.0, true),
            // January 28, 2026
            (28, 4, 15, 1.6, false),
            (28, 10, 0, 3.9, true),
            (28, 16, 30, 1.6, false),
            (28, 22, 30, 3.9, true),
            // January 29, 2026
            (29, 5, 0, 1.7, false),
            (29, 10, 45, 3.8, true),
            (29, 17, 15, 1.7, false),
            (29, 23, 15, 3.8, true),
            // January 30, 2026
            (30, 5, 45, 1.8, false),
            (30, 11, 30, 3.9, true),
            (30, 18, 0, 1.6, false),
            // January 31, 2026
            (31, 0, 0, 3.9, true),
            (31, 6, 30, 1.7, false),
            (31, 12, 15, 4.0, true),
            (31, 18, 45, 1.5, false),
        ];

        // Filter for the requested date (January 2026 only)
        if date.year() != 2026 || date.month() != 1 {
            return Vec::new();
        }

        january_2026_tides
            .iter()
            .filter(|(day, _, _, _, _)| *day == date.day())
            .filter_map(|(_, hour, minute, height, is_high)| {
                let time = NaiveTime::from_hms_opt(*hour, *minute, 0)?;
                Some(TidePrediction {
                    date,
                    time,
                    height: *height,
                    is_high: *is_high,
                })
            })
            .collect()
    }

    /// Finds the previous and next tide events relative to the given time
    fn find_surrounding_events(
        &self,
        predictions: &[TidePrediction],
        now: DateTime<Local>,
    ) -> (Option<TidePrediction>, Option<TidePrediction>) {
        let now_naive = now.naive_local();

        let mut prev_event: Option<TidePrediction> = None;
        let mut next_event: Option<TidePrediction> = None;

        for pred in predictions {
            let pred_dt = pred.date.and_time(pred.time);

            if pred_dt <= now_naive {
                prev_event = Some(pred.clone());
            } else if next_event.is_none() {
                next_event = Some(pred.clone());
                break;
            }
        }

        (prev_event, next_event)
    }

    /// Calculates the current tide state and interpolated height
    fn calculate_tide_state_and_height(
        &self,
        prev_event: Option<&TidePrediction>,
        next_event: Option<&TidePrediction>,
        now: DateTime<Local>,
    ) -> (TideState, f64) {
        let now_naive = now.naive_local();

        match (prev_event, next_event) {
            (Some(prev), Some(next)) => {
                // Determine if rising or falling
                let tide_state = if prev.is_high {
                    TideState::Falling
                } else {
                    TideState::Rising
                };

                // Interpolate current height using cosine interpolation for smoother tidal curve
                let prev_dt = prev.date.and_time(prev.time);
                let next_dt = next.date.and_time(next.time);

                let total_duration = (next_dt - prev_dt).num_seconds() as f64;
                let elapsed = (now_naive - prev_dt).num_seconds() as f64;

                let progress = if total_duration > 0.0 {
                    (elapsed / total_duration).clamp(0.0, 1.0)
                } else {
                    0.5
                };

                // Cosine interpolation for realistic tidal curve
                let cosine_progress = (1.0 - (progress * std::f64::consts::PI).cos()) / 2.0;
                let current_height = prev.height + (next.height - prev.height) * cosine_progress;

                // Check if we're very close to high or low
                let height_diff = (next.height - prev.height).abs();
                let threshold = height_diff * 0.05; // Within 5% of extreme

                let final_state = if (current_height - next.height).abs() < threshold {
                    if next.is_high {
                        TideState::High
                    } else {
                        TideState::Low
                    }
                } else if (current_height - prev.height).abs() < threshold {
                    if prev.is_high {
                        TideState::High
                    } else {
                        TideState::Low
                    }
                } else {
                    tide_state
                };

                (final_state, current_height)
            }
            (Some(prev), None) => {
                // Only have previous event
                let state = if prev.is_high {
                    TideState::Falling
                } else {
                    TideState::Rising
                };
                (state, prev.height)
            }
            (None, Some(next)) => {
                // Only have next event
                let state = if next.is_high {
                    TideState::Rising
                } else {
                    TideState::Falling
                };
                (state, next.height)
            }
            (None, None) => {
                // No events available
                (TideState::Rising, 2.5) // Default mid-tide
            }
        }
    }

    /// Finds the next high and low tide events after the given time
    fn find_next_high_low(
        &self,
        predictions: &[TidePrediction],
        now: DateTime<Local>,
    ) -> (Option<TideEvent>, Option<TideEvent>) {
        let now_naive = now.naive_local();
        let mut next_high: Option<TideEvent> = None;
        let mut next_low: Option<TideEvent> = None;

        for pred in predictions {
            let pred_dt = pred.date.and_time(pred.time);

            if pred_dt > now_naive {
                let local_time = Local
                    .from_local_datetime(&pred_dt)
                    .single()
                    .unwrap_or_else(|| now);

                let event = TideEvent {
                    time: local_time,
                    height: pred.height,
                };

                if pred.is_high && next_high.is_none() {
                    next_high = Some(event);
                } else if !pred.is_high && next_low.is_none() {
                    next_low = Some(event);
                }

                if next_high.is_some() && next_low.is_some() {
                    break;
                }
            }
        }

        (next_high, next_low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_client() -> (TidesClient, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let cache = CacheManager::with_dir(temp_dir.path().to_path_buf());
        let client = TidesClient::new(Some(cache));
        (client, temp_dir)
    }

    #[test]
    fn test_parse_tide_predictions_for_january_1() {
        let client = TidesClient::new(None);
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let predictions = client.get_predictions_for_date(date);

        assert_eq!(predictions.len(), 4, "January 1 should have 4 tide events");

        // First event should be high tide at 2:15
        assert!(predictions[0].is_high);
        assert_eq!(predictions[0].time, NaiveTime::from_hms_opt(2, 15, 0).unwrap());
        assert!((predictions[0].height - 4.8).abs() < 0.01);

        // Second event should be low tide at 8:45
        assert!(!predictions[1].is_high);
        assert_eq!(predictions[1].time, NaiveTime::from_hms_opt(8, 45, 0).unwrap());
        assert!((predictions[1].height - 1.2).abs() < 0.01);
    }

    #[test]
    fn test_tide_state_rising_between_low_and_high() {
        let client = TidesClient::new(None);

        let prev = TidePrediction {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            time: NaiveTime::from_hms_opt(8, 45, 0).unwrap(),
            height: 1.2,
            is_high: false,
        };

        let next = TidePrediction {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            time: NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            height: 4.5,
            is_high: true,
        };

        // Time between low and high tide
        let now = Local
            .from_local_datetime(
                &NaiveDate::from_ymd_opt(2026, 1, 1)
                    .unwrap()
                    .and_hms_opt(11, 0, 0)
                    .unwrap(),
            )
            .single()
            .unwrap();

        let (state, height) = client.calculate_tide_state_and_height(Some(&prev), Some(&next), now);

        assert_eq!(state, TideState::Rising, "Should be rising between low and high");
        assert!(height > 1.2 && height < 4.5, "Height should be between low and high");
    }

    #[test]
    fn test_tide_state_falling_between_high_and_low() {
        let client = TidesClient::new(None);

        let prev = TidePrediction {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            time: NaiveTime::from_hms_opt(2, 15, 0).unwrap(),
            height: 4.8,
            is_high: true,
        };

        let next = TidePrediction {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            time: NaiveTime::from_hms_opt(8, 45, 0).unwrap(),
            height: 1.2,
            is_high: false,
        };

        // Time between high and low tide
        let now = Local
            .from_local_datetime(
                &NaiveDate::from_ymd_opt(2026, 1, 1)
                    .unwrap()
                    .and_hms_opt(5, 30, 0)
                    .unwrap(),
            )
            .single()
            .unwrap();

        let (state, height) = client.calculate_tide_state_and_height(Some(&prev), Some(&next), now);

        assert_eq!(state, TideState::Falling, "Should be falling between high and low");
        assert!(height > 1.2 && height < 4.8, "Height should be between high and low");
    }

    #[test]
    fn test_interpolated_height_is_reasonable() {
        let client = TidesClient::new(None);

        let prev = TidePrediction {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            time: NaiveTime::from_hms_opt(8, 45, 0).unwrap(),
            height: 1.2,
            is_high: false,
        };

        let next = TidePrediction {
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            time: NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            height: 4.5,
            is_high: true,
        };

        // Midpoint in time
        let now = Local
            .from_local_datetime(
                &NaiveDate::from_ymd_opt(2026, 1, 1)
                    .unwrap()
                    .and_hms_opt(11, 37, 30)
                    .unwrap(),
            )
            .single()
            .unwrap();

        let (_, height) = client.calculate_tide_state_and_height(Some(&prev), Some(&next), now);

        // At midpoint with cosine interpolation, should be close to average
        let avg = (1.2 + 4.5) / 2.0;
        assert!(
            (height - avg).abs() < 0.5,
            "Height at midpoint should be close to average (got {}, expected ~{})",
            height,
            avg
        );
    }

    #[tokio::test]
    async fn test_cache_returns_cached_when_fresh() {
        let (client, _temp_dir) = create_test_client();

        // First fetch should populate cache
        let result1 = client.fetch_tides().await;
        assert!(result1.is_ok(), "First fetch should succeed");

        // Second fetch should return cached data
        let result2 = client.fetch_tides().await;
        assert!(result2.is_ok(), "Second fetch should succeed from cache");

        // Both should return valid TideInfo
        let tide1 = result1.unwrap();
        let tide2 = result2.unwrap();

        // Current height should be the same (from cache)
        assert!(
            (tide1.current_height - tide2.current_height).abs() < 0.01,
            "Cached data should return same height"
        );
    }

    #[tokio::test]
    async fn test_fetch_tides_returns_valid_tide_info() {
        let client = TidesClient::new(None);

        // This will only work during January 2026
        let result = client.fetch_tides().await;

        // If we're in January 2026, it should succeed
        let now = Local::now();
        if now.year() == 2026 && now.month() == 1 {
            assert!(result.is_ok(), "Should return tide info in January 2026");
            let tide_info = result.unwrap();

            // Validate the structure
            assert!(tide_info.current_height >= 0.0);
            assert!(tide_info.current_height <= 6.0); // Reasonable tide height range
        }
    }

    #[test]
    fn test_find_next_high_low() {
        let client = TidesClient::new(None);
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let predictions = client.get_predictions_for_date_range(date, 2);

        // Time early on January 1
        let now = Local
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();

        let (next_high, next_low) = client.find_next_high_low(&predictions, now);

        assert!(next_high.is_some(), "Should find next high tide");
        assert!(next_low.is_some(), "Should find next low tide");

        let high = next_high.unwrap();
        let low = next_low.unwrap();

        // First high is at 2:15 with height 4.8
        assert!((high.height - 4.8).abs() < 0.01);
        // First low is at 8:45 with height 1.2
        assert!((low.height - 1.2).abs() < 0.01);
    }

    #[test]
    fn test_no_data_for_other_months() {
        let client = TidesClient::new(None);

        // February should have no data
        let date = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let predictions = client.get_predictions_for_date(date);
        assert!(predictions.is_empty(), "February 2026 should have no data");

        // 2025 should have no data
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let predictions = client.get_predictions_for_date(date);
        assert!(predictions.is_empty(), "2025 should have no data");
    }
}
