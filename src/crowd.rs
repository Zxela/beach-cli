//! Crowd estimation heuristics for Vancouver beaches
//!
//! This module provides functions to estimate beach crowd levels based on
//! time of day, day of week, and season.

#![allow(dead_code)]

use chrono::Weekday;

/// Estimates the crowd level at a beach based on temporal factors.
///
/// Returns a value from 0.0 (empty) to 1.0 (packed) representing the
/// estimated crowd level.
///
/// # Arguments
///
/// * `month` - Month of the year (1-12)
/// * `weekday` - Day of the week
/// * `hour` - Hour of the day (0-23)
///
/// # Returns
///
/// A float value clamped to the range 0.0-1.0 representing crowd level.
///
/// # Example
///
/// ```
/// use chrono::Weekday;
/// use vanbeach::crowd::estimate_crowd;
///
/// // July Saturday at 2pm - expect high crowd
/// let crowd = estimate_crowd(7, Weekday::Sat, 14);
/// assert!(crowd > 0.8);
/// ```
pub fn estimate_crowd(month: u32, weekday: Weekday, hour: u32) -> f32 {
    let season_factor = calculate_season_factor(month);
    let day_factor = calculate_day_factor(weekday);
    let hour_factor = calculate_hour_factor(hour);

    let crowd = season_factor * day_factor * hour_factor;

    // Clamp result to 0.0-1.0 range
    crowd.clamp(0.0, 1.0)
}

/// Calculates the seasonal factor for crowd estimation.
///
/// Summer months (June-August) have the highest factor, with shoulder
/// seasons and winter having progressively lower values.
fn calculate_season_factor(month: u32) -> f32 {
    match month {
        6..=8 => 1.0,  // Summer - busiest
        5 | 9 => 0.6,  // Shoulder season
        4 | 10 => 0.3, // Spring/fall
        _ => 0.1,      // Winter - minimal
    }
}

/// Calculates the day-of-week factor for crowd estimation.
///
/// Weekends are busiest, with Friday slightly less busy, and weekdays
/// having the lowest factor.
fn calculate_day_factor(weekday: Weekday) -> f32 {
    match weekday {
        Weekday::Sat | Weekday::Sun => 1.0,
        Weekday::Fri => 0.7,
        _ => 0.4, // Mon-Thu
    }
}

