//! Core data models for Vancouver Beach CLI
//!
//! This module contains all the data types used throughout the application
//! for representing beaches, weather, tides, and water quality information.

pub mod beach;
pub mod tides;
pub mod water_quality;
pub mod weather;

pub use beach::{all_beaches, get_beach_by_id};
pub use tides::TidesClient;
pub use water_quality::{WaterQualityClient, WaterQualityError};
#[allow(unused_imports)]
pub use weather::{ApiHourlyForecast, WeatherClient, WeatherData, WeatherError};

use chrono::{DateTime, Local, NaiveDate, NaiveTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Represents a beach location in Vancouver
///
/// Uses `&'static str` for string fields to allow static initialization
/// of the BEACHES array. For runtime-created Beach instances, use string
/// literals or leak the strings.
///
/// Note: This struct only implements `Serialize` (not `Deserialize`) because
/// the static string references cannot be safely deserialized. Use `get_beach_by_id`
/// to look up beaches from deserialized beach IDs.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Beach {
    /// Unique identifier for the beach
    pub id: &'static str,
    /// Human-readable name of the beach
    pub name: &'static str,
    /// Latitude coordinate
    pub latitude: f64,
    /// Longitude coordinate
    pub longitude: f64,
    /// Optional identifier for water quality monitoring station
    pub water_quality_id: Option<&'static str>,
}

/// Hourly weather forecast data for a single hour
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyForecast {
    /// Hour of the day (0-23)
    pub hour: u8,
    /// Temperature in Celsius
    pub temperature: f64,
    /// Feels-like temperature in Celsius
    pub feels_like: f64,
    /// Weather condition
    pub condition: WeatherCondition,
    /// Wind speed in km/h
    pub wind: f64,
    /// Wind direction (e.g., "N", "NE", "SW")
    pub wind_direction: String,
    /// UV index
    pub uv: f64,
    /// Precipitation chance percentage (0-100)
    pub precipitation_chance: u8,
}

/// Weather conditions at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weather {
    /// Current temperature in Celsius
    pub temperature: f64,
    /// Feels-like temperature in Celsius
    pub feels_like: f64,
    /// Current weather condition
    pub condition: WeatherCondition,
    /// Relative humidity percentage (0-100)
    pub humidity: u8,
    /// Wind speed in km/h
    pub wind: f64,
    /// UV index
    pub uv: f64,
    /// Sunrise time
    pub sunrise: NaiveTime,
    /// Sunset time
    pub sunset: NaiveTime,
    /// When this data was fetched
    pub fetched_at: DateTime<Utc>,
    /// Hourly forecasts for today
    #[serde(default)]
    pub hourly: Vec<HourlyForecast>,
}

/// Types of weather conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherCondition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Rain,
    Showers,
    Thunderstorm,
    Snow,
    Fog,
}

/// Tide information including current state and upcoming events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TideInfo {
    /// Current tide height in meters
    pub current_height: f64,
    /// Current tide state (rising, falling, etc.)
    pub tide_state: TideState,
    /// Next high tide event
    pub next_high: Option<TideEvent>,
    /// Next low tide event
    pub next_low: Option<TideEvent>,
    /// When this data was fetched
    pub fetched_at: DateTime<Utc>,
}

/// A specific tide event (high or low tide)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TideEvent {
    /// Time of the tide event in local timezone
    pub time: DateTime<Local>,
    /// Height of the tide in meters
    pub height: f64,
}

/// Current state of the tide
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TideState {
    Rising,
    Falling,
    High,
    Low,
}

