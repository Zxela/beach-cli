//! Activity types and preferences for the beach recommendation engine.
//!
//! This module defines the core activity types and preference enums used
//! throughout the scoring engine and UI.

/// Beach activities that users can select for recommendations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Activity {
    /// Swimming in the ocean
    Swimming,
    /// Sunbathing on the beach
    Sunbathing,
    /// Sailing or other water sports
    Sailing,
    /// Watching the sunset
    Sunset,
    /// Seeking peace and quiet
    Peace,
}

#[allow(dead_code)]
impl Activity {
    /// Returns a slice containing all activity variants.
    pub fn all() -> &'static [Activity] {
        &[
            Activity::Swimming,
            Activity::Sunbathing,
            Activity::Sailing,
            Activity::Sunset,
            Activity::Peace,
        ]
    }

    /// Returns a human-readable display label for the activity.
    pub fn label(&self) -> &'static str {
        match self {
            Activity::Swimming => "Swimming",
            Activity::Sunbathing => "Sunbathing",
            Activity::Sailing => "Sailing",
            Activity::Sunset => "Sunset",
            Activity::Peace => "Peace & Quiet",
        }
    }

    /// Parses user input into an Activity.
    ///
    /// Matching is case-insensitive and supports aliases:
    /// - "swim" | "swimming" -> Swimming
    /// - "sun" | "sunbathing" | "sunbathe" -> Sunbathing
    /// - "sail" | "sailing" -> Sailing
    /// - "sunset" -> Sunset
    /// - "peace" | "quiet" -> Peace
    ///
    /// Returns `None` if the input doesn't match any activity.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Activity> {
        match s.to_lowercase().trim() {
            "swim" | "swimming" => Some(Activity::Swimming),
            "sun" | "sunbathing" | "sunbathe" => Some(Activity::Sunbathing),
            "sail" | "sailing" => Some(Activity::Sailing),
            "sunset" => Some(Activity::Sunset),
            "peace" | "quiet" => Some(Activity::Peace),
            _ => None,
        }
    }
}

/// Tide level preferences for activities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TidePreference {
    /// Prefer high tide
    High,
    /// Prefer mid tide
    Mid,
    /// Prefer low tide
    Low,
    /// No tide preference
    Any,
}

/// UV exposure preferences for activities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum UvPreference {
    /// Prefer high UV (good for tanning)
    High,
    /// Prefer moderate UV
    Moderate,
    /// Prefer low UV (sun-sensitive)
    Low,
    /// No UV preference
    Any,
}

// ============================================================================
// SCORING ENGINE - Activity profiles and scoring functions
// ============================================================================

use crate::data::WaterStatus;

/// Weights and preferences for scoring a time slot for a specific activity.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ActivityProfile {
    /// The activity this profile is for
    pub activity: Activity,
    /// Weight for temperature scoring (0.0-1.0)
    pub temp_weight: f32,
    /// Ideal temperature range in Celsius (min, max)
    pub temp_ideal_range: (f32, f32),
    /// Weight for water quality scoring (0.0 = ignore, higher = critical)
    pub water_quality_weight: f32,
    /// Weight for wind scoring
    pub wind_weight: f32,
    /// Ideal wind range in km/h (min, max)
    pub wind_ideal_range: (f32, f32),
    /// Weight for UV scoring
    pub uv_weight: f32,
    /// UV preference for this activity
    pub uv_preference: UvPreference,
    /// Weight for tide scoring
    pub tide_weight: f32,
    /// Tide preference for this activity
    pub tide_preference: TidePreference,
    /// Weight for crowd scoring (higher = more crowd-averse)
    pub crowd_weight: f32,
    /// Optional custom time-of-day scoring function
    pub time_of_day_scorer: Option<fn(u8) -> f32>,
}

/// Individual factor scores (0.0-1.0) for a time slot.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ScoreFactors {
    /// Temperature score (0.0-1.0)
    pub temperature: f32,
    /// Water quality score (0.0-1.0)
    pub water_quality: f32,
    /// Wind score (0.0-1.0)
    pub wind: f32,
    /// UV score (0.0-1.0)
    pub uv: f32,
    /// Tide score (0.0-1.0)
    pub tide: f32,
    /// Crowd score (0.0-1.0)
    pub crowd: f32,
    /// Time of day score (0.0-1.0)
    pub time_of_day: f32,
}

/// Complete score for a time slot including all factors.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TimeSlotScore {
    /// Hour of the day (0-23)
    pub hour: u8,
    /// Beach identifier
    pub beach_id: String,
    /// Activity being scored
    pub activity: Activity,
    /// Final score (0-100)
    pub score: u8,
    /// Individual factor scores
    pub factors: ScoreFactors,
    /// Whether this time slot is blocked due to unsafe conditions
    pub blocked: bool,
    /// Reason for blocking, if blocked
    pub block_reason: Option<String>,
}

#[allow(dead_code)]
impl ActivityProfile {
    /// Score temperature based on ideal range.
    /// Returns 1.0 when in ideal range, scales down to 0.0 when 5+ degrees outside.
    pub fn score_temperature(&self, temp: f32) -> f32 {
        let (min, max) = self.temp_ideal_range;
        if temp < min - 5.0 || temp > max + 5.0 {
            0.0 // Way outside range
        } else if temp >= min && temp <= max {
            1.0 // Perfect
        } else if temp < min {
            // Below ideal, scale from 0 to 1
            ((temp - (min - 5.0)) / 5.0).clamp(0.0, 1.0)
        } else {
            // Above ideal, scale from 1 to 0
            ((max + 5.0 - temp) / 5.0).clamp(0.0, 1.0)
        }
    }

