//! Open-Meteo weather API client
//!
//! This module provides functionality to fetch weather data from the Open-Meteo API
//! and parse it into our Weather data structures.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{HourlyForecast, Weather, WeatherCondition};

/// Base URL for the Open-Meteo API
const OPEN_METEO_BASE_URL: &str = "https://api.open-meteo.com/v1/forecast";

/// Hourly weather forecast data from Open-Meteo API (internal structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ApiHourlyForecast {
    /// Time of the forecast
    pub time: NaiveDateTime,
    /// Temperature in Celsius
    pub temperature: f64,
    /// WMO weather code
    pub weather_code: u8,
    /// Wind speed in km/h
    pub wind_speed: f64,
    /// UV index
    pub uv_index: f64,
}

/// Combined weather data including current conditions and hourly forecasts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct WeatherData {
    /// Current weather conditions
    pub current: Weather,
    /// Hourly forecasts for the next 48 hours
    pub hourly: Vec<ApiHourlyForecast>,
}

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
    #[allow(dead_code)]
    pub fn with_client(client: Client) -> Self {
        Self {
            client,
            timezone: "America/Vancouver".to_string(),
        }
    }

    /// Create a new WeatherClient with a custom timezone
    #[allow(dead_code)]
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
    /// * `Ok(Weather)` - Weather data for the location including hourly forecasts for today
    /// * `Err(WeatherError)` - If the request or parsing fails
    pub async fn fetch_weather(&self, lat: f64, lon: f64) -> Result<Weather, WeatherError> {
        let url = format!(
            "{}?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m&daily=sunrise,sunset,uv_index_max&hourly=temperature_2m,apparent_temperature,weathercode,windspeed_10m,winddirection_10m,uv_index,precipitation_probability&forecast_days=2&timezone={}",
            OPEN_METEO_BASE_URL, lat, lon, self.timezone
        );

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        let api_response: OpenMeteoResponseFull = serde_json::from_str(&text)?;

        self.parse_response_full(api_response)
    }

    /// Fetch weather data with 48-hour hourly forecasts for the given coordinates
    ///
    /// # Arguments
    /// * `lat` - Latitude coordinate
    /// * `lon` - Longitude coordinate
    ///
    /// # Returns
    /// * `Ok(WeatherData)` - Weather data with current conditions and hourly forecasts
    /// * `Err(WeatherError)` - If the request or parsing fails
    #[allow(dead_code)]
    pub async fn fetch_weather_with_hourly(
        &self,
        lat: f64,
        lon: f64,
    ) -> Result<WeatherData, WeatherError> {
        let url = format!(
            "{}?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m&daily=sunrise,sunset,uv_index_max&hourly=temperature_2m,weathercode,windspeed_10m,uv_index&forecast_hours=48&timezone={}",
            OPEN_METEO_BASE_URL, lat, lon, self.timezone
        );

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        let api_response: OpenMeteoResponseWithHourly = serde_json::from_str(&text)?;

        self.parse_response_with_hourly(api_response)
    }

    /// Parse the Open-Meteo API response into a Weather struct (kept for test compatibility)
    #[allow(dead_code)]
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
            hourly: Vec::new(),
        })
    }

    /// Parse the Open-Meteo API response with hourly data into a WeatherData struct
    fn parse_response_with_hourly(
        &self,
        response: OpenMeteoResponseWithHourly,
    ) -> Result<WeatherData, WeatherError> {
        let current = response.current;
        let daily = response.daily;
        let hourly = response.hourly;

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

        let current_weather = Weather {
            temperature,
            feels_like,
            condition,
            humidity,
            wind,
            uv,
            sunrise,
            sunset,
            fetched_at: Utc::now(),
            hourly: Vec::new(),
        };

        // Parse hourly forecasts
        let hourly_forecasts = self.parse_hourly_data(&hourly)?;

        Ok(WeatherData {
            current: current_weather,
            hourly: hourly_forecasts,
        })
    }

    /// Parse hourly weather data arrays into ApiHourlyForecast structs
    fn parse_hourly_data(
        &self,
        hourly: &HourlyWeather,
    ) -> Result<Vec<ApiHourlyForecast>, WeatherError> {
        let len = hourly.time.len();

        // Validate that all arrays have the same length
        if hourly.temperature_2m.len() != len
            || hourly.weathercode.len() != len
            || hourly.windspeed_10m.len() != len
            || hourly.uv_index.len() != len
        {
            return Err(WeatherError::MissingField(
                "hourly arrays have inconsistent lengths".to_string(),
            ));
        }

        let mut forecasts = Vec::with_capacity(len);

        for i in 0..len {
            let time = parse_datetime(&hourly.time[i])?;
            forecasts.push(ApiHourlyForecast {
                time,
                temperature: hourly.temperature_2m[i],
                weather_code: hourly.weathercode[i],
                wind_speed: hourly.windspeed_10m[i],
                uv_index: hourly.uv_index[i],
            });
        }

        Ok(forecasts)
    }

    /// Parse the Open-Meteo API response with full hourly data into a Weather struct
    /// This populates the Weather.hourly field with today's hourly forecasts
    fn parse_response_full(
        &self,
        response: OpenMeteoResponseFull,
    ) -> Result<Weather, WeatherError> {
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

        // Extract today's date from the first daily time entry
        let today = daily
            .sunrise
            .first()
            .and_then(|s| s.split('T').next())
            .and_then(|date_str| NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok())
            .unwrap_or_else(|| chrono::Local::now().date_naive());

        // Parse hourly forecasts for today only, defaulting to empty vec if missing
        let hourly = match response.hourly {
            Some(hourly_data) => self.parse_hourly_data_full(&hourly_data, today),
            None => Vec::new(),
        };

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
            hourly,
        })
    }

    /// Parse full hourly weather data arrays into HourlyForecast structs, filtered to today only
    fn parse_hourly_data_full(
        &self,
        hourly: &HourlyWeatherFull,
        today: NaiveDate,
    ) -> Vec<HourlyForecast> {
        let len = hourly.time.len();

        // Validate that required arrays have the same length; if not, return empty
        if hourly.temperature_2m.len() != len
            || hourly.weathercode.len() != len
            || hourly.windspeed_10m.len() != len
            || hourly.uv_index.len() != len
        {
            return Vec::new();
        }

        let mut forecasts = Vec::new();

        for i in 0..len {
            // Parse datetime from time string
            let time = match parse_datetime(&hourly.time[i]) {
                Ok(dt) => dt,
                Err(_) => continue, // Skip invalid times
            };

            // Filter to today's date only
            if time.date() != today {
                continue;
            }

            // Get feels_like, defaulting to temperature if not available
            let feels_like = hourly
                .apparent_temperature
                .get(i)
                .copied()
                .unwrap_or(hourly.temperature_2m[i]);

            // Get wind direction, defaulting to 0 (N) if not available
            let wind_direction_degrees = hourly.winddirection_10m.get(i).copied().unwrap_or(0.0);

            // Get precipitation probability, defaulting to 0 if not available
            let precipitation_chance = hourly
                .precipitation_probability
                .get(i)
                .and_then(|v| *v)
                .unwrap_or(0);

            forecasts.push(HourlyForecast {
                hour: time.hour() as u8,
                temperature: hourly.temperature_2m[i],
                feels_like,
                condition: weather_code_to_condition(hourly.weathercode[i]),
                wind: hourly.windspeed_10m[i],
                wind_direction: degrees_to_direction(wind_direction_degrees),
                uv: hourly.uv_index[i],
                precipitation_chance,
            });
        }

        forecasts
    }
}

