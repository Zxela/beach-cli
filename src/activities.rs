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
}