    /// Score wind based on ideal range.
    /// Returns 1.0 when in ideal range, scales down outside.
    pub fn score_wind(&self, wind: f32) -> f32 {
        let (min, max) = self.wind_ideal_range;
        if wind >= min && wind <= max {
            1.0
        } else if wind < min {
            if min == 0.0 {
                1.0 // Can't have negative wind, so at 0 min wind is perfect
            } else {
                (wind / min).clamp(0.0, 1.0)
            }
        } else {
            // Above max, scale down
            ((max * 1.5 - wind) / (max * 0.5)).clamp(0.0, 1.0)
        }
    }

    /// Score water quality status.
    /// Returns 0.0 for Closed, 1.0 for Safe.
    pub fn score_water_quality(&self, status: WaterStatus) -> f32 {
        match status {
            WaterStatus::Safe => 1.0,
            WaterStatus::Advisory => 0.3,
            WaterStatus::Closed => 0.0,
            WaterStatus::Unknown => 0.5,
        }
    }

    /// Score UV index based on activity preference.
    pub fn score_uv(&self, uv: f32) -> f32 {
        match self.uv_preference {
            UvPreference::High => (uv / 8.0).clamp(0.0, 1.0),
            UvPreference::Moderate => 1.0 - ((uv - 5.0).abs() / 5.0).clamp(0.0, 1.0),
            UvPreference::Low => (1.0 - uv / 6.0).clamp(0.0, 1.0),
            UvPreference::Any => 1.0,
        }
    }

    /// Score tide based on preference.
    /// Height is normalized to max_height.
    pub fn score_tide(&self, height: f32, max_height: f32) -> f32 {
        let normalized = height / max_height; // 0.0 = low, 1.0 = high
        match self.tide_preference {
            TidePreference::High => normalized,
            TidePreference::Mid => 1.0 - (normalized - 0.5).abs() * 2.0,
            TidePreference::Low => 1.0 - normalized,
            TidePreference::Any => 1.0,
        }
    }

    /// Score crowd level (inverted - high crowd = low score).
    pub fn score_crowd(&self, crowd_level: f32) -> f32 {
        1.0 - crowd_level.clamp(0.0, 1.0)
    }

    /// Compute the overall score for a time slot.
    /// Returns a TimeSlotScore with the final 0-100 score and all factor scores.
    #[allow(clippy::too_many_arguments)]
    pub fn score_time_slot(
        &self,
        hour: u8,
        beach_id: &str,
        temp: f32,
        wind: f32,
        uv: f32,
        water_status: WaterStatus,
        tide_height: f32,
        max_tide: f32,
        crowd_level: f32,
    ) -> TimeSlotScore {
        let factors = ScoreFactors {
            temperature: self.score_temperature(temp),
            water_quality: self.score_water_quality(water_status),
            wind: self.score_wind(wind),
            uv: self.score_uv(uv),
            tide: self.score_tide(tide_height, max_tide),
            crowd: self.score_crowd(crowd_level),
            time_of_day: self.time_of_day_scorer.map(|f| f(hour)).unwrap_or(1.0),
        };

        let weighted_sum = factors.temperature * self.temp_weight
            + factors.water_quality * self.water_quality_weight
            + factors.wind * self.wind_weight
            + factors.uv * self.uv_weight
            + factors.tide * self.tide_weight
            + factors.crowd * self.crowd_weight
            + factors.time_of_day * 0.1; // Slight time preference

        let total_weight = self.temp_weight
            + self.water_quality_weight
            + self.wind_weight
            + self.uv_weight
            + self.tide_weight
            + self.crowd_weight
            + 0.1;

        let score = ((weighted_sum / total_weight) * 100.0).clamp(0.0, 100.0) as u8;

        TimeSlotScore {
            hour,
            beach_id: beach_id.to_string(),
            activity: self.activity,
            score,
            factors,
            blocked: false,
            block_reason: None,
        }
    }

    /// Check weather sanity gates for the activity.
    ///
    /// Returns a blocking reason if the weather conditions make this activity
    /// unsafe or nonsensical, or None if the activity can proceed.
    ///
    /// # Arguments
    ///
    /// * `temp` - Temperature in Celsius
    /// * `wind` - Wind speed in km/h
    /// * `weather_code` - Optional WMO weather code
    ///
    /// # Weather Code Reference
    ///
    /// - 51-67: Drizzle and rain
    /// - 71-77: Snow
    /// - 80-82: Rain showers
    /// - 95-99: Thunderstorm
    pub fn check_sanity_gates(&self, temp: f32, wind: f32, weather_code: Option<u8>) -> Option<String> {
        let code = weather_code.unwrap_or(0);

        // Universal blocks: snow or thunderstorm blocks all activities
        if (71..=77).contains(&code) {
            return Some("Snow conditions are unsafe for beach activities".to_string());
        }
        if (95..=99).contains(&code) {
            return Some("Thunderstorm conditions are dangerous".to_string());
        }

        // Activity-specific blocks
        match self.activity {
            Activity::Swimming => {
                if temp < 15.0 {
                    return Some(format!("Temperature {:.1}°C is too cold for swimming (minimum 15°C)", temp));
                }
                // Rain codes: 51-67 (drizzle/rain), 80-82 (rain showers)
                if (51..=67).contains(&code) || (80..=82).contains(&code) {
                    return Some("Rain makes swimming unsafe and unpleasant".to_string());
                }
            }
            Activity::Sunbathing => {
                if temp < 18.0 {
                    return Some(format!("Temperature {:.1}°C is too cold for sunbathing (minimum 18°C)", temp));
                }
                // Overcast (code 3) or rain
                if code == 3 {
                    return Some("Overcast conditions are not suitable for sunbathing".to_string());
                }
                if (51..=67).contains(&code) || (80..=82).contains(&code) {
                    return Some("Rain makes sunbathing impossible".to_string());
                }
            }
            Activity::Sailing => {
                if wind > 40.0 {
                    return Some(format!("Wind speed {:.1} km/h is dangerously high for sailing (maximum 40 km/h)", wind));
                }
            }
            Activity::Sunset | Activity::Peace => {
                // No additional blocks beyond universal ones
            }
        }

        None
    }

