//! Open-Meteo weather API client
//!
//! This module provides functionality to fetch weather data from the Open-Meteo API
//! and parse it into our Weather data structures.

use chrono::{NaiveTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

use super::{Weather, WeatherCondition};

/// Base URL for the Open-Meteo API
const OPEN_METEO_BASE_URL: &str = "https://api.open-meteo.com/v1/forecast";

/// Errors that can occur when fetching weather data
#[derive(Debug, Error)]
pub enum WeatherError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    /// Failed to parse JSON response
    #[error("Failed to parse JSON response: {0}")]
    ParseError(#[from] serde_json::Error),

    /// Missing expected field in response
    #[error("Missing expected field in response: {0}")]
    MissingField(String),

    /// Invalid time format in response
    #[error("Invalid time format: {0}")]
    InvalidTimeFormat(String),
}

/// Client for fetching weather data from Open-Meteo API
#[derive(Debug, Clone)]
pub struct WeatherClient {
    client: Client,
    timezone: String,
}

impl Default for WeatherClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherClient {
    /// Create a new WeatherClient with default settings
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            timezone: "America/Vancouver".to_string(),
        }
    }

    /// Create a new WeatherClient with a custom HTTP client
    pub fn with_client(client: Client) -> Self {
        Self {
            client,
            timezone: "America/Vancouver".to_string(),
        }
    }

    /// Create a new WeatherClient with a custom timezone
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = timezone.into();
        self
    }

    /// Fetch weather data for the given coordinates
    ///
    /// # Arguments
    /// * `lat` - Latitude coordinate
    /// * `lon` - Longitude coordinate
    ///
    /// # Returns
    /// * `Ok(Weather)` - Weather data for the location
    /// * `Err(WeatherError)` - If the request or parsing fails
    pub async fn fetch_weather(&self, lat: f64, lon: f64) -> Result<Weather, WeatherError> {
        let url = format!(
            "{}?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m&daily=sunrise,sunset,uv_index_max&timezone={}",
            OPEN_METEO_BASE_URL, lat, lon, self.timezone
        );

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        let api_response: OpenMeteoResponse = serde_json::from_str(&text)?;

        self.parse_response(api_response)
    }

    /// Parse the Open-Meteo API response into a Weather struct
    fn parse_response(&self, response: OpenMeteoResponse) -> Result<Weather, WeatherError> {
        let current = response.current;
        let daily = response.daily;

        // Extract temperature and weather data
        let temperature = current.temperature_2m;
        let feels_like = current.apparent_temperature;
        let humidity = current.relative_humidity_2m as u8;
        let wind = current.wind_speed_10m;

        // Map weather code to condition
        let condition = weather_code_to_condition(current.weather_code);

        // Extract UV index (first day's max)
        let uv = daily
            .uv_index_max
            .first()
            .copied()
            .ok_or_else(|| WeatherError::MissingField("uv_index_max".to_string()))?;

        // Extract sunrise time (first day)
        let sunrise_str = daily
            .sunrise
            .first()
            .ok_or_else(|| WeatherError::MissingField("sunrise".to_string()))?;
        let sunrise = parse_time(sunrise_str)?;

        // Extract sunset time (first day)
        let sunset_str = daily
            .sunset
            .first()
            .ok_or_else(|| WeatherError::MissingField("sunset".to_string()))?;
        let sunset = parse_time(sunset_str)?;

        Ok(Weather {
            temperature,
            feels_like,
            condition,
            humidity,
            wind,
            uv,
            sunrise,
            sunset,
            fetched_at: Utc::now(),
        })
    }
}

/// Parse a time string in ISO 8601 format (e.g., "2024-07-15T05:30") to NaiveTime
fn parse_time(time_str: &str) -> Result<NaiveTime, WeatherError> {
    // Extract the time portion after 'T'
    let time_part = time_str
        .split('T')
        .nth(1)
        .ok_or_else(|| WeatherError::InvalidTimeFormat(time_str.to_string()))?;

    NaiveTime::parse_from_str(time_part, "%H:%M")
        .map_err(|_| WeatherError::InvalidTimeFormat(time_str.to_string()))
}