impl TideInfo {
    /// Generates estimated tide heights for hours 6am-9pm (16 hours)
    ///
    /// Uses a simplified sinusoidal tide model based on the next high/low events.
    /// Returns a vector of 16 heights, one per hour from 6am to 9pm.
    ///
    /// # Arguments
    /// * `max_height` - Maximum tide height for the region (e.g., 4.8m for Vancouver)
    pub fn hourly_heights(&self, max_height: f64) -> Vec<f64> {
        let mut heights = Vec::with_capacity(16);

        // Get phase from next_high time if available
        let phase = self
            .next_high
            .as_ref()
            .map(|h| h.time.hour() as f64)
            .unwrap_or(12.0);

        for hour in 6..=21 {
            // Simple sine wave approximation (two tides per day)
            let t = (hour as f64 - phase) * std::f64::consts::PI / 6.0;
            let height = (max_height / 2.0) * (1.0 + t.cos());
            heights.push(height.clamp(0.0, max_height));
        }

        heights
    }
}

/// Water quality information from monitoring stations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterQuality {
    /// Current water quality status
    pub status: WaterStatus,
    /// E. coli count (CFU per 100mL), if available
    pub ecoli_count: Option<u32>,
    /// Date when the water sample was taken
    pub sample_date: NaiveDate,
    /// Reason for advisory, if applicable
    pub advisory_reason: Option<String>,
    /// When this data was fetched
    pub fetched_at: DateTime<Utc>,
}

impl WaterQuality {
    /// Returns true if the water quality data is stale (sample > 48 hours old)
    pub fn is_stale(&self) -> bool {
        let today = chrono::Local::now().date_naive();
        let days_old = (today - self.sample_date).num_days();
        days_old > 2
    }

    /// Returns true if the water quality data is very stale (sample > 7 days old)
    #[allow(dead_code)]
    pub fn is_very_stale(&self) -> bool {
        let today = chrono::Local::now().date_naive();
        let days_old = (today - self.sample_date).num_days();
        days_old > 7
    }

    /// Returns the effective status, downgrading to Unknown if stale
    pub fn effective_status(&self) -> WaterStatus {
        if self.is_stale() {
            WaterStatus::Unknown
        } else {
            self.status
        }
    }
}

/// Water quality status levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterStatus {
    /// Safe for swimming
    Safe,
    /// Advisory in effect - swimming not recommended
    Advisory,
    /// Beach closed for swimming
    Closed,
    /// Status unknown or data unavailable
    Unknown,
}

/// Combined beach conditions including all available data
///
/// Note: This struct only implements `Serialize` (not `Deserialize`) because
/// the Beach struct uses static string references.
#[derive(Debug, Clone, Serialize)]
pub struct BeachConditions {
    /// The beach this data is for
    pub beach: Beach,
    /// Current weather conditions, if available
    pub weather: Option<Weather>,
    /// Current tide information, if available
    pub tides: Option<TideInfo>,
    /// Current water quality information, if available
    pub water_quality: Option<WaterQuality>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beach_creation() {
        let beach = Beach {
            id: "kitsilano",
            name: "Kitsilano Beach",
            latitude: 49.2743,
            longitude: -123.1544,
            water_quality_id: Some("kits-001"),
        };

        assert_eq!(beach.id, "kitsilano");
        assert_eq!(beach.name, "Kitsilano Beach");
        assert!((beach.latitude - 49.2743).abs() < 0.0001);
        assert!((beach.longitude - (-123.1544)).abs() < 0.0001);
        assert_eq!(beach.water_quality_id, Some("kits-001"));
    }