    /// Score a time slot with weather code for sanity gate checking.
    ///
    /// This method first checks sanity gates and returns a blocked score if
    /// conditions are unsafe. Otherwise, it delegates to `score_time_slot`.
    #[allow(clippy::too_many_arguments)]
    pub fn score_time_slot_with_weather_code(
        &self,
        hour: u8,
        beach_id: &str,
        temp: f32,
        wind: f32,
        uv: f32,
        water_status: WaterStatus,
        tide_height: f32,
        max_tide: f32,
        crowd_level: f32,
        weather_code: Option<u8>,
    ) -> TimeSlotScore {
        // Check sanity gates first
        if let Some(reason) = self.check_sanity_gates(temp, wind, weather_code) {
            // Return a blocked score
            return TimeSlotScore {
                hour,
                beach_id: beach_id.to_string(),
                activity: self.activity,
                score: 0,
                factors: ScoreFactors {
                    temperature: 0.0,
                    water_quality: 0.0,
                    wind: 0.0,
                    uv: 0.0,
                    tide: 0.0,
                    crowd: 0.0,
                    time_of_day: 0.0,
                },
                blocked: true,
                block_reason: Some(reason),
            };
        }

        // Not blocked, use regular scoring
        self.score_time_slot(
            hour,
            beach_id,
            temp,
            wind,
            uv,
            water_status,
            tide_height,
            max_tide,
            crowd_level,
        )
    }
}

/// Returns the preset ActivityProfile for a given activity.
#[allow(dead_code)]
pub fn get_profile(activity: Activity) -> ActivityProfile {
    match activity {
        Activity::Swimming => ActivityProfile {
            activity: Activity::Swimming,
            temp_weight: 0.3,
            temp_ideal_range: (20.0, 28.0),
            water_quality_weight: 0.4, // Critical
            wind_weight: 0.1,
            wind_ideal_range: (0.0, 15.0),
            uv_weight: 0.05,
            uv_preference: UvPreference::Moderate,
            tide_weight: 0.15,
            tide_preference: TidePreference::Mid,
            crowd_weight: 0.1,
            time_of_day_scorer: None,
        },
        Activity::Sunbathing => ActivityProfile {
            activity: Activity::Sunbathing,
            temp_weight: 0.35,
            temp_ideal_range: (24.0, 32.0),
            water_quality_weight: 0.0,
            wind_weight: 0.25,
            wind_ideal_range: (0.0, 10.0),
            uv_weight: 0.25,
            uv_preference: UvPreference::High,
            tide_weight: 0.0,
            tide_preference: TidePreference::Any,
            crowd_weight: 0.15,
            time_of_day_scorer: None,
        },
        Activity::Sailing => ActivityProfile {
            activity: Activity::Sailing,
            temp_weight: 0.1,
            temp_ideal_range: (15.0, 30.0),
            water_quality_weight: 0.0,
            wind_weight: 0.6,
            wind_ideal_range: (15.0, 25.0),
            uv_weight: 0.0,
            uv_preference: UvPreference::Any,
            tide_weight: 0.2,
            tide_preference: TidePreference::High,
            crowd_weight: 0.1,
            time_of_day_scorer: None,
        },
        Activity::Sunset => ActivityProfile {
            activity: Activity::Sunset,
            temp_weight: 0.15,
            temp_ideal_range: (15.0, 28.0),
            water_quality_weight: 0.0,
            wind_weight: 0.1,
            wind_ideal_range: (0.0, 20.0),
            uv_weight: 0.0,
            uv_preference: UvPreference::Any,
            tide_weight: 0.0,
            tide_preference: TidePreference::Any,
            crowd_weight: 0.15,
            time_of_day_scorer: Some(sunset_time_scorer),
        },
        Activity::Peace => ActivityProfile {
            activity: Activity::Peace,
            temp_weight: 0.1,
            temp_ideal_range: (12.0, 25.0),
            water_quality_weight: 0.0,
            wind_weight: 0.1,
            wind_ideal_range: (0.0, 15.0),
            uv_weight: 0.1,
            uv_preference: UvPreference::Low,
            tide_weight: 0.0,
            tide_preference: TidePreference::Any,
            crowd_weight: 0.7, // Highly crowd-averse
            time_of_day_scorer: Some(peace_time_scorer),
        },
    }
}

/// Custom time-of-day scorer for sunset activities.
/// Peaks at evening hours (18-20).
#[allow(dead_code)]
pub fn sunset_time_scorer(hour: u8) -> f32 {
    match hour {
        18..=20 => 1.0,
        17 | 21 => 0.7,
        16 | 22 => 0.3,
        _ => 0.1,
    }
}