/// Map WMO weather code to WeatherCondition enum
///
/// Weather codes from WMO (World Meteorological Organization):
/// - 0: Clear sky
/// - 1-3: Partly cloudy
/// - 45, 48: Fog
/// - 51-55: Drizzle
/// - 56-57: Freezing drizzle
/// - 61-65: Rain
/// - 66-67: Freezing rain
/// - 71-77: Snow
/// - 80-82: Rain showers
/// - 85-86: Snow showers
/// - 95-99: Thunderstorm
pub fn weather_code_to_condition(code: u8) -> WeatherCondition {
    match code {
        0 => WeatherCondition::Clear,
        1..=3 => WeatherCondition::PartlyCloudy,
        45 | 48 => WeatherCondition::Fog,
        51..=55 | 61..=65 | 80..=82 => WeatherCondition::Rain,
        56..=57 | 66..=67 => WeatherCondition::Showers,
        71..=77 | 85..=86 => WeatherCondition::Snow,
        95..=99 => WeatherCondition::Thunderstorm,
        _ => WeatherCondition::Cloudy, // Default for unknown codes
    }
}

/// Open-Meteo API response structure
#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current: CurrentWeather,
    daily: DailyWeather,
}

/// Current weather data from Open-Meteo
#[derive(Debug, Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    relative_humidity_2m: f64,
    apparent_temperature: f64,
    weather_code: u8,
    wind_speed_10m: f64,
    #[allow(dead_code)]
    wind_direction_10m: f64,
}