/// Convert wind direction in degrees to compass direction string
fn degrees_to_direction(degrees: f64) -> String {
    // Normalize to 0-360 range
    let deg = ((degrees % 360.0) + 360.0) % 360.0;

    // Map to 16 compass points
    let directions = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];

    // Each direction covers 22.5 degrees, offset by 11.25 to center
    let index = ((deg + 11.25) / 22.5) as usize % 16;
    directions[index].to_string()
}

/// Parse a datetime string in ISO 8601 format (e.g., "2024-07-15T05:30") to NaiveDateTime
#[allow(dead_code)]
fn parse_datetime(datetime_str: &str) -> Result<NaiveDateTime, WeatherError> {
    NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M")
        .map_err(|_| WeatherError::InvalidTimeFormat(datetime_str.to_string()))
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

/// Open-Meteo API response structure (used in tests for backward compatibility)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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

/// Open-Meteo API response structure with hourly data
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenMeteoResponseWithHourly {
    current: CurrentWeather,
    daily: DailyWeather,
    hourly: HourlyWeather,
}

/// Hourly weather data from Open-Meteo
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HourlyWeather {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    weathercode: Vec<u8>,
    windspeed_10m: Vec<f64>,
    uv_index: Vec<f64>,
}

/// Open-Meteo API response structure with full hourly data for fetch_weather
#[derive(Debug, Deserialize)]
struct OpenMeteoResponseFull {
    current: CurrentWeather,
    daily: DailyWeather,
    #[serde(default)]
    hourly: Option<HourlyWeatherFull>,
}

/// Hourly weather data from Open-Meteo with all fields needed for HourlyForecast
#[derive(Debug, Deserialize)]
struct HourlyWeatherFull {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    #[serde(default)]
    apparent_temperature: Vec<f64>,
    weathercode: Vec<u8>,
    windspeed_10m: Vec<f64>,
    #[serde(default)]
    winddirection_10m: Vec<f64>,
    uv_index: Vec<f64>,
    #[serde(default)]
    precipitation_probability: Vec<Option<u8>>,
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
        assert_eq!(
            weather_code_to_condition(95),
            WeatherCondition::Thunderstorm
        );
        assert_eq!(
            weather_code_to_condition(96),
            WeatherCondition::Thunderstorm
        );
        assert_eq!(
            weather_code_to_condition(99),
            WeatherCondition::Thunderstorm
        );

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

    /// Sample valid Open-Meteo API response with hourly data
    const VALID_RESPONSE_WITH_HOURLY: &str = r#"{
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
        },
        "hourly_units": {
            "time": "iso8601",
            "temperature_2m": "°C",
            "weathercode": "wmo code",
            "windspeed_10m": "km/h",
            "uv_index": ""
        },
        "hourly": {
            "time": [
                "2024-07-15T00:00", "2024-07-15T01:00", "2024-07-15T02:00", "2024-07-15T03:00",
                "2024-07-15T04:00", "2024-07-15T05:00", "2024-07-15T06:00", "2024-07-15T07:00",
                "2024-07-15T08:00", "2024-07-15T09:00", "2024-07-15T10:00", "2024-07-15T11:00",
                "2024-07-15T12:00", "2024-07-15T13:00", "2024-07-15T14:00", "2024-07-15T15:00",
                "2024-07-15T16:00", "2024-07-15T17:00", "2024-07-15T18:00", "2024-07-15T19:00",
                "2024-07-15T20:00", "2024-07-15T21:00", "2024-07-15T22:00", "2024-07-15T23:00",
                "2024-07-16T00:00", "2024-07-16T01:00", "2024-07-16T02:00", "2024-07-16T03:00",
                "2024-07-16T04:00", "2024-07-16T05:00", "2024-07-16T06:00", "2024-07-16T07:00",
                "2024-07-16T08:00", "2024-07-16T09:00", "2024-07-16T10:00", "2024-07-16T11:00",
                "2024-07-16T12:00", "2024-07-16T13:00", "2024-07-16T14:00", "2024-07-16T15:00",
                "2024-07-16T16:00", "2024-07-16T17:00", "2024-07-16T18:00", "2024-07-16T19:00",
                "2024-07-16T20:00", "2024-07-16T21:00", "2024-07-16T22:00", "2024-07-16T23:00"
            ],
            "temperature_2m": [
                15.2, 14.8, 14.5, 14.2, 14.0, 14.5, 16.0, 18.5,
                20.0, 21.5, 22.5, 23.5, 24.0, 24.5, 24.8, 24.5,
                24.0, 23.0, 21.5, 20.0, 18.5, 17.5, 16.5, 15.8,
                15.5, 15.2, 14.8, 14.5, 14.2, 14.8, 16.5, 19.0,
                20.5, 22.0, 23.0, 24.0, 24.5, 25.0, 25.2, 25.0,
                24.5, 23.5, 22.0, 20.5, 19.0, 18.0, 17.0, 16.2
            ],
            "weathercode": [
                0, 0, 0, 0, 0, 1, 1, 1,
                2, 2, 2, 3, 3, 2, 2, 2,
                1, 1, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 1, 1, 2,
                2, 3, 3, 3, 2, 2, 1, 1,
                1, 0, 0, 0, 0, 0, 0, 0
            ],
            "windspeed_10m": [
                5.2, 4.8, 4.5, 4.2, 4.0, 5.5, 7.0, 9.5,
                11.0, 12.5, 13.5, 14.5, 15.0, 15.5, 15.8, 15.5,
                15.0, 14.0, 12.5, 11.0, 9.5, 8.5, 7.5, 6.8,
                6.5, 6.2, 5.8, 5.5, 5.2, 5.8, 7.5, 10.0,
                11.5, 13.0, 14.0, 15.0, 15.5, 16.0, 16.2, 16.0,
                15.5, 14.5, 13.0, 11.5, 10.0, 9.0, 8.0, 7.2
            ],
            "uv_index": [
                0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.5, 3.0,
                4.5, 6.0, 7.0, 7.5, 7.8, 7.5, 7.0, 6.0,
                4.5, 3.0, 1.5, 0.5, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.5, 3.5,
                5.0, 6.5, 7.5, 8.0, 8.2, 8.0, 7.5, 6.5,
                5.0, 3.5, 1.5, 0.5, 0.0, 0.0, 0.0, 0.0
            ]
        }
    }"#;

    #[test]
    fn test_parse_valid_hourly_response() {
        let response: OpenMeteoResponseWithHourly =
            serde_json::from_str(VALID_RESPONSE_WITH_HOURLY)
                .expect("Failed to parse valid response with hourly");

        let client = WeatherClient::new();
        let weather_data = client
            .parse_response_with_hourly(response)
            .expect("Failed to parse weather data with hourly");

        // Verify current weather
        assert!((weather_data.current.temperature - 22.5).abs() < 0.01);
        assert!((weather_data.current.feels_like - 23.8).abs() < 0.01);
        assert_eq!(
            weather_data.current.condition,
            WeatherCondition::PartlyCloudy
        );
        assert_eq!(weather_data.current.humidity, 65);
        assert!((weather_data.current.wind - 12.5).abs() < 0.01);
        assert!((weather_data.current.uv - 7.5).abs() < 0.01);

        // Verify hourly array length (48 hours)
        assert_eq!(weather_data.hourly.len(), 48);
    }

    #[test]
    fn test_hourly_forecast_fields_correctly_extracted() {
        let response: OpenMeteoResponseWithHourly =
            serde_json::from_str(VALID_RESPONSE_WITH_HOURLY)
                .expect("Failed to parse valid response with hourly");

        let client = WeatherClient::new();
        let weather_data = client
            .parse_response_with_hourly(response)
            .expect("Failed to parse weather data with hourly");

        // Check first hour
        let first_hour = &weather_data.hourly[0];
        assert_eq!(
            first_hour.time,
            NaiveDateTime::parse_from_str("2024-07-15T00:00", "%Y-%m-%dT%H:%M").unwrap()
        );
        assert!((first_hour.temperature - 15.2).abs() < 0.01);
        assert_eq!(first_hour.weather_code, 0);
        assert!((first_hour.wind_speed - 5.2).abs() < 0.01);
        assert!((first_hour.uv_index - 0.0).abs() < 0.01);

        // Check mid-day hour (index 14 = 2pm on first day)
        let midday = &weather_data.hourly[14];
        assert_eq!(
            midday.time,
            NaiveDateTime::parse_from_str("2024-07-15T14:00", "%Y-%m-%dT%H:%M").unwrap()
        );
        assert!((midday.temperature - 24.8).abs() < 0.01);
        assert_eq!(midday.weather_code, 2);
        assert!((midday.wind_speed - 15.8).abs() < 0.01);
        assert!((midday.uv_index - 7.0).abs() < 0.01);

        // Check last hour (48th entry, index 47)
        let last_hour = &weather_data.hourly[47];
        assert_eq!(
            last_hour.time,
            NaiveDateTime::parse_from_str("2024-07-16T23:00", "%Y-%m-%dT%H:%M").unwrap()
        );
        assert!((last_hour.temperature - 16.2).abs() < 0.01);
        assert_eq!(last_hour.weather_code, 0);
        assert!((last_hour.wind_speed - 7.2).abs() < 0.01);
        assert!((last_hour.uv_index - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_hourly_array_has_expected_length() {
        let response: OpenMeteoResponseWithHourly =
            serde_json::from_str(VALID_RESPONSE_WITH_HOURLY)
                .expect("Failed to parse valid response with hourly");

        let client = WeatherClient::new();
        let weather_data = client
            .parse_response_with_hourly(response)
            .expect("Failed to parse weather data with hourly");

        // Should have exactly 48 hourly entries
        assert_eq!(weather_data.hourly.len(), 48);

        // Verify each entry has valid time progression (1 hour apart)
        for (i, hour) in weather_data.hourly.iter().enumerate().skip(1) {
            let prev_hour = &weather_data.hourly[i - 1];
            let diff = hour.time.signed_duration_since(prev_hour.time);
            assert_eq!(
                diff.num_hours(),
                1,
                "Hour {} should be 1 hour after hour {}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn test_parse_datetime() {
        let dt = parse_datetime("2024-07-15T14:30").expect("Failed to parse datetime");
        assert_eq!(
            dt,
            NaiveDateTime::parse_from_str("2024-07-15T14:30", "%Y-%m-%dT%H:%M").unwrap()
        );

        let dt = parse_datetime("2024-07-15T00:00").expect("Failed to parse datetime");
        assert_eq!(
            dt,
            NaiveDateTime::parse_from_str("2024-07-15T00:00", "%Y-%m-%dT%H:%M").unwrap()
        );
    }

    #[test]
    fn test_parse_datetime_invalid() {
        // Missing T separator
        assert!(parse_datetime("2024-07-15 14:30").is_err());

        // Invalid format
        assert!(parse_datetime("not a datetime").is_err());
    }

    #[test]
    fn test_api_hourly_forecast_serialization() {
        let forecast = ApiHourlyForecast {
            time: NaiveDateTime::parse_from_str("2024-07-15T14:00", "%Y-%m-%dT%H:%M").unwrap(),
            temperature: 22.5,
            weather_code: 2,
            wind_speed: 12.5,
            uv_index: 7.0,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&forecast).expect("Failed to serialize ApiHourlyForecast");

        // Deserialize back
        let deserialized: ApiHourlyForecast =
            serde_json::from_str(&json).expect("Failed to deserialize ApiHourlyForecast");

        assert_eq!(deserialized.time, forecast.time);
        assert!((deserialized.temperature - 22.5).abs() < 0.01);
        assert_eq!(deserialized.weather_code, 2);
        assert!((deserialized.wind_speed - 12.5).abs() < 0.01);
        assert!((deserialized.uv_index - 7.0).abs() < 0.01);
    }

    #[test]
    fn test_weather_data_serialization() {
        let weather_data = WeatherData {
            current: Weather {
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
            },
            hourly: vec![ApiHourlyForecast {
                time: NaiveDateTime::parse_from_str("2024-07-15T14:00", "%Y-%m-%dT%H:%M").unwrap(),
                temperature: 22.5,
                weather_code: 2,
                wind_speed: 12.5,
                uv_index: 7.0,
            }],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&weather_data).expect("Failed to serialize WeatherData");

        // Deserialize back
        let deserialized: WeatherData =
            serde_json::from_str(&json).expect("Failed to deserialize WeatherData");

        assert!((deserialized.current.temperature - 22.5).abs() < 0.01);
        assert_eq!(deserialized.hourly.len(), 1);
        assert!((deserialized.hourly[0].temperature - 22.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_hourly_with_inconsistent_array_lengths() {
        // Create hourly data with inconsistent array lengths
        let hourly = HourlyWeather {
            time: vec![
                "2024-07-15T00:00".to_string(),
                "2024-07-15T01:00".to_string(),
            ],
            temperature_2m: vec![15.0], // Only 1 element instead of 2
            weathercode: vec![0, 0],
            windspeed_10m: vec![5.0, 5.0],
            uv_index: vec![0.0, 0.0],
        };

        let client = WeatherClient::new();
        let result = client.parse_hourly_data(&hourly);

        assert!(result.is_err());
        match result {
            Err(WeatherError::MissingField(msg)) => {
                assert!(msg.contains("inconsistent lengths"));
            }
            _ => panic!("Expected MissingField error about inconsistent lengths"),
        }
    }

    #[test]
    fn test_existing_fetch_weather_still_parses_response() {
        // Verify that the original OpenMeteoResponse struct still works
        // This ensures backward compatibility of fetch_weather()
        let response: OpenMeteoResponse =
            serde_json::from_str(VALID_RESPONSE).expect("Failed to parse valid response");

        let client = WeatherClient::new();
        let weather = client
            .parse_response(response)
            .expect("Failed to parse weather");

        // Verify the basic weather data is still correctly parsed
        assert!((weather.temperature - 22.5).abs() < 0.01);
        assert_eq!(weather.condition, WeatherCondition::PartlyCloudy);
    }

    /// Sample valid Open-Meteo API response with full hourly data for parse_response_full
    const VALID_RESPONSE_FULL: &str = r#"{
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
            "time": ["2024-07-15", "2024-07-16"],
            "sunrise": ["2024-07-15T05:30", "2024-07-16T05:31"],
            "sunset": ["2024-07-15T21:15", "2024-07-16T21:14"],
            "uv_index_max": [7.5, 8.0]
        },
        "hourly_units": {
            "time": "iso8601",
            "temperature_2m": "°C",
            "apparent_temperature": "°C",
            "weathercode": "wmo code",
            "windspeed_10m": "km/h",
            "winddirection_10m": "°",
            "uv_index": "",
            "precipitation_probability": "%"
        },
        "hourly": {
            "time": [
                "2024-07-15T00:00", "2024-07-15T01:00", "2024-07-15T02:00", "2024-07-15T03:00",
                "2024-07-15T04:00", "2024-07-15T05:00", "2024-07-15T06:00", "2024-07-15T07:00",
                "2024-07-15T08:00", "2024-07-15T09:00", "2024-07-15T10:00", "2024-07-15T11:00",
                "2024-07-15T12:00", "2024-07-15T13:00", "2024-07-15T14:00", "2024-07-15T15:00",
                "2024-07-15T16:00", "2024-07-15T17:00", "2024-07-15T18:00", "2024-07-15T19:00",
                "2024-07-15T20:00", "2024-07-15T21:00", "2024-07-15T22:00", "2024-07-15T23:00",
                "2024-07-16T00:00", "2024-07-16T01:00", "2024-07-16T02:00", "2024-07-16T03:00"
            ],
            "temperature_2m": [
                15.2, 14.8, 14.5, 14.2, 14.0, 14.5, 16.0, 18.5,
                20.0, 21.5, 22.5, 23.5, 24.0, 24.5, 24.8, 24.5,
                24.0, 23.0, 21.5, 20.0, 18.5, 17.5, 16.5, 15.8,
                15.5, 15.2, 14.8, 14.5
            ],
            "apparent_temperature": [
                14.0, 13.5, 13.2, 12.9, 12.7, 13.2, 15.0, 17.5,
                19.0, 20.5, 21.5, 22.5, 23.0, 23.5, 23.8, 23.5,
                23.0, 22.0, 20.5, 19.0, 17.5, 16.5, 15.5, 14.8,
                14.5, 14.2, 13.8, 13.5
            ],
            "weathercode": [
                0, 0, 0, 0, 0, 1, 1, 1,
                2, 2, 2, 3, 3, 2, 2, 2,
                1, 1, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ],
            "windspeed_10m": [
                5.2, 4.8, 4.5, 4.2, 4.0, 5.5, 7.0, 9.5,
                11.0, 12.5, 13.5, 14.5, 15.0, 15.5, 15.8, 15.5,
                15.0, 14.0, 12.5, 11.0, 9.5, 8.5, 7.5, 6.8,
                6.5, 6.2, 5.8, 5.5
            ],
            "winddirection_10m": [
                0, 22.5, 45, 90, 135, 180, 225, 270,
                315, 337.5, 0, 45, 90, 135, 180, 225,
                270, 315, 0, 45, 90, 135, 180, 225,
                270, 315, 0, 45
            ],
            "uv_index": [
                0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.5, 3.0,
                4.5, 6.0, 7.0, 7.5, 7.8, 7.5, 7.0, 6.0,
                4.5, 3.0, 1.5, 0.5, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0
            ],
            "precipitation_probability": [
                0, 0, 0, 5, 10, 15, 20, 25,
                30, 25, 20, 15, 10, 5, 0, 0,
                5, 10, 15, 10, 5, 0, 0, 0,
                0, 0, 0, 5
            ]
        }
    }"#;

    #[test]
    fn test_parse_response_full_with_hourly() {
        let response: OpenMeteoResponseFull =
            serde_json::from_str(VALID_RESPONSE_FULL).expect("Failed to parse valid response full");

        let client = WeatherClient::new();
        let weather = client
            .parse_response_full(response)
            .expect("Failed to parse weather with full hourly");

        // Verify current weather
        assert!((weather.temperature - 22.5).abs() < 0.01);
        assert!((weather.feels_like - 23.8).abs() < 0.01);
        assert_eq!(weather.condition, WeatherCondition::PartlyCloudy);
        assert_eq!(weather.humidity, 65);
        assert!((weather.wind - 12.5).abs() < 0.01);
        assert!((weather.uv - 7.5).abs() < 0.01);

        // Verify hourly array - should only contain today's hours (24 hours)
        assert_eq!(weather.hourly.len(), 24);
    }

    #[test]
    fn test_hourly_forecasts_filtered_to_today() {
        let response: OpenMeteoResponseFull =
            serde_json::from_str(VALID_RESPONSE_FULL).expect("Failed to parse valid response full");

        let client = WeatherClient::new();
        let weather = client
            .parse_response_full(response)
            .expect("Failed to parse weather with full hourly");

        // All hours should be from 0-23
        for forecast in &weather.hourly {
            assert!(
                forecast.hour < 24,
                "Hour {} should be less than 24",
                forecast.hour
            );
        }

        // Verify hours are sequential from 0 to 23
        for (i, forecast) in weather.hourly.iter().enumerate() {
            assert_eq!(
                forecast.hour as usize, i,
                "Hour at index {} should be {}, got {}",
                i, i, forecast.hour
            );
        }
    }

    #[test]
    fn test_hourly_forecast_fields_populated() {
        let response: OpenMeteoResponseFull =
            serde_json::from_str(VALID_RESPONSE_FULL).expect("Failed to parse valid response full");

        let client = WeatherClient::new();
        let weather = client
            .parse_response_full(response)
            .expect("Failed to parse weather with full hourly");

        // Check first hour (midnight)
        let first_hour = &weather.hourly[0];
        assert_eq!(first_hour.hour, 0);
        assert!((first_hour.temperature - 15.2).abs() < 0.01);
        assert!((first_hour.feels_like - 14.0).abs() < 0.01);
        assert_eq!(first_hour.condition, WeatherCondition::Clear);
        assert!((first_hour.wind - 5.2).abs() < 0.01);
        assert_eq!(first_hour.wind_direction, "N");
        assert!((first_hour.uv - 0.0).abs() < 0.01);
        assert_eq!(first_hour.precipitation_chance, 0);

        // Check mid-day hour (index 14 = 2pm)
        let midday = &weather.hourly[14];
        assert_eq!(midday.hour, 14);
        assert!((midday.temperature - 24.8).abs() < 0.01);
        assert!((midday.feels_like - 23.8).abs() < 0.01);
        assert_eq!(midday.condition, WeatherCondition::PartlyCloudy);
        assert!((midday.wind - 15.8).abs() < 0.01);
        assert_eq!(midday.wind_direction, "S");
        assert!((midday.uv - 7.0).abs() < 0.01);
        assert_eq!(midday.precipitation_chance, 0);
    }

    #[test]
    fn test_parse_response_full_without_hourly() {
        // Response without hourly data should still parse and have empty hourly vec
        let response_no_hourly = r#"{
            "current": {
                "temperature_2m": 22.5,
                "relative_humidity_2m": 65,
                "apparent_temperature": 23.8,
                "weather_code": 2,
                "wind_speed_10m": 12.5,
                "wind_direction_10m": 270
            },
            "daily": {
                "time": ["2024-07-15"],
                "sunrise": ["2024-07-15T05:30"],
                "sunset": ["2024-07-15T21:15"],
                "uv_index_max": [7.5]
            }
        }"#;

        let response: OpenMeteoResponseFull = serde_json::from_str(response_no_hourly)
            .expect("Failed to parse response without hourly");

        let client = WeatherClient::new();
        let weather = client
            .parse_response_full(response)
            .expect("Failed to parse weather without hourly");

        // Should have empty hourly vec
        assert!(weather.hourly.is_empty());

        // Current weather should still be valid
        assert!((weather.temperature - 22.5).abs() < 0.01);
    }

    #[test]
    fn test_degrees_to_direction() {
        // Test cardinal directions
        assert_eq!(degrees_to_direction(0.0), "N");
        assert_eq!(degrees_to_direction(90.0), "E");
        assert_eq!(degrees_to_direction(180.0), "S");
        assert_eq!(degrees_to_direction(270.0), "W");

        // Test intercardinal directions
        assert_eq!(degrees_to_direction(45.0), "NE");
        assert_eq!(degrees_to_direction(135.0), "SE");
        assert_eq!(degrees_to_direction(225.0), "SW");
        assert_eq!(degrees_to_direction(315.0), "NW");

        // Test 16-point compass directions
        assert_eq!(degrees_to_direction(22.5), "NNE");
        assert_eq!(degrees_to_direction(67.5), "ENE");
        assert_eq!(degrees_to_direction(112.5), "ESE");
        assert_eq!(degrees_to_direction(157.5), "SSE");
        assert_eq!(degrees_to_direction(202.5), "SSW");
        assert_eq!(degrees_to_direction(247.5), "WSW");
        assert_eq!(degrees_to_direction(292.5), "WNW");
        assert_eq!(degrees_to_direction(337.5), "NNW");

        // Test values near boundaries
        assert_eq!(degrees_to_direction(11.0), "N"); // Just under NNE boundary
        assert_eq!(degrees_to_direction(12.0), "NNE"); // Just over N boundary

        // Test wrap-around
        assert_eq!(degrees_to_direction(360.0), "N");
        assert_eq!(degrees_to_direction(361.0), "N");
        assert_eq!(degrees_to_direction(-10.0), "N");
        assert_eq!(degrees_to_direction(-90.0), "W");
    }

    #[test]
    fn test_hourly_with_missing_optional_fields() {
        // Response with minimal hourly data (missing apparent_temperature, winddirection, precipitation_probability)
        let response_minimal_hourly = r#"{
            "current": {
                "temperature_2m": 22.5,
                "relative_humidity_2m": 65,
                "apparent_temperature": 23.8,
                "weather_code": 2,
                "wind_speed_10m": 12.5,
                "wind_direction_10m": 270
            },
            "daily": {
                "time": ["2024-07-15"],
                "sunrise": ["2024-07-15T05:30"],
                "sunset": ["2024-07-15T21:15"],
                "uv_index_max": [7.5]
            },
            "hourly": {
                "time": ["2024-07-15T12:00", "2024-07-15T13:00"],
                "temperature_2m": [22.0, 23.0],
                "weathercode": [2, 2],
                "windspeed_10m": [10.0, 11.0],
                "uv_index": [6.0, 6.5]
            }
        }"#;

        let response: OpenMeteoResponseFull = serde_json::from_str(response_minimal_hourly)
            .expect("Failed to parse minimal hourly response");

        let client = WeatherClient::new();
        let weather = client
            .parse_response_full(response)
            .expect("Failed to parse weather with minimal hourly");

        // Should have 2 hourly forecasts
        assert_eq!(weather.hourly.len(), 2);

        // Check defaults are applied
        let forecast = &weather.hourly[0];
        assert_eq!(forecast.hour, 12);
        assert!((forecast.temperature - 22.0).abs() < 0.01);
        // feels_like should default to temperature when apparent_temperature is missing
        assert!((forecast.feels_like - 22.0).abs() < 0.01);
        // wind_direction should default to "N" (0 degrees)
        assert_eq!(forecast.wind_direction, "N");
        // precipitation_chance should default to 0
        assert_eq!(forecast.precipitation_chance, 0);
    }

    #[test]
    fn test_cache_includes_hourly_data_serialization() {
        // Create a Weather object with hourly data and verify it serializes/deserializes correctly
        // This ensures cache compatibility
        let hourly = vec![
            HourlyForecast {
                hour: 10,
                temperature: 20.0,
                feels_like: 19.0,
                condition: WeatherCondition::Clear,
                wind: 10.0,
                wind_direction: "NW".to_string(),
                uv: 5.0,
                precipitation_chance: 10,
            },
            HourlyForecast {
                hour: 11,
                temperature: 21.0,
                feels_like: 20.0,
                condition: WeatherCondition::PartlyCloudy,
                wind: 12.0,
                wind_direction: "W".to_string(),
                uv: 6.0,
                precipitation_chance: 15,
            },
        ];

        let weather = Weather {
            temperature: 20.0,
            feels_like: 19.0,
            condition: WeatherCondition::Clear,
            humidity: 60,
            wind: 10.0,
            uv: 5.0,
            sunrise: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(20, 30, 0).unwrap(),
            fetched_at: Utc::now(),
            hourly,
        };

        // Serialize to JSON (simulating cache write)
        let json =
            serde_json::to_string(&weather).expect("Failed to serialize Weather with hourly");

        // Deserialize back (simulating cache read)
        let deserialized: Weather =
            serde_json::from_str(&json).expect("Failed to deserialize Weather with hourly");

        // Verify hourly data is preserved
        assert_eq!(deserialized.hourly.len(), 2);
        assert_eq!(deserialized.hourly[0].hour, 10);
        assert_eq!(deserialized.hourly[0].wind_direction, "NW");
        assert_eq!(deserialized.hourly[0].precipitation_chance, 10);
        assert_eq!(deserialized.hourly[1].hour, 11);
        assert_eq!(deserialized.hourly[1].wind_direction, "W");
        assert_eq!(deserialized.hourly[1].precipitation_chance, 15);
    }
}