/// Dynamic time-of-day scorer for sunset activities based on actual sunset time.
///
/// Scores hours based on their distance from the provided sunset hour,
/// allowing accurate recommendations regardless of season or location.
///
/// # Arguments
///
/// * `hour` - The hour of the day to score (0-23)
/// * `sunset_hour` - The hour when sunset occurs (0-23)
///
/// # Returns
///
/// A score from 0.1 to 1.0:
/// - 1.0 at sunset_hour (peak viewing time)
/// - 0.9 at ±1 hour (golden hour before, twilight after)
/// - 0.5 at ±2 hours (good but not optimal)
/// - 0.2 at ±3 hours (marginal)
/// - 0.1 beyond ±3 hours (too far from sunset)
///
/// # Examples
///
/// ```
/// use vanbeach::activities::sunset_time_scorer_dynamic;
///
/// // Summer sunset at 21:00
/// assert_eq!(sunset_time_scorer_dynamic(21, 21), 1.0);  // Peak at sunset
/// assert_eq!(sunset_time_scorer_dynamic(20, 21), 0.9);  // Golden hour
/// assert_eq!(sunset_time_scorer_dynamic(22, 21), 0.9);  // Twilight
///
/// // Winter sunset at 17:00
/// assert_eq!(sunset_time_scorer_dynamic(17, 17), 1.0);  // Peak at sunset
/// assert_eq!(sunset_time_scorer_dynamic(15, 17), 0.5);  // 2 hours before
/// ```
#[allow(dead_code)]
pub fn sunset_time_scorer_dynamic(hour: u8, sunset_hour: u8) -> f32 {
    let diff = (hour as i16 - sunset_hour as i16).abs();
    match diff {
        0 => 1.0,  // Sunset hour
        1 => 0.9,  // 1 hour before/after
        2 => 0.5,  // 2 hours before/after
        3 => 0.2,  // 3 hours before/after
        _ => 0.1,  // Too far from sunset
    }
}