    #[test]
    fn test_weather_serialization_roundtrip() {
        let weather = Weather {
            temperature: 22.5,
            feels_like: 24.0,
            condition: WeatherCondition::PartlyCloudy,
            humidity: 65,
            wind: 12.5,
            uv: 6.0,
            sunrise: NaiveTime::from_hms_opt(5, 30, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(21, 15, 0).unwrap(),
            fetched_at: Utc::now(),
            hourly: Vec::new(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&weather).expect("Failed to serialize Weather");

        // Deserialize back
        let deserialized: Weather =
            serde_json::from_str(&json).expect("Failed to deserialize Weather");

        // Verify values match
        assert!((deserialized.temperature - 22.5).abs() < 0.01);
        assert!((deserialized.feels_like - 24.0).abs() < 0.01);
        assert_eq!(deserialized.condition, WeatherCondition::PartlyCloudy);
        assert_eq!(deserialized.humidity, 65);
        assert!((deserialized.wind - 12.5).abs() < 0.01);
        assert!((deserialized.uv - 6.0).abs() < 0.01);
        assert_eq!(
            deserialized.sunrise,
            NaiveTime::from_hms_opt(5, 30, 0).unwrap()
        );
        assert_eq!(
            deserialized.sunset,
            NaiveTime::from_hms_opt(21, 15, 0).unwrap()
        );
    }

    #[test]
    fn test_weather_condition_variants() {
        let conditions = [
            WeatherCondition::Clear,
            WeatherCondition::PartlyCloudy,
            WeatherCondition::Cloudy,
            WeatherCondition::Rain,
            WeatherCondition::Showers,
            WeatherCondition::Thunderstorm,
            WeatherCondition::Snow,
            WeatherCondition::Fog,
        ];

        // Verify all variants are distinct
        for (i, cond1) in conditions.iter().enumerate() {
            for (j, cond2) in conditions.iter().enumerate() {
                if i == j {
                    assert_eq!(cond1, cond2);
                } else {
                    assert_ne!(cond1, cond2);
                }
            }
        }
    }

    #[test]
    fn test_tide_state_variants() {
        let states = [
            TideState::Rising,
            TideState::Falling,
            TideState::High,
            TideState::Low,
        ];

        // Verify all variants are distinct
        for (i, state1) in states.iter().enumerate() {
            for (j, state2) in states.iter().enumerate() {
                if i == j {
                    assert_eq!(state1, state2);
                } else {
                    assert_ne!(state1, state2);
                }
            }
        }
    }

    #[test]
    fn test_water_status_variants() {
        let statuses = [
            WaterStatus::Safe,
            WaterStatus::Advisory,
            WaterStatus::Closed,
            WaterStatus::Unknown,
        ];

        // Verify all variants are distinct
        for (i, status1) in statuses.iter().enumerate() {
            for (j, status2) in statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(status1, status2);
                } else {
                    assert_ne!(status1, status2);
                }
            }
        }
    }

    #[test]
    fn test_tide_info_creation() {
        let tide_info = TideInfo {
            current_height: 2.5,
            tide_state: TideState::Rising,
            next_high: Some(TideEvent {
                time: Local::now(),
                height: 4.2,
            }),
            next_low: Some(TideEvent {
                time: Local::now(),
                height: 0.8,
            }),
            fetched_at: Utc::now(),
        };

        assert!((tide_info.current_height - 2.5).abs() < 0.01);
        assert_eq!(tide_info.tide_state, TideState::Rising);
        assert!(tide_info.next_high.is_some());
        assert!(tide_info.next_low.is_some());
    }

    #[test]
    fn test_water_quality_creation() {
        let water_quality = WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(50),
            sample_date: NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(),
            advisory_reason: None,
            fetched_at: Utc::now(),
        };