/// Calculates the hour-of-day factor for crowd estimation.
///
/// Peak afternoon hours (12-4pm) have the highest factor, with morning
/// and evening hours having progressively lower values.
fn calculate_hour_factor(hour: u32) -> f32 {
    match hour {
        12..=16 => 1.0, // Peak afternoon
        10..=11 | 17..=18 => 0.7,
        8..=9 | 19..=20 => 0.4,
        6..=7 | 21 => 0.2,
        _ => 0.1, // Night/early morning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_july_saturday_2pm_returns_high_crowd() {
        // July (month 7), Saturday, 2pm (hour 14)
        let crowd = estimate_crowd(7, Weekday::Sat, 14);
        assert!(
            crowd > 0.8,
            "July Saturday 2pm should return > 0.8, got {}",
            crowd
        );
    }

    #[test]
    fn test_january_tuesday_7am_returns_low_crowd() {
        // January (month 1), Tuesday, 7am (hour 7)
        let crowd = estimate_crowd(1, Weekday::Tue, 7);
        assert!(
            crowd < 0.2,
            "January Tuesday 7am should return < 0.2, got {}",
            crowd
        );
    }

    #[test]
    fn test_output_always_in_valid_range() {
        // Test various combinations to ensure output is always 0.0-1.0
        for month in 1..=12 {
            for hour in 0..=23 {
                for weekday in [
                    Weekday::Mon,
                    Weekday::Tue,
                    Weekday::Wed,
                    Weekday::Thu,
                    Weekday::Fri,
                    Weekday::Sat,
                    Weekday::Sun,
                ] {
                    let crowd = estimate_crowd(month, weekday, hour);
                    assert!(
                        (0.0..=1.0).contains(&crowd),
                        "Crowd should be in 0.0-1.0 range for month={}, weekday={:?}, hour={}, got {}",
                        month,
                        weekday,
                        hour,
                        crowd
                    );
                }
            }
        }
    }

    #[test]
    fn test_boundary_months() {
        // Test month 1 (January - winter)
        let january_crowd = estimate_crowd(1, Weekday::Sat, 14);
        assert!(
            january_crowd <= 0.2,
            "January should have low seasonal factor, got {}",
            january_crowd
        );

        // Test month 12 (December - winter)
        let december_crowd = estimate_crowd(12, Weekday::Sat, 14);
        assert!(
            december_crowd <= 0.2,
            "December should have low seasonal factor, got {}",
            december_crowd
        );
    }

    #[test]
    fn test_boundary_hours() {
        // Test hour 0 (midnight)
        let midnight_crowd = estimate_crowd(7, Weekday::Sat, 0);
        assert!(
            midnight_crowd <= 0.2,
            "Midnight should have very low hour factor, got {}",
            midnight_crowd
        );

        // Test hour 23 (11pm)
        let late_night_crowd = estimate_crowd(7, Weekday::Sat, 23);
        assert!(
            late_night_crowd <= 0.2,
            "11pm should have very low hour factor, got {}",
            late_night_crowd
        );
    }

    #[test]
    fn test_all_weekdays_produce_valid_output() {
        let weekdays = [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ];

        for weekday in weekdays {
            let crowd = estimate_crowd(7, weekday, 14); // July, 2pm
            assert!(
                (0.0..=1.0).contains(&crowd),
                "Weekday {:?} should produce valid output, got {}",
                weekday,
                crowd
            );
            assert!(
                crowd > 0.0,
                "Summer afternoon should have non-zero crowd for {:?}, got {}",
                weekday,
                crowd
            );
        }
    }

    #[test]
    fn test_weekend_has_higher_crowd_than_weekday() {
        let saturday_crowd = estimate_crowd(7, Weekday::Sat, 14);
        let tuesday_crowd = estimate_crowd(7, Weekday::Tue, 14);

        assert!(
            saturday_crowd > tuesday_crowd,
            "Saturday ({}) should have higher crowd than Tuesday ({})",
            saturday_crowd,
            tuesday_crowd
        );
    }

    #[test]
    fn test_summer_has_higher_crowd_than_winter() {
        let july_crowd = estimate_crowd(7, Weekday::Sat, 14);
        let january_crowd = estimate_crowd(1, Weekday::Sat, 14);

        assert!(
            july_crowd > january_crowd,
            "July ({}) should have higher crowd than January ({})",
            july_crowd,
            january_crowd
        );
    }

    #[test]
    fn test_peak_hour_has_higher_crowd_than_early_morning() {
        let afternoon_crowd = estimate_crowd(7, Weekday::Sat, 14);
        let early_morning_crowd = estimate_crowd(7, Weekday::Sat, 6);

        assert!(
            afternoon_crowd > early_morning_crowd,
            "2pm ({}) should have higher crowd than 6am ({})",
            afternoon_crowd,
            early_morning_crowd
        );
    }

    #[test]
    fn test_season_factor_values() {
        // Summer peak
        assert_eq!(calculate_season_factor(7), 1.0);
        // Shoulder season
        assert_eq!(calculate_season_factor(5), 0.6);
        assert_eq!(calculate_season_factor(9), 0.6);
        // Spring/fall
        assert_eq!(calculate_season_factor(4), 0.3);
        assert_eq!(calculate_season_factor(10), 0.3);
        // Winter
        assert_eq!(calculate_season_factor(1), 0.1);
        assert_eq!(calculate_season_factor(12), 0.1);
    }

    #[test]
    fn test_day_factor_values() {
        assert_eq!(calculate_day_factor(Weekday::Sat), 1.0);
        assert_eq!(calculate_day_factor(Weekday::Sun), 1.0);
        assert_eq!(calculate_day_factor(Weekday::Fri), 0.7);
        assert_eq!(calculate_day_factor(Weekday::Mon), 0.4);
        assert_eq!(calculate_day_factor(Weekday::Tue), 0.4);
        assert_eq!(calculate_day_factor(Weekday::Wed), 0.4);
        assert_eq!(calculate_day_factor(Weekday::Thu), 0.4);
    }

    #[test]
    fn test_hour_factor_values() {
        // Peak afternoon
        assert_eq!(calculate_hour_factor(14), 1.0);
        // Near peak
        assert_eq!(calculate_hour_factor(10), 0.7);
        assert_eq!(calculate_hour_factor(17), 0.7);
        // Moderate
        assert_eq!(calculate_hour_factor(8), 0.4);
        assert_eq!(calculate_hour_factor(19), 0.4);
        // Low
        assert_eq!(calculate_hour_factor(6), 0.2);
        assert_eq!(calculate_hour_factor(21), 0.2);
        // Very low
        assert_eq!(calculate_hour_factor(0), 0.1);
        assert_eq!(calculate_hour_factor(23), 0.1);
    }
}