/// Custom time-of-day scorer for peace & quiet activities.
/// Peaks at early morning (6-7).
#[allow(dead_code)]
pub fn peace_time_scorer(hour: u8) -> f32 {
    match hour {
        6..=7 => 1.0,
        8 => 0.8,
        5 | 9 => 0.5,
        _ => 0.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_all_returns_five_activities() {
        let activities = Activity::all();
        assert_eq!(activities.len(), 5);
        assert!(activities.contains(&Activity::Swimming));
        assert!(activities.contains(&Activity::Sunbathing));
        assert!(activities.contains(&Activity::Sailing));
        assert!(activities.contains(&Activity::Sunset));
        assert!(activities.contains(&Activity::Peace));
    }

    #[test]
    fn test_activity_label_swimming() {
        assert_eq!(Activity::Swimming.label(), "Swimming");
    }

    #[test]
    fn test_activity_label_sunbathing() {
        assert_eq!(Activity::Sunbathing.label(), "Sunbathing");
    }

    #[test]
    fn test_activity_label_sailing() {
        assert_eq!(Activity::Sailing.label(), "Sailing");
    }

    #[test]
    fn test_activity_label_sunset() {
        assert_eq!(Activity::Sunset.label(), "Sunset");
    }

    #[test]
    fn test_activity_label_peace() {
        assert_eq!(Activity::Peace.label(), "Peace & Quiet");
    }

    #[test]
    fn test_from_str_swimming_aliases() {
        assert_eq!(Activity::from_str("swim"), Some(Activity::Swimming));
        assert_eq!(Activity::from_str("swimming"), Some(Activity::Swimming));
        assert_eq!(Activity::from_str("SWIM"), Some(Activity::Swimming));
        assert_eq!(Activity::from_str("Swimming"), Some(Activity::Swimming));
    }

    #[test]
    fn test_from_str_sunbathing_aliases() {
        assert_eq!(Activity::from_str("sun"), Some(Activity::Sunbathing));
        assert_eq!(Activity::from_str("sunbathing"), Some(Activity::Sunbathing));
        assert_eq!(Activity::from_str("sunbathe"), Some(Activity::Sunbathing));
        assert_eq!(Activity::from_str("SUN"), Some(Activity::Sunbathing));
    }

    #[test]
    fn test_from_str_sailing_aliases() {
        assert_eq!(Activity::from_str("sail"), Some(Activity::Sailing));
        assert_eq!(Activity::from_str("sailing"), Some(Activity::Sailing));
        assert_eq!(Activity::from_str("SAILING"), Some(Activity::Sailing));
    }

    #[test]
    fn test_from_str_sunset() {
        assert_eq!(Activity::from_str("sunset"), Some(Activity::Sunset));
        assert_eq!(Activity::from_str("SUNSET"), Some(Activity::Sunset));
        assert_eq!(Activity::from_str("Sunset"), Some(Activity::Sunset));
    }

    #[test]
    fn test_from_str_peace_aliases() {
        assert_eq!(Activity::from_str("peace"), Some(Activity::Peace));
        assert_eq!(Activity::from_str("quiet"), Some(Activity::Peace));
        assert_eq!(Activity::from_str("PEACE"), Some(Activity::Peace));
        assert_eq!(Activity::from_str("QUIET"), Some(Activity::Peace));
    }

    #[test]
    fn test_from_str_invalid_input() {
        assert_eq!(Activity::from_str("invalid"), None);
        assert_eq!(Activity::from_str(""), None);
        assert_eq!(Activity::from_str("running"), None);
        assert_eq!(Activity::from_str("beach"), None);
    }

    #[test]
    fn test_from_str_with_whitespace() {
        assert_eq!(Activity::from_str("  swim  "), Some(Activity::Swimming));
        assert_eq!(Activity::from_str("\tsunset\n"), Some(Activity::Sunset));
    }

    #[test]
    fn test_tide_preference_derives() {
        // Test that TidePreference implements Debug, Clone, Copy, PartialEq, Eq
        let pref = TidePreference::High;
        let cloned = pref;
        assert_eq!(pref, cloned);
        assert_eq!(format!("{:?}", pref), "High");
    }

    #[test]
    fn test_uv_preference_derives() {
        // Test that UvPreference implements Debug, Clone, Copy, PartialEq, Eq
        let pref = UvPreference::Moderate;
        let cloned = pref;
        assert_eq!(pref, cloned);
        assert_eq!(format!("{:?}", pref), "Moderate");
    }

    // ========================================================================
    // Scoring Engine Tests
    // ========================================================================

    #[test]
    fn test_score_temperature_returns_1_when_in_ideal_range() {
        let profile = get_profile(Activity::Swimming);
        // Swimming ideal range is 20-28°C
        assert_eq!(profile.score_temperature(20.0), 1.0);
        assert_eq!(profile.score_temperature(24.0), 1.0);
        assert_eq!(profile.score_temperature(28.0), 1.0);
    }

    #[test]
    fn test_score_temperature_returns_0_when_far_outside_range() {
        let profile = get_profile(Activity::Swimming);
        // Swimming ideal range is 20-28°C, so 5+ degrees outside = 0
        assert_eq!(profile.score_temperature(14.9), 0.0); // Below min-5
        assert_eq!(profile.score_temperature(10.0), 0.0);
        assert_eq!(profile.score_temperature(33.1), 0.0); // Above max+5
        assert_eq!(profile.score_temperature(40.0), 0.0);
    }

    #[test]
    fn test_score_temperature_scales_between_ideal_and_boundary() {
        let profile = get_profile(Activity::Swimming);
        // 17.5°C is halfway between 15 (min-5) and 20 (min)
        let score = profile.score_temperature(17.5);
        assert!(score > 0.4 && score < 0.6, "Expected ~0.5, got {}", score);

        // 30.5°C is halfway between 28 (max) and 33 (max+5)
        let score2 = profile.score_temperature(30.5);
        assert!(
            score2 > 0.4 && score2 < 0.6,
            "Expected ~0.5, got {}",
            score2
        );
    }

    #[test]
    fn test_score_water_quality_returns_0_for_closed_1_for_safe() {
        let profile = get_profile(Activity::Swimming);
        assert_eq!(profile.score_water_quality(WaterStatus::Safe), 1.0);
        assert_eq!(profile.score_water_quality(WaterStatus::Advisory), 0.3);
        assert_eq!(profile.score_water_quality(WaterStatus::Closed), 0.0);
        assert_eq!(profile.score_water_quality(WaterStatus::Unknown), 0.5);
    }

    #[test]
    fn test_score_wind_returns_1_when_in_ideal_range() {
        let profile = get_profile(Activity::Sailing);
        // Sailing ideal wind is 15-25 km/h
        assert_eq!(profile.score_wind(15.0), 1.0);
        assert_eq!(profile.score_wind(20.0), 1.0);
        assert_eq!(profile.score_wind(25.0), 1.0);
    }

    #[test]
    fn test_score_wind_below_min_for_sailing() {
        let profile = get_profile(Activity::Sailing);
        // Sailing ideal wind is 15-25 km/h, so 0 wind = 0/15 = 0
        assert_eq!(profile.score_wind(0.0), 0.0);
        // 7.5 km/h = 7.5/15 = 0.5
        let score = profile.score_wind(7.5);
        assert!((score - 0.5).abs() < 0.01, "Expected 0.5, got {}", score);
    }

    #[test]
    fn test_score_wind_handles_zero_min_range() {
        let profile = get_profile(Activity::Swimming);
        // Swimming ideal wind is 0-15 km/h
        // When min is 0, any wind at or below max is perfect
        assert_eq!(profile.score_wind(0.0), 1.0);
        assert_eq!(profile.score_wind(5.0), 1.0);
        assert_eq!(profile.score_wind(15.0), 1.0);
    }

    #[test]
    fn test_swimming_profile_penalizes_unsafe_water_heavily() {
        let profile = get_profile(Activity::Swimming);
        // Water quality weight is 0.4 (the highest) for swimming
        assert_eq!(profile.water_quality_weight, 0.4);

        // Score with closed water should be significantly lower
        let safe_score =
            profile.score_time_slot(12, "test", 24.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3);
        let closed_score = profile.score_time_slot(
            12,
            "test",
            24.0,
            5.0,
            5.0,
            WaterStatus::Closed,
            2.4,
            4.8,
            0.3,
        );

        // With water_quality_weight=0.4, closed water (0.0) vs safe (1.0)
        // should make a significant difference
        assert!(
            safe_score.score > closed_score.score + 30,
            "Safe={} should be much higher than Closed={}",
            safe_score.score,
            closed_score.score
        );
    }

    #[test]
    fn test_sailing_profile_rewards_high_wind() {
        let profile = get_profile(Activity::Sailing);
        // Wind weight is 0.6 for sailing
        assert_eq!(profile.wind_weight, 0.6);

        // Good wind (20 km/h) vs no wind (0 km/h)
        let good_wind_score = profile.score_time_slot(
            12,
            "test",
            20.0,
            20.0,
            3.0,
            WaterStatus::Safe,
            4.0,
            4.8,
            0.3,
        );
        let no_wind_score =
            profile.score_time_slot(12, "test", 20.0, 0.0, 3.0, WaterStatus::Safe, 4.0, 4.8, 0.3);

        assert!(
            good_wind_score.score > no_wind_score.score + 40,
            "Good wind={} should be much higher than no wind={}",
            good_wind_score.score,
            no_wind_score.score
        );
    }

    #[test]
    fn test_peace_profile_heavily_weights_crowd_aversion() {
        let profile = get_profile(Activity::Peace);
        // Crowd weight is 0.7 for peace
        assert_eq!(profile.crowd_weight, 0.7);

        // Empty beach vs packed beach
        let quiet_score =
            profile.score_time_slot(7, "test", 18.0, 5.0, 2.0, WaterStatus::Safe, 2.4, 4.8, 0.0);
        let crowded_score =
            profile.score_time_slot(7, "test", 18.0, 5.0, 2.0, WaterStatus::Safe, 2.4, 4.8, 1.0);

        assert!(
            quiet_score.score > crowded_score.score + 50,
            "Quiet={} should be much higher than crowded={}",
            quiet_score.score,
            crowded_score.score
        );
    }

    #[test]
    fn test_sunset_time_scorer_peaks_at_evening_hours() {
        assert_eq!(sunset_time_scorer(18), 1.0);
        assert_eq!(sunset_time_scorer(19), 1.0);
        assert_eq!(sunset_time_scorer(20), 1.0);
        assert_eq!(sunset_time_scorer(17), 0.7);
        assert_eq!(sunset_time_scorer(21), 0.7);
        assert_eq!(sunset_time_scorer(16), 0.3);
        assert_eq!(sunset_time_scorer(12), 0.1);
        assert_eq!(sunset_time_scorer(8), 0.1);
    }

    #[test]
    fn test_peace_time_scorer_peaks_at_early_morning() {
        assert_eq!(peace_time_scorer(6), 1.0);
        assert_eq!(peace_time_scorer(7), 1.0);
        assert_eq!(peace_time_scorer(8), 0.8);
        assert_eq!(peace_time_scorer(5), 0.5);
        assert_eq!(peace_time_scorer(9), 0.5);
        assert_eq!(peace_time_scorer(12), 0.2);
        assert_eq!(peace_time_scorer(18), 0.2);
    }

    #[test]
    fn test_full_score_time_slot_produces_score_in_0_100_range() {
        // Test with various profiles and conditions
        for activity in Activity::all() {
            let profile = get_profile(*activity);

            // Test with "perfect" conditions
            let perfect = profile.score_time_slot(
                12,
                "test",
                24.0,
                10.0,
                5.0,
                WaterStatus::Safe,
                2.4,
                4.8,
                0.0,
            );
            assert!(
                perfect.score <= 100,
                "Score {} for {:?} should be <= 100",
                perfect.score,
                activity
            );

            // Test with "bad" conditions
            let bad = profile.score_time_slot(
                3,
                "test",
                5.0,
                50.0,
                11.0,
                WaterStatus::Closed,
                0.0,
                4.8,
                1.0,
            );
            assert!(
                bad.score <= 100,
                "Score {} for {:?} should be <= 100",
                bad.score,
                activity
            );
        }
    }

    #[test]
    fn test_get_profile_returns_correct_activity() {
        assert_eq!(get_profile(Activity::Swimming).activity, Activity::Swimming);
        assert_eq!(
            get_profile(Activity::Sunbathing).activity,
            Activity::Sunbathing
        );
        assert_eq!(get_profile(Activity::Sailing).activity, Activity::Sailing);
        assert_eq!(get_profile(Activity::Sunset).activity, Activity::Sunset);
        assert_eq!(get_profile(Activity::Peace).activity, Activity::Peace);
    }

    #[test]
    fn test_score_uv_for_high_preference() {
        let profile = get_profile(Activity::Sunbathing);
        // High UV preference: higher UV = higher score
        assert!(profile.score_uv(8.0) > profile.score_uv(4.0));
        assert_eq!(profile.score_uv(8.0), 1.0);
        assert_eq!(profile.score_uv(0.0), 0.0);
    }

    #[test]
    fn test_score_uv_for_low_preference() {
        let profile = get_profile(Activity::Peace);
        // Low UV preference: lower UV = higher score
        assert!(profile.score_uv(2.0) > profile.score_uv(6.0));
        assert_eq!(profile.score_uv(0.0), 1.0);
    }

    #[test]
    fn test_score_tide_for_high_preference() {
        let profile = get_profile(Activity::Sailing);
        // High tide preference
        assert_eq!(profile.score_tide(4.8, 4.8), 1.0); // Full high tide
        assert_eq!(profile.score_tide(0.0, 4.8), 0.0); // Low tide
    }

    #[test]
    fn test_score_tide_for_mid_preference() {
        let profile = get_profile(Activity::Swimming);
        // Mid tide preference: mid tide = 1.0, extremes = 0.0
        assert_eq!(profile.score_tide(2.4, 4.8), 1.0); // Perfect mid
        assert_eq!(profile.score_tide(0.0, 4.8), 0.0); // Too low
        assert_eq!(profile.score_tide(4.8, 4.8), 0.0); // Too high
    }

    #[test]
    fn test_score_crowd_inverts_crowd_level() {
        let profile = get_profile(Activity::Swimming);
        assert_eq!(profile.score_crowd(0.0), 1.0); // Empty = great
        assert_eq!(profile.score_crowd(1.0), 0.0); // Packed = bad
        assert_eq!(profile.score_crowd(0.5), 0.5); // Half = half
    }

    #[test]
    fn test_time_slot_score_has_correct_metadata() {
        let profile = get_profile(Activity::Swimming);
        let score = profile.score_time_slot(
            14,
            "kitsilano",
            24.0,
            10.0,
            5.0,
            WaterStatus::Safe,
            2.4,
            4.8,
            0.2,
        );

        assert_eq!(score.hour, 14);
        assert_eq!(score.beach_id, "kitsilano");
        assert_eq!(score.activity, Activity::Swimming);
    }

    #[test]
    fn test_score_factors_are_all_in_range() {
        let profile = get_profile(Activity::Swimming);
        let result = profile.score_time_slot(
            12,
            "test",
            24.0,
            10.0,
            5.0,
            WaterStatus::Safe,
            2.4,
            4.8,
            0.3,
        );

        assert!(result.factors.temperature >= 0.0 && result.factors.temperature <= 1.0);
        assert!(result.factors.water_quality >= 0.0 && result.factors.water_quality <= 1.0);
        assert!(result.factors.wind >= 0.0 && result.factors.wind <= 1.0);
        assert!(result.factors.uv >= 0.0 && result.factors.uv <= 1.0);
        assert!(result.factors.tide >= 0.0 && result.factors.tide <= 1.0);
        assert!(result.factors.crowd >= 0.0 && result.factors.crowd <= 1.0);
        assert!(result.factors.time_of_day >= 0.0 && result.factors.time_of_day <= 1.0);
    }

    // ========================================================================
    // Dynamic Sunset Time Scorer Tests
    // ========================================================================

    #[test]
    fn test_sunset_time_scorer_dynamic_peaks_at_sunset_hour() {
        // Verify score is 1.0 when hour == sunset_hour
        // Test with sunset_hour = 17, 20, 21
        assert_eq!(sunset_time_scorer_dynamic(17, 17), 1.0);
        assert_eq!(sunset_time_scorer_dynamic(20, 20), 1.0);
        assert_eq!(sunset_time_scorer_dynamic(21, 21), 1.0);
    }

    #[test]
    fn test_sunset_time_scorer_dynamic_scores_decrease_with_distance() {
        // Use sunset_hour = 19 as reference
        let sunset_hour = 19;

        // Verify hour ±1 from sunset scores 0.9
        assert_eq!(sunset_time_scorer_dynamic(18, sunset_hour), 0.9);
        assert_eq!(sunset_time_scorer_dynamic(20, sunset_hour), 0.9);

        // Verify hour ±2 from sunset scores 0.5
        assert_eq!(sunset_time_scorer_dynamic(17, sunset_hour), 0.5);
        assert_eq!(sunset_time_scorer_dynamic(21, sunset_hour), 0.5);

        // Verify hour ±3 from sunset scores 0.2
        assert_eq!(sunset_time_scorer_dynamic(16, sunset_hour), 0.2);
        assert_eq!(sunset_time_scorer_dynamic(22, sunset_hour), 0.2);

        // Verify hour ±4+ from sunset scores 0.1
        assert_eq!(sunset_time_scorer_dynamic(15, sunset_hour), 0.1);
        assert_eq!(sunset_time_scorer_dynamic(23, sunset_hour), 0.1);
        assert_eq!(sunset_time_scorer_dynamic(10, sunset_hour), 0.1);
        assert_eq!(sunset_time_scorer_dynamic(0, sunset_hour), 0.1);
    }

    #[test]
    fn test_sunset_time_scorer_dynamic_with_early_sunset() {
        // Test with sunset_hour = 17 (winter)
        let sunset_hour = 17;

        // Verify hour 17 scores 1.0
        assert_eq!(sunset_time_scorer_dynamic(17, sunset_hour), 1.0);

        // Verify hour 18 scores 0.9 (1 hour after)
        assert_eq!(sunset_time_scorer_dynamic(18, sunset_hour), 0.9);

        // Verify hour 16 scores 0.9 (1 hour before)
        assert_eq!(sunset_time_scorer_dynamic(16, sunset_hour), 0.9);

        // Verify hour 19 scores 0.5 (2 hours after)
        assert_eq!(sunset_time_scorer_dynamic(19, sunset_hour), 0.5);

        // Verify hour 15 scores 0.5 (2 hours before)
        assert_eq!(sunset_time_scorer_dynamic(15, sunset_hour), 0.5);
    }

    #[test]
    fn test_sunset_time_scorer_dynamic_with_late_sunset() {
        // Test with sunset_hour = 21 (summer)
        let sunset_hour = 21;

        // Verify hour 21 scores 1.0
        assert_eq!(sunset_time_scorer_dynamic(21, sunset_hour), 1.0);

        // Verify hour 20 scores 0.9 (1 hour before)
        assert_eq!(sunset_time_scorer_dynamic(20, sunset_hour), 0.9);

        // Verify hour 22 scores 0.9 (1 hour after)
        assert_eq!(sunset_time_scorer_dynamic(22, sunset_hour), 0.9);

        // Verify hour 19 scores 0.5 (2 hours before)
        assert_eq!(sunset_time_scorer_dynamic(19, sunset_hour), 0.5);

        // Verify hour 23 scores 0.5 (2 hours after)
        assert_eq!(sunset_time_scorer_dynamic(23, sunset_hour), 0.5);
    }

    // ========================================================================
    // Weather Sanity Gates Tests
    // ========================================================================

    #[test]
    fn test_swimming_blocked_when_raining() {
        let profile = get_profile(Activity::Swimming);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 24.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(61),
        );
        assert_eq!(score.score, 0);
        assert!(score.blocked);
    }

    #[test]
    fn test_swimming_blocked_when_cold() {
        let profile = get_profile(Activity::Swimming);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 12.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(0),
        );
        assert_eq!(score.score, 0);
        assert!(score.blocked);
    }

    #[test]
    fn test_sunbathing_blocked_when_overcast() {
        let profile = get_profile(Activity::Sunbathing);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 25.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(3),
        );
        assert_eq!(score.score, 0);
        assert!(score.blocked);
    }

    #[test]
    fn test_sailing_blocked_when_dangerous_wind() {
        let profile = get_profile(Activity::Sailing);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 20.0, 45.0, 3.0, WaterStatus::Safe, 4.0, 4.8, 0.3, Some(0),
        );
        assert_eq!(score.score, 0);
        assert!(score.blocked);
    }

    #[test]
    fn test_all_activities_blocked_during_thunderstorm() {
        for activity in Activity::all() {
            let profile = get_profile(*activity);
            let score = profile.score_time_slot_with_weather_code(
                12, "test", 25.0, 10.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(95),
            );
            assert_eq!(score.score, 0, "Activity {:?} should be blocked during thunderstorm", activity);
            assert!(score.blocked, "Activity {:?} should be blocked during thunderstorm", activity);
            assert!(score.block_reason.as_ref().unwrap().contains("Thunderstorm"),
                "Activity {:?} block reason should mention thunderstorm", activity);
        }
    }

    #[test]
    fn test_all_activities_blocked_during_snow() {
        for activity in Activity::all() {
            let profile = get_profile(*activity);
            let score = profile.score_time_slot_with_weather_code(
                12, "test", 0.0, 10.0, 2.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(73),
            );
            assert_eq!(score.score, 0, "Activity {:?} should be blocked during snow", activity);
            assert!(score.blocked, "Activity {:?} should be blocked during snow", activity);
            assert!(score.block_reason.as_ref().unwrap().contains("Snow"),
                "Activity {:?} block reason should mention snow", activity);
        }
    }

    #[test]
    fn test_swimming_not_blocked_when_conditions_good() {
        let profile = get_profile(Activity::Swimming);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 24.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(0),
        );
        assert!(!score.blocked);
        assert!(score.block_reason.is_none());
        assert!(score.score > 0);
    }

    #[test]
    fn test_sunbathing_blocked_when_raining() {
        let profile = get_profile(Activity::Sunbathing);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 25.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(61),
        );
        assert_eq!(score.score, 0);
        assert!(score.blocked);
        assert!(score.block_reason.as_ref().unwrap().contains("Rain"));
    }

    #[test]
    fn test_sunbathing_blocked_when_cold() {
        let profile = get_profile(Activity::Sunbathing);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 15.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(0),
        );
        assert_eq!(score.score, 0);
        assert!(score.blocked);
        assert!(score.block_reason.as_ref().unwrap().contains("cold"));
    }

    #[test]
    fn test_sailing_not_blocked_with_moderate_wind() {
        let profile = get_profile(Activity::Sailing);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 20.0, 20.0, 3.0, WaterStatus::Safe, 4.0, 4.8, 0.3, Some(0),
        );
        assert!(!score.blocked);
        assert!(score.block_reason.is_none());
        assert!(score.score > 0);
    }

    #[test]
    fn test_sanity_gate_block_reason_contains_temperature() {
        let profile = get_profile(Activity::Swimming);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 10.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(0),
        );
        assert!(score.block_reason.as_ref().unwrap().contains("10.0"));
    }

    #[test]
    fn test_sanity_gate_block_reason_contains_wind_speed() {
        let profile = get_profile(Activity::Sailing);
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 20.0, 50.0, 3.0, WaterStatus::Safe, 4.0, 4.8, 0.3, Some(0),
        );
        assert!(score.block_reason.as_ref().unwrap().contains("50.0"));
    }

    #[test]
    fn test_peace_activity_not_blocked_by_cold_or_wind() {
        let profile = get_profile(Activity::Peace);
        // Peace activity should work in cold weather and high wind (unless thunderstorm/snow)
        let score = profile.score_time_slot_with_weather_code(
            12, "test", 5.0, 35.0, 2.0, WaterStatus::Safe, 2.4, 4.8, 0.1, Some(0),
        );
        assert!(!score.blocked);
        assert!(score.score > 0);
    }

    #[test]
    fn test_sunset_activity_not_blocked_by_cold_or_wind() {
        let profile = get_profile(Activity::Sunset);
        // Sunset activity should work in cold weather and high wind (unless thunderstorm/snow)
        let score = profile.score_time_slot_with_weather_code(
            19, "test", 5.0, 35.0, 2.0, WaterStatus::Safe, 2.4, 4.8, 0.1, Some(0),
        );
        assert!(!score.blocked);
        assert!(score.score > 0);
    }

    #[test]
    fn test_score_time_slot_sets_blocked_false() {
        // Verify that the original score_time_slot method sets blocked=false
        let profile = get_profile(Activity::Swimming);
        let score = profile.score_time_slot(
            12, "test", 24.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3,
        );
        assert!(!score.blocked);
        assert!(score.block_reason.is_none());
    }

    #[test]
    fn test_swimming_blocked_with_rain_shower_codes() {
        let profile = get_profile(Activity::Swimming);
        // Test rain shower codes 80-82
        for code in 80..=82 {
            let score = profile.score_time_slot_with_weather_code(
                12, "test", 24.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(code),
            );
            assert!(score.blocked, "Swimming should be blocked with weather code {}", code);
        }
    }

    #[test]
    fn test_swimming_blocked_with_drizzle_codes() {
        let profile = get_profile(Activity::Swimming);
        // Test drizzle/rain codes 51-67
        for code in [51, 53, 55, 61, 63, 65, 67] {
            let score = profile.score_time_slot_with_weather_code(
                12, "test", 24.0, 5.0, 5.0, WaterStatus::Safe, 2.4, 4.8, 0.3, Some(code),
            );
            assert!(score.blocked, "Swimming should be blocked with weather code {}", code);
        }
    }
}
