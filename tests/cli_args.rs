//! Integration tests for CLI argument handling
//!
//! Tests the --plan flag and activity parsing from command line.

use std::process::Command;

/// Helper to run the CLI with given args and capture output
fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vanbeach"))
        .args(args)
        .output()
        .expect("Failed to execute vanbeach")
}

#[test]
fn test_help_flag_exits_successfully() {
    let output = run_cli(&["--help"]);
    assert!(
        output.status.success(),
        "Expected --help to exit successfully"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("vanbeach"), "Help should mention vanbeach");
    assert!(stdout.contains("plan"), "Help should mention --plan flag");
}

#[test]
fn test_invalid_activity_prints_error_and_exits() {
    let output = run_cli(&["--plan", "invalid_activity"]);
    assert!(
        !output.status.success(),
        "Expected invalid activity to fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("Invalid") || stderr.contains("unknown"),
        "Should print error message about invalid activity: {}",
        stderr
    );
}

#[test]
fn test_plan_with_swim_is_valid() {
    // This test just verifies the argument is accepted (doesn't error immediately)
    // The actual state transition is tested in unit tests
    let output = run_cli(&["--plan", "swim", "--help"]);
    // With --help, it should succeed regardless of other flags
    // This is a workaround since we can't easily test TUI apps
    assert!(output.status.success());
}

#[test]
fn test_plan_with_sunset_is_valid() {
    let output = run_cli(&["--plan", "sunset", "--help"]);
    assert!(output.status.success());
}

#[cfg(test)]
mod unit_tests {
    //! Unit tests for CLI parsing that don't require running the binary

    use clap::Parser;
    use vanbeach::activities::Activity;
    use vanbeach::cli::{parse_activity_arg, Cli, StartupConfig};

    #[test]
    fn test_cli_no_args_returns_none_plan() {
        let cli = Cli::parse_from(["vanbeach"]);
        assert!(cli.plan.is_none());
    }

    #[test]
    fn test_cli_plan_flag_without_value() {
        let cli = Cli::parse_from(["vanbeach", "--plan"]);
        assert!(cli.plan.is_some());
        assert!(cli.plan.as_ref().unwrap().is_none());
    }

    #[test]
    fn test_cli_plan_flag_with_swim() {
        let cli = Cli::parse_from(["vanbeach", "--plan", "swim"]);
        assert!(cli.plan.is_some());
        assert_eq!(cli.plan.as_ref().unwrap().as_deref(), Some("swim"));
    }

    #[test]
    fn test_cli_plan_flag_with_sunset() {
        let cli = Cli::parse_from(["vanbeach", "--plan", "sunset"]);
        assert!(cli.plan.is_some());
        assert_eq!(cli.plan.as_ref().unwrap().as_deref(), Some("sunset"));
    }

    #[test]
    fn test_parse_activity_arg_swim_returns_swimming() {
        let result = parse_activity_arg("swim");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Activity::Swimming);
    }

    #[test]
    fn test_parse_activity_arg_sunset_returns_sunset() {
        let result = parse_activity_arg("sunset");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Activity::Sunset);
    }

    #[test]
    fn test_parse_activity_arg_invalid_returns_error() {
        let result = parse_activity_arg("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_startup_config_default_is_normal() {
        let config = StartupConfig::default();
        assert!(!config.start_in_plan_trip);
        assert!(config.initial_activity.is_none());
    }

    #[test]
    fn test_startup_config_from_cli_no_plan() {
        let cli = Cli::parse_from(["vanbeach"]);
        let config = StartupConfig::from_cli(&cli);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(!config.start_in_plan_trip);
        assert!(config.initial_activity.is_none());
    }

    #[test]
    fn test_startup_config_from_cli_plan_only() {
        let cli = Cli::parse_from(["vanbeach", "--plan"]);
        let config = StartupConfig::from_cli(&cli);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.start_in_plan_trip);
        assert!(config.initial_activity.is_none());
    }

    #[test]
    fn test_startup_config_from_cli_plan_with_activity() {
        let cli = Cli::parse_from(["vanbeach", "--plan", "swim"]);
        let config = StartupConfig::from_cli(&cli);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.start_in_plan_trip);
        assert_eq!(config.initial_activity, Some(Activity::Swimming));
    }

    #[test]
    fn test_startup_config_from_cli_plan_with_invalid_activity() {
        let cli = Cli::parse_from(["vanbeach", "--plan", "invalid"]);
        let config = StartupConfig::from_cli(&cli);
        assert!(config.is_err());
    }
}