        assert_eq!(water_quality.status, WaterStatus::Safe);
        assert_eq!(water_quality.ecoli_count, Some(50));
        assert!(water_quality.advisory_reason.is_none());
    }

    #[test]
    fn test_beach_conditions_creation() {
        let beach = Beach {
            id: "english-bay",
            name: "English Bay Beach",
            latitude: 49.2867,
            longitude: -123.1422,
            water_quality_id: None,
        };

        let conditions = BeachConditions {
            beach,
            weather: None,
            tides: None,
            water_quality: None,
        };

        assert_eq!(conditions.beach.id, "english-bay");
        assert!(conditions.weather.is_none());
        assert!(conditions.tides.is_none());
        assert!(conditions.water_quality.is_none());
    }

    #[test]
    fn test_water_quality_is_stale_after_48_hours() {
        let today = chrono::Local::now().date_naive();
        let old_date = today - chrono::Duration::days(3); // 3 days ago
        let wq = WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(50),
            sample_date: old_date,
            advisory_reason: None,
            fetched_at: Utc::now(),
        };

        assert!(
            wq.is_stale(),
            "Water quality from 3+ days ago should be stale"
        );
    }

    #[test]
    fn test_water_quality_not_stale_within_48_hours() {
        let today = chrono::Local::now().date_naive();
        let recent_date = today - chrono::Duration::days(1); // Yesterday
        let wq = WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(50),
            sample_date: recent_date,
            advisory_reason: None,
            fetched_at: Utc::now(),
        };

        assert!(
            !wq.is_stale(),
            "Water quality from yesterday should not be stale"
        );
    }

    #[test]
    fn test_water_quality_effective_status_downgrades_when_stale() {
        let today = chrono::Local::now().date_naive();
        let old_date = today - chrono::Duration::days(5);
        let wq = WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(50),
            sample_date: old_date,
            advisory_reason: None,
            fetched_at: Utc::now(),
        };

        assert_eq!(
            wq.effective_status(),
            WaterStatus::Unknown,
            "Stale safe water should downgrade to unknown"
        );
    }

    #[test]
    fn test_water_quality_effective_status_keeps_fresh() {
        let today = chrono::Local::now().date_naive();
        let wq = WaterQuality {
            status: WaterStatus::Safe,
            ecoli_count: Some(50),
            sample_date: today,
            advisory_reason: None,
            fetched_at: Utc::now(),
        };

        assert_eq!(
            wq.effective_status(),
            WaterStatus::Safe,
            "Fresh safe water should stay safe"
        );
    }

    #[test]
    fn test_tide_info_hourly_heights_returns_16_values() {
        let tide_info = TideInfo {
            current_height: 2.5,
            tide_state: TideState::Rising,
            next_high: Some(TideEvent {
                time: Local::now(),
                height: 4.2,
            }),
            next_low: Some(TideEvent {
                time: Local::now(),
                height: 0.8,
            }),
            fetched_at: Utc::now(),
        };

        let heights = tide_info.hourly_heights(4.8);
        assert_eq!(heights.len(), 16, "Should return 16 heights (6am-9pm)");
    }

    #[test]
    fn test_tide_info_hourly_heights_within_range() {
        let tide_info = TideInfo {
            current_height: 2.5,
            tide_state: TideState::Rising,
            next_high: Some(TideEvent {
                time: Local::now(),
                height: 4.2,
            }),
            next_low: None,
            fetched_at: Utc::now(),
        };

        let heights = tide_info.hourly_heights(4.8);
        for (i, height) in heights.iter().enumerate() {
            assert!(
                *height >= 0.0 && *height <= 4.8,
                "Height at index {} should be in range 0-4.8, got {}",
                i,
                height
            );
        }
    }

    #[test]
    fn test_hourly_forecast_creation() {
        let forecast = HourlyForecast {
            hour: 14,
            temperature: 22.5,
            feels_like: 24.0,
            condition: WeatherCondition::PartlyCloudy,
            wind: 12.5,
            wind_direction: "NW".to_string(),
            uv: 6.0,
            precipitation_chance: 20,
        };

        assert_eq!(forecast.hour, 14);
        assert!((forecast.temperature - 22.5).abs() < 0.01);
        assert!((forecast.feels_like - 24.0).abs() < 0.01);
        assert_eq!(forecast.condition, WeatherCondition::PartlyCloudy);
        assert!((forecast.wind - 12.5).abs() < 0.01);
        assert_eq!(forecast.wind_direction, "NW");
        assert!((forecast.uv - 6.0).abs() < 0.01);
        assert_eq!(forecast.precipitation_chance, 20);
    }

    #[test]
    fn test_hourly_forecast_serialization_roundtrip() {
        let forecast = HourlyForecast {
            hour: 9,
            temperature: 18.0,
            feels_like: 17.5,
            condition: WeatherCondition::Clear,
            wind: 8.0,
            wind_direction: "E".to_string(),
            uv: 3.0,
            precipitation_chance: 0,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&forecast).expect("Failed to serialize HourlyForecast");

        // Deserialize back
        let deserialized: HourlyForecast =
            serde_json::from_str(&json).expect("Failed to deserialize HourlyForecast");

        assert_eq!(deserialized.hour, forecast.hour);
        assert!((deserialized.temperature - 18.0).abs() < 0.01);
        assert!((deserialized.feels_like - 17.5).abs() < 0.01);
        assert_eq!(deserialized.condition, WeatherCondition::Clear);
        assert!((deserialized.wind - 8.0).abs() < 0.01);
        assert_eq!(deserialized.wind_direction, "E");
        assert!((deserialized.uv - 3.0).abs() < 0.01);
        assert_eq!(deserialized.precipitation_chance, 0);
    }

    #[test]
    fn test_weather_with_hourly_forecasts() {
        let hourly_forecasts = vec![
            HourlyForecast {
                hour: 10,
                temperature: 20.0,
                feels_like: 21.0,
                condition: WeatherCondition::Clear,
                wind: 10.0,
                wind_direction: "N".to_string(),
                uv: 5.0,
                precipitation_chance: 0,
            },
            HourlyForecast {
                hour: 11,
                temperature: 22.0,
                feels_like: 23.0,
                condition: WeatherCondition::PartlyCloudy,
                wind: 12.0,
                wind_direction: "NE".to_string(),
                uv: 6.0,
                precipitation_chance: 10,
            },
        ];

        let weather = Weather {
            temperature: 20.0,
            feels_like: 21.0,
            condition: WeatherCondition::Clear,
            humidity: 60,
            wind: 10.0,
            uv: 5.0,
            sunrise: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(20, 30, 0).unwrap(),
            fetched_at: Utc::now(),
            hourly: hourly_forecasts,
        };

        assert_eq!(weather.hourly.len(), 2);
        assert_eq!(weather.hourly[0].hour, 10);
        assert_eq!(weather.hourly[1].hour, 11);
    }

    #[test]
    fn test_weather_with_hourly_serialization_roundtrip() {
        let weather = Weather {
            temperature: 22.5,
            feels_like: 24.0,
            condition: WeatherCondition::PartlyCloudy,
            humidity: 65,
            wind: 12.5,
            uv: 6.0,
            sunrise: NaiveTime::from_hms_opt(5, 30, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(21, 15, 0).unwrap(),
            fetched_at: Utc::now(),
            hourly: vec![HourlyForecast {
                hour: 14,
                temperature: 24.0,
                feels_like: 25.0,
                condition: WeatherCondition::Clear,
                wind: 10.0,
                wind_direction: "SW".to_string(),
                uv: 7.0,
                precipitation_chance: 5,
            }],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&weather).expect("Failed to serialize Weather with hourly");

        // Deserialize back
        let deserialized: Weather =
            serde_json::from_str(&json).expect("Failed to deserialize Weather with hourly");

        assert!((deserialized.temperature - 22.5).abs() < 0.01);
        assert_eq!(deserialized.hourly.len(), 1);
        assert_eq!(deserialized.hourly[0].hour, 14);
        assert!((deserialized.hourly[0].temperature - 24.0).abs() < 0.01);
        assert_eq!(deserialized.hourly[0].wind_direction, "SW");
    }

    #[test]
    fn test_weather_hourly_default_backwards_compat() {
        // Test that Weather can be deserialized without hourly field (backwards compatibility)
        let json_without_hourly = r#"{
            "temperature": 22.5,
            "feels_like": 24.0,
            "condition": "PartlyCloudy",
            "humidity": 65,
            "wind": 12.5,
            "uv": 6.0,
            "sunrise": "05:30:00",
            "sunset": "21:15:00",
            "fetched_at": "2024-07-15T14:00:00Z"
        }"#;

        let weather: Weather = serde_json::from_str(json_without_hourly)
            .expect("Failed to deserialize Weather without hourly field");

        // hourly should default to empty vec
        assert!(weather.hourly.is_empty());
        assert!((weather.temperature - 22.5).abs() < 0.01);
    }
}