/// Daily weather data from Open-Meteo
#[derive(Debug, Deserialize)]
struct DailyWeather {
    sunrise: Vec<String>,
    sunset: Vec<String>,
    uv_index_max: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample valid Open-Meteo API response
    const VALID_RESPONSE: &str = r#"{
        "latitude": 49.28,
        "longitude": -123.12,
        "generationtime_ms": 0.123,
        "utc_offset_seconds": -25200,
        "timezone": "America/Vancouver",
        "timezone_abbreviation": "PDT",
        "elevation": 5.0,
        "current_units": {
            "time": "iso8601",
            "interval": "seconds",
            "temperature_2m": "°C",
            "relative_humidity_2m": "%",
            "apparent_temperature": "°C",
            "weather_code": "wmo code",
            "wind_speed_10m": "km/h",
            "wind_direction_10m": "°"
        },
        "current": {
            "time": "2024-07-15T14:00",
            "interval": 900,
            "temperature_2m": 22.5,
            "relative_humidity_2m": 65,
            "apparent_temperature": 23.8,
            "weather_code": 2,
            "wind_speed_10m": 12.5,
            "wind_direction_10m": 270
        },
        "daily_units": {
            "time": "iso8601",
            "sunrise": "iso8601",
            "sunset": "iso8601",
            "uv_index_max": ""
        },
        "daily": {
            "time": ["2024-07-15"],
            "sunrise": ["2024-07-15T05:30"],
            "sunset": ["2024-07-15T21:15"],
            "uv_index_max": [7.5]
        }
    }"#;

    #[test]
    fn test_parse_valid_response() {
        let response: OpenMeteoResponse =
            serde_json::from_str(VALID_RESPONSE).expect("Failed to parse valid response");

        let client = WeatherClient::new();
        let weather = client
            .parse_response(response)
            .expect("Failed to parse weather");

        assert!((weather.temperature - 22.5).abs() < 0.01);
        assert!((weather.feels_like - 23.8).abs() < 0.01);
        assert_eq!(weather.condition, WeatherCondition::PartlyCloudy);
        assert_eq!(weather.humidity, 65);
        assert!((weather.wind - 12.5).abs() < 0.01);
        assert!((weather.uv - 7.5).abs() < 0.01);
        assert_eq!(weather.sunrise, NaiveTime::from_hms_opt(5, 30, 0).unwrap());
        assert_eq!(weather.sunset, NaiveTime::from_hms_opt(21, 15, 0).unwrap());
    }

    #[test]
    fn test_weather_code_mapping() {
        // Clear
        assert_eq!(weather_code_to_condition(0), WeatherCondition::Clear);

        // Partly cloudy
        assert_eq!(weather_code_to_condition(1), WeatherCondition::PartlyCloudy);
        assert_eq!(weather_code_to_condition(2), WeatherCondition::PartlyCloudy);
        assert_eq!(weather_code_to_condition(3), WeatherCondition::PartlyCloudy);

        // Fog
        assert_eq!(weather_code_to_condition(45), WeatherCondition::Fog);
        assert_eq!(weather_code_to_condition(48), WeatherCondition::Fog);

        // Rain (drizzle, rain, rain showers)
        assert_eq!(weather_code_to_condition(51), WeatherCondition::Rain);
        assert_eq!(weather_code_to_condition(53), WeatherCondition::Rain);
        assert_eq!(weather_code_to_condition(55), WeatherCondition::Rain);
        assert_eq!(weather_code_to_condition(61), WeatherCondition::Rain);
        assert_eq!(weather_code_to_condition(63), WeatherCondition::Rain);
        assert_eq!(weather_code_to_condition(65), WeatherCondition::Rain);
        assert_eq!(weather_code_to_condition(80), WeatherCondition::Rain);
        assert_eq!(weather_code_to_condition(81), WeatherCondition::Rain);
        assert_eq!(weather_code_to_condition(82), WeatherCondition::Rain);

        // Showers (freezing drizzle, freezing rain)
        assert_eq!(weather_code_to_condition(56), WeatherCondition::Showers);
        assert_eq!(weather_code_to_condition(57), WeatherCondition::Showers);
        assert_eq!(weather_code_to_condition(66), WeatherCondition::Showers);
        assert_eq!(weather_code_to_condition(67), WeatherCondition::Showers);

        // Snow
        assert_eq!(weather_code_to_condition(71), WeatherCondition::Snow);
        assert_eq!(weather_code_to_condition(73), WeatherCondition::Snow);
        assert_eq!(weather_code_to_condition(75), WeatherCondition::Snow);
        assert_eq!(weather_code_to_condition(77), WeatherCondition::Snow);
        assert_eq!(weather_code_to_condition(85), WeatherCondition::Snow);
        assert_eq!(weather_code_to_condition(86), WeatherCondition::Snow);

        // Thunderstorm
        assert_eq!(weather_code_to_condition(95), WeatherCondition::Thunderstorm);
        assert_eq!(weather_code_to_condition(96), WeatherCondition::Thunderstorm);
        assert_eq!(weather_code_to_condition(99), WeatherCondition::Thunderstorm);

        // Unknown codes default to Cloudy
        assert_eq!(weather_code_to_condition(100), WeatherCondition::Cloudy);
        assert_eq!(weather_code_to_condition(255), WeatherCondition::Cloudy);
    }

    #[test]
    fn test_parse_time() {
        let time = parse_time("2024-07-15T05:30").expect("Failed to parse time");
        assert_eq!(time, NaiveTime::from_hms_opt(5, 30, 0).unwrap());

        let time = parse_time("2024-07-15T21:15").expect("Failed to parse time");
        assert_eq!(time, NaiveTime::from_hms_opt(21, 15, 0).unwrap());

        let time = parse_time("2024-07-15T00:00").expect("Failed to parse time");
        assert_eq!(time, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_time_invalid() {
        // Missing T separator
        assert!(parse_time("2024-07-15 05:30").is_err());

        // Invalid format
        assert!(parse_time("not a time").is_err());
    }

    #[test]
    fn test_parse_malformed_json() {
        let malformed = "{ invalid json }";
        let result: Result<OpenMeteoResponse, _> = serde_json::from_str(malformed);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_fields() {
        let missing_daily = r#"{
            "current": {
                "temperature_2m": 22.5,
                "relative_humidity_2m": 65,
                "apparent_temperature": 23.8,
                "weather_code": 2,
                "wind_speed_10m": 12.5,
                "wind_direction_10m": 270
            }
        }"#;

        let result: Result<OpenMeteoResponse, _> = serde_json::from_str(missing_daily);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_daily_arrays() {
        let empty_arrays = r#"{
            "current": {
                "temperature_2m": 22.5,
                "relative_humidity_2m": 65,
                "apparent_temperature": 23.8,
                "weather_code": 2,
                "wind_speed_10m": 12.5,
                "wind_direction_10m": 270
            },
            "daily": {
                "sunrise": [],
                "sunset": [],
                "uv_index_max": []
            }
        }"#;

        let response: OpenMeteoResponse =
            serde_json::from_str(empty_arrays).expect("Failed to parse");
        let client = WeatherClient::new();
        let result = client.parse_response(response);

        assert!(result.is_err());
        match result {
            Err(WeatherError::MissingField(field)) => {
                assert_eq!(field, "uv_index_max");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_weather_client_default() {
        let client = WeatherClient::default();
        assert_eq!(client.timezone, "America/Vancouver");
    }

    #[test]
    fn test_weather_client_with_timezone() {
        let client = WeatherClient::new().with_timezone("Europe/London");
        assert_eq!(client.timezone, "Europe/London");
    }
}
