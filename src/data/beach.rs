//! Static beach data for Vancouver beaches
//!
//! This module contains the static list of all Vancouver beaches with their
//! geographic coordinates and water quality monitoring station IDs.

use super::Beach;

/// Static array of all Vancouver beaches
///
/// Contains 12 beaches from the Vancouver area with accurate coordinates
/// and water quality monitoring station IDs matching Vancouver Open Data naming.
pub static BEACHES: [Beach; 12] = [
    Beach {
        id: "kitsilano",
        name: "Kitsilano Beach",
        latitude: 49.2743,
        longitude: -123.1544,
        water_quality_id: Some("kitsilano-beach"),
    },
    Beach {
        id: "english-bay",
        name: "English Bay Beach",
        latitude: 49.2863,
        longitude: -123.1432,
        water_quality_id: Some("english-bay"),
    },
    Beach {
        id: "jericho",
        name: "Jericho Beach",
        latitude: 49.2726,
        longitude: -123.1967,
        water_quality_id: Some("jericho-beach"),
    },
    Beach {
        id: "spanish-banks-east",
        name: "Spanish Banks East",
        latitude: 49.2756,
        longitude: -123.2089,
        water_quality_id: Some("spanish-banks-east"),
    },
    Beach {
        id: "spanish-banks-west",
        name: "Spanish Banks West",
        latitude: 49.2769,
        longitude: -123.2244,
        water_quality_id: Some("spanish-banks-west"),
    },
    Beach {
        id: "locarno",
        name: "Locarno Beach",
        latitude: 49.2768,
        longitude: -123.2167,
        water_quality_id: Some("locarno-beach"),
    },
    Beach {
        id: "wreck",
        name: "Wreck Beach",
        latitude: 49.2621,
        longitude: -123.2617,
        water_quality_id: Some("wreck-beach"),
    },
    Beach {
        id: "second",
        name: "Second Beach",
        latitude: 49.2912,
        longitude: -123.1513,
        water_quality_id: Some("second-beach"),
    },
    Beach {
        id: "third",
        name: "Third Beach",
        latitude: 49.2989,
        longitude: -123.1588,
        water_quality_id: Some("third-beach"),
    },
    Beach {
        id: "sunset",
        name: "Sunset Beach",
        latitude: 49.2799,
        longitude: -123.1339,
        water_quality_id: Some("sunset-beach"),
    },
    Beach {
        id: "trout-lake",
        name: "Trout Lake Beach",
        latitude: 49.2555,
        longitude: -123.0644,
        water_quality_id: Some("trout-lake"),
    },
    Beach {
        id: "new-brighton",
        name: "New Brighton Beach",
        latitude: 49.2930,
        longitude: -123.0365,
        water_quality_id: Some("new-brighton"),
    },
];

/// Get a beach by its ID
///
/// # Arguments
///
/// * `id` - The unique identifier for the beach (e.g., "kitsilano", "english-bay")
///
/// # Returns
///
/// Returns `Some(&Beach)` if found, `None` otherwise
///
/// # Example
///
/// ```
/// use weather_cli::data::beach::get_beach_by_id;
///
/// if let Some(beach) = get_beach_by_id("kitsilano") {
///     println!("Found: {}", beach.name);
/// }
/// ```
pub fn get_beach_by_id(id: &str) -> Option<&'static Beach> {
    BEACHES.iter().find(|beach| beach.id == id)
}

