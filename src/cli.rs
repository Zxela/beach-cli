//! Command-line interface parsing for Vancouver Beach CLI
//!
//! This module handles parsing of CLI arguments using clap, including the
//! --plan flag for direct Plan Trip mode access with optional activity selection.

use clap::Parser;
use thiserror::Error;

use crate::activities::Activity;

/// Error types for CLI argument parsing
#[derive(Debug, Error)]
pub enum CliError {
    /// The specified activity name is not recognized
    #[error("Invalid activity: '{0}'. Valid activities: swim, sun, sail, sunset, peace, quiet")]
    InvalidActivity(String),
}

/// Vancouver Beach CLI - View beach conditions and plan beach trips
#[derive(Parser, Debug)]
#[command(name = "vanbeach")]
#[command(about = "Vancouver beach conditions and trip planning")]
#[command(version)]
pub struct Cli {
    /// Open directly in plan mode, optionally with a pre-selected activity
    ///
    /// Examples:
    ///   vanbeach --plan          # Open in Plan Trip mode
    ///   vanbeach --plan swim     # Open in Plan Trip mode with Swimming selected
    ///   vanbeach --plan sunset   # Open in Plan Trip mode with Sunset selected
    ///
    /// Valid activities: swim, sun, sail, sunset, peace, quiet
    #[arg(long, value_name = "ACTIVITY")]
    pub plan: Option<Option<String>>,
}

/// Configuration derived from CLI arguments for application startup
#[derive(Debug, Clone, Default)]
pub struct StartupConfig {
    /// Whether to start directly in PlanTrip state
    pub start_in_plan_trip: bool,
    /// Initial activity to select (if specified)
    pub initial_activity: Option<Activity>,
}

/// Parses an activity string argument into an Activity enum.
///
/// # Arguments
/// * `s` - The activity string from CLI
///
/// # Returns
/// * `Ok(Activity)` if the string matches a valid activity
/// * `Err(CliError::InvalidActivity)` if the string doesn't match
pub fn parse_activity_arg(s: &str) -> Result<Activity, CliError> {
    Activity::from_str(s).ok_or_else(|| CliError::InvalidActivity(s.to_string()))
}

impl StartupConfig {
    /// Creates a StartupConfig from parsed CLI arguments.
    ///
    /// # Arguments
    /// * `cli` - The parsed CLI struct
    ///
    /// # Returns
    /// * `Ok(StartupConfig)` with appropriate settings
    /// * `Err(CliError)` if an invalid activity was specified
    pub fn from_cli(cli: &Cli) -> Result<Self, CliError> {
        match &cli.plan {
            None => {
                // No --plan flag: normal startup
                Ok(StartupConfig::default())
            }
            Some(None) => {
                // --plan flag without activity: start in PlanTrip
                Ok(StartupConfig {
                    start_in_plan_trip: true,
                    initial_activity: None,
                })
            }
            Some(Some(activity_str)) => {
                // --plan <activity>: start in PlanTrip with activity
                let activity = parse_activity_arg(activity_str)?;
                Ok(StartupConfig {
                    start_in_plan_trip: true,
                    initial_activity: Some(activity),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_activity_arg_swimming_aliases() {
        assert_eq!(parse_activity_arg("swim").unwrap(), Activity::Swimming);
        assert_eq!(parse_activity_arg("swimming").unwrap(), Activity::Swimming);
    }

    #[test]
    fn test_parse_activity_arg_sunbathing_aliases() {
        assert_eq!(parse_activity_arg("sun").unwrap(), Activity::Sunbathing);
        assert_eq!(parse_activity_arg("sunbathing").unwrap(), Activity::Sunbathing);
        assert_eq!(parse_activity_arg("sunbathe").unwrap(), Activity::Sunbathing);
    }

    #[test]
    fn test_parse_activity_arg_sailing_aliases() {
        assert_eq!(parse_activity_arg("sail").unwrap(), Activity::Sailing);
        assert_eq!(parse_activity_arg("sailing").unwrap(), Activity::Sailing);
    }

    #[test]
    fn test_parse_activity_arg_sunset() {
        assert_eq!(parse_activity_arg("sunset").unwrap(), Activity::Sunset);
    }

    #[test]
    fn test_parse_activity_arg_peace_aliases() {
        assert_eq!(parse_activity_arg("peace").unwrap(), Activity::Peace);
        assert_eq!(parse_activity_arg("quiet").unwrap(), Activity::Peace);
    }

    #[test]
    fn test_parse_activity_arg_invalid() {
        let result = parse_activity_arg("invalid");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid activity"));
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn test_startup_config_default() {
        let config = StartupConfig::default();
        assert!(!config.start_in_plan_trip);
        assert!(config.initial_activity.is_none());
    }

    #[test]
    fn test_cli_parse_no_args() {
        let cli = Cli::parse_from(["vanbeach"]);
        assert!(cli.plan.is_none());
    }

    #[test]
    fn test_cli_parse_plan_only() {
        let cli = Cli::parse_from(["vanbeach", "--plan"]);
        assert!(cli.plan.is_some());
        assert!(cli.plan.as_ref().unwrap().is_none());
    }

    #[test]
    fn test_cli_parse_plan_with_activity() {
        let cli = Cli::parse_from(["vanbeach", "--plan", "swim"]);
        assert!(cli.plan.is_some());
        assert_eq!(cli.plan.as_ref().unwrap().as_deref(), Some("swim"));
    }

    #[test]
    fn test_startup_config_from_cli_no_plan() {
        let cli = Cli::parse_from(["vanbeach"]);
        let config = StartupConfig::from_cli(&cli).unwrap();
        assert!(!config.start_in_plan_trip);
        assert!(config.initial_activity.is_none());
    }

    #[test]
    fn test_startup_config_from_cli_plan_only() {
        let cli = Cli::parse_from(["vanbeach", "--plan"]);
        let config = StartupConfig::from_cli(&cli).unwrap();
        assert!(config.start_in_plan_trip);
        assert!(config.initial_activity.is_none());
    }

    #[test]
    fn test_startup_config_from_cli_plan_with_activity() {
        let cli = Cli::parse_from(["vanbeach", "--plan", "sunset"]);
        let config = StartupConfig::from_cli(&cli).unwrap();
        assert!(config.start_in_plan_trip);
        assert_eq!(config.initial_activity, Some(Activity::Sunset));
    }

    #[test]
    fn test_startup_config_from_cli_invalid_activity() {
        let cli = Cli::parse_from(["vanbeach", "--plan", "invalid"]);
        let result = StartupConfig::from_cli(&cli);
        assert!(result.is_err());
    }
}
