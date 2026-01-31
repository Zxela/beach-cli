//! Background data refresh system
//!
//! Provides automatic refresh of weather and water quality data in the background
//! using tokio channels to communicate updates to the main application.

use std::time::Duration;
use tokio::sync::mpsc;

use crate::data::{TideInfo, WaterQuality, Weather};

/// Messages sent from background refresh to main app
#[derive(Debug, Clone)]
pub enum RefreshMessage {
    /// Weather data updated for a beach
    WeatherUpdated {
        beach_id: String,
        weather: Weather,
    },
    /// Water quality data updated for a beach
    WaterQualityUpdated {
        beach_id: String,
        water_quality: WaterQuality,
    },
    /// Tide data updated (shared across all beaches)
    TidesUpdated(TideInfo),
    /// An error occurred during refresh
    RefreshError(String),
    /// Refresh started
    RefreshStarted,
    /// Refresh completed
    RefreshCompleted,
}

/// Configuration for refresh intervals
#[derive(Debug, Clone)]
pub struct RefreshConfig {
    /// Interval for weather data refresh
    pub weather_interval: Duration,
    /// Interval for water quality data refresh
    pub water_quality_interval: Duration,
    /// Whether auto-refresh is enabled
    pub enabled: bool,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            weather_interval: Duration::from_secs(300),      // 5 minutes
            water_quality_interval: Duration::from_secs(1800), // 30 minutes
            enabled: true,
        }
    }
}

/// Handle for controlling the background refresh system
pub struct RefreshHandle {
    /// Channel for receiving refresh messages
    pub receiver: mpsc::Receiver<RefreshMessage>,
    /// Flag to signal shutdown
    shutdown_tx: mpsc::Sender<()>,
}

impl RefreshHandle {
    /// Creates a new RefreshHandle and spawns background refresh tasks
    ///
    /// # Arguments
    /// * `config` - Configuration for refresh intervals
    ///
    /// # Returns
    /// A RefreshHandle that receives updates via the `receiver` channel
    pub fn spawn(config: RefreshConfig) -> Self {
        let (msg_tx, msg_rx) = mpsc::channel(32);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        if config.enabled {
            let weather_interval = config.weather_interval;
            let tx = msg_tx.clone();

            // Spawn weather refresh task
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(weather_interval);
                // Skip the first tick (immediate)
                interval.tick().await;

                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            // Signal that a refresh cycle is starting
                            let _ = tx.send(RefreshMessage::RefreshStarted).await;

                            // In a full implementation, we would:
                            // 1. Fetch weather for all beaches
                            // 2. Send WeatherUpdated messages for each
                            // 3. Send RefreshCompleted when done

                            // For now, just signal completion
                            let _ = tx.send(RefreshMessage::RefreshCompleted).await;
                        }
                        _ = shutdown_rx.recv() => {
                            break;
                        }
                    }
                }
            });
        }

        Self {
            receiver: msg_rx,
            shutdown_tx,
        }
    }

    /// Requests an immediate refresh
    #[allow(dead_code)]
    pub async fn request_refresh(&self) {
        // In a full implementation, this would trigger an immediate refresh
    }

    /// Shuts down the background refresh tasks
    #[allow(dead_code)]
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

/// Checks for pending refresh messages without blocking
///
/// # Arguments
/// * `handle` - The RefreshHandle to check
///
/// # Returns
/// * `Some(RefreshMessage)` if a message was available
/// * `None` if no messages are pending
pub fn try_recv(handle: &mut RefreshHandle) -> Option<RefreshMessage> {
    handle.receiver.try_recv().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_config_default() {
        let config = RefreshConfig::default();
        assert_eq!(config.weather_interval, Duration::from_secs(300));
        assert_eq!(config.water_quality_interval, Duration::from_secs(1800));
        assert!(config.enabled);
    }

    #[test]
    fn test_refresh_config_custom() {
        let config = RefreshConfig {
            weather_interval: Duration::from_secs(60),
            water_quality_interval: Duration::from_secs(600),
            enabled: false,
        };
        assert_eq!(config.weather_interval, Duration::from_secs(60));
        assert_eq!(config.water_quality_interval, Duration::from_secs(600));
        assert!(!config.enabled);
    }

    #[tokio::test]
    async fn test_refresh_handle_spawn_disabled() {
        let config = RefreshConfig {
            enabled: false,
            ..Default::default()
        };

        let mut handle = RefreshHandle::spawn(config);

        // With refresh disabled, there should be no messages
        assert!(try_recv(&mut handle).is_none());
    }
}