/// Get all available beaches
///
/// # Returns
///
/// Returns a static slice containing all 12 Vancouver beaches
///
/// # Example
///
/// ```
/// use weather_cli::data::beach::all_beaches;
///
/// for beach in all_beaches() {
///     println!("{}: ({}, {})", beach.name, beach.latitude, beach.longitude);
/// }
/// ```
pub fn all_beaches() -> &'static [Beach] {
    &BEACHES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beaches_array_has_12_entries() {
        assert_eq!(BEACHES.len(), 12);
    }

    #[test]
    fn test_all_beaches_returns_12_entries() {
        assert_eq!(all_beaches().len(), 12);
    }

    #[test]
    fn test_each_beach_has_valid_vancouver_coordinates() {
        // Vancouver area latitude range: 49.2 to 49.4
        // Vancouver area longitude range: -123.0 to -123.3
        for beach in all_beaches() {
            assert!(
                beach.latitude >= 49.2 && beach.latitude <= 49.4,
                "Beach {} has invalid latitude: {}",
                beach.name,
                beach.latitude
            );
            assert!(
                beach.longitude >= -123.3 && beach.longitude <= -123.0,
                "Beach {} has invalid longitude: {}",
                beach.name,
                beach.longitude
            );
        }
    }

    #[test]
    fn test_each_beach_has_non_zero_coordinates() {
        for beach in all_beaches() {
            assert!(
                beach.latitude != 0.0,
                "Beach {} has zero latitude",
                beach.name
            );
            assert!(
                beach.longitude != 0.0,
                "Beach {} has zero longitude",
                beach.name
            );
        }
    }

    #[test]
    fn test_get_beach_by_id_returns_correct_beach() {
        let beach = get_beach_by_id("kitsilano");
        assert!(beach.is_some());
        let beach = beach.unwrap();
        assert_eq!(beach.id, "kitsilano");
        assert_eq!(beach.name, "Kitsilano Beach");
        assert!((beach.latitude - 49.2743).abs() < 0.0001);
        assert!((beach.longitude - (-123.1544)).abs() < 0.0001);
    }

    #[test]
    fn test_get_beach_by_id_english_bay() {
        let beach = get_beach_by_id("english-bay");
        assert!(beach.is_some());
        let beach = beach.unwrap();
        assert_eq!(beach.name, "English Bay Beach");
    }

    #[test]
    fn test_get_beach_by_id_returns_none_for_invalid_id() {
        assert!(get_beach_by_id("invalid-beach").is_none());
        assert!(get_beach_by_id("").is_none());
        assert!(get_beach_by_id("KITSILANO").is_none()); // Case sensitive
    }

    #[test]
    fn test_all_beaches_have_unique_ids() {
        let mut ids: Vec<&str> = all_beaches().iter().map(|b| b.id).collect();
        ids.sort();
        let original_len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Beach IDs are not unique");
    }

    #[test]
    fn test_all_beaches_have_water_quality_id() {
        for beach in all_beaches() {
            assert!(
                beach.water_quality_id.is_some(),
                "Beach {} is missing water_quality_id",
                beach.name
            );
        }
    }

    #[test]
    fn test_specific_beach_coordinates() {
        // Verify specific coordinates from the PRD
        let test_cases = [
            ("kitsilano", 49.2743, -123.1544),
            ("english-bay", 49.2863, -123.1432),
            ("jericho", 49.2726, -123.1967),
            ("spanish-banks-east", 49.2756, -123.2089),
            ("spanish-banks-west", 49.2769, -123.2244),
            ("locarno", 49.2768, -123.2167),
            ("wreck", 49.2621, -123.2617),
            ("second", 49.2912, -123.1513),
            ("third", 49.2989, -123.1588),
            ("sunset", 49.2799, -123.1339),
            ("trout-lake", 49.2555, -123.0644),
            ("new-brighton", 49.2930, -123.0365),
        ];

        for (id, expected_lat, expected_lon) in test_cases {
            let beach = get_beach_by_id(id).expect(&format!("Beach {} not found", id));
            assert!(
                (beach.latitude - expected_lat).abs() < 0.0001,
                "Beach {} latitude mismatch: expected {}, got {}",
                id,
                expected_lat,
                beach.latitude
            );
            assert!(
                (beach.longitude - expected_lon).abs() < 0.0001,
                "Beach {} longitude mismatch: expected {}, got {}",
                id,
                expected_lon,
                beach.longitude
            );
        }
    }
}
