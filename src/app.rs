//! Application state management for Vancouver Beach CLI
//!
//! This module contains the main application state, handling keyboard input,
//! data loading, and state transitions between different views.

use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;

use crate::cache::CacheManager;
use crate::data::{
    all_beaches, get_beach_by_id, Beach, BeachConditions, TidesClient, WaterQuality,
    WaterQualityClient, WaterQualityError, Weather, WeatherClient, WeatherError,
};

/// Application state enum representing the current view
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    /// Initial loading state while fetching data
    Loading,
    /// List view showing all beaches
    BeachList,
    /// Detail view for a specific beach
    BeachDetail(String),
}

/// Main application struct managing state and data
pub struct App {
    /// Current application state/view
    pub state: AppState,
    /// Index of currently selected beach in list view
    pub selected_index: usize,
    /// Cached beach conditions data keyed by beach ID
    pub beach_conditions: HashMap<String, BeachConditions>,
    /// Flag indicating the application should quit
    pub should_quit: bool,
    /// Weather API client
    weather_client: WeatherClient,
    /// Tides API client
    tides_client: TidesClient,
    /// Water quality API client
    water_quality_client: WaterQualityClient,
}

impl App {
    /// Creates a new App instance with default state
    pub fn new() -> Self {
        let cache = CacheManager::new();
        Self {
            state: AppState::Loading,
            selected_index: 0,
            beach_conditions: HashMap::new(),
            should_quit: false,
            weather_client: WeatherClient::new(),
            tides_client: TidesClient::new(cache.clone()),
            water_quality_client: cache
                .map(WaterQualityClient::with_cache)
                .unwrap_or_default(),
        }
    }

    /// Creates a new App instance with custom clients (for testing)
    #[cfg(test)]
    pub fn with_clients(
        weather_client: WeatherClient,
        tides_client: TidesClient,
        water_quality_client: WaterQualityClient,
    ) -> Self {
        Self {
            state: AppState::Loading,
            selected_index: 0,
            beach_conditions: HashMap::new(),
            should_quit: false,
            weather_client,
            tides_client,
            water_quality_client,
        }
    }

    /// Returns the total number of beaches
    pub fn beach_count(&self) -> usize {
        all_beaches().len()
    }

    /// Returns the currently selected beach, if any
    pub fn selected_beach(&self) -> Option<&'static Beach> {
        all_beaches().get(self.selected_index)
    }

    /// Loads all beach data concurrently
    ///
    /// Fetches weather for all beaches, tides (shared), and water quality for each beach.
    /// Transitions to BeachList state when complete.
    pub async fn load_all_data(&mut self) {
        let beaches = all_beaches();

        // Fetch tides once (same for all beaches)
        let tides_result = self.tides_client.fetch_tides().await.ok();

        // Fetch weather and water quality for all beaches concurrently
        let mut weather_futures = Vec::new();
        let mut water_quality_futures = Vec::new();

        for beach in beaches {
            weather_futures.push(self.weather_client.fetch_weather(beach.latitude, beach.longitude));
            if let Some(wq_id) = beach.water_quality_id {
                water_quality_futures.push(self.water_quality_client.fetch_water_quality(wq_id));
            }
        }

        // Wait for all weather requests concurrently
        let weather_results: Vec<Result<Weather, WeatherError>> =
            futures::future::join_all(weather_futures).await;

        // Wait for all water quality requests concurrently
        let water_quality_results: Vec<Result<WaterQuality, WaterQualityError>> =
            futures::future::join_all(water_quality_futures).await;

        // Build beach conditions for each beach
        let mut wq_index = 0;
        for (i, beach) in beaches.iter().enumerate() {
            let weather = weather_results.get(i).and_then(|r| r.as_ref().ok().cloned());

            let water_quality = if beach.water_quality_id.is_some() {
                let result = water_quality_results
                    .get(wq_index)
                    .and_then(|r| r.as_ref().ok().cloned());
                wq_index += 1;
                result
            } else {
                None
            };

            let conditions = BeachConditions {
                beach: *beach,
                weather,
                tides: tides_result.clone(),
                water_quality,
            };

            self.beach_conditions.insert(beach.id.to_string(), conditions);
        }

        // Transition to beach list state
        self.state = AppState::BeachList;
    }

    /// Refreshes data for a single beach
    ///
    /// # Arguments
    /// * `beach_id` - The ID of the beach to refresh
    #[allow(dead_code)]
    pub async fn refresh_beach(&mut self, beach_id: &str) {
        let Some(beach) = get_beach_by_id(beach_id) else {
            return;
        };

        // Fetch weather
        let weather = self
            .weather_client
            .fetch_weather(beach.latitude, beach.longitude)
            .await
            .ok();

        // Fetch tides
        let tides = self.tides_client.fetch_tides().await.ok();

        // Fetch water quality
        let water_quality = if let Some(wq_id) = beach.water_quality_id {
            self.water_quality_client.fetch_water_quality(wq_id).await.ok()
        } else {
            None
        };

        let conditions = BeachConditions {
            beach: *beach,
            weather,
            tides,
            water_quality,
        };

        self.beach_conditions.insert(beach_id.to_string(), conditions);
    }

    /// Handles keyboard input and updates state accordingly
    ///
    /// # Arguments
    /// * `key_event` - The keyboard event to handle
    ///
    /// # Key Bindings
    /// - `q` or `Esc` (in BeachList): Quit the application
    /// - `Up`/`k`: Move selection up in list
    /// - `Down`/`j`: Move selection down in list
    /// - `Enter`: Select current beach (go to detail view)
    /// - `Esc` (in BeachDetail): Go back to list view
    pub fn handle_key(&mut self, key_event: KeyEvent) {
        match self.state {
            AppState::Loading => {
                // Only quit is allowed during loading
                if key_event.code == KeyCode::Char('q') {
                    self.should_quit = true;
                }
            }
            AppState::BeachList => {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.should_quit = true;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.move_selection_up();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.move_selection_down();
                    }
                    KeyCode::Enter => {
                        if let Some(beach) = self.selected_beach() {
                            self.state = AppState::BeachDetail(beach.id.to_string());
                        }
                    }
                    _ => {}
                }
            }
            AppState::BeachDetail(_) => {
                match key_event.code {
                    KeyCode::Char('q') => {
                        self.should_quit = true;
                    }
                    KeyCode::Esc => {
                        self.state = AppState::BeachList;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Moves the selection up in the list, wrapping to bottom if at top
    fn move_selection_up(&mut self) {
        let count = self.beach_count();
        if count == 0 {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = count - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    /// Moves the selection down in the list, wrapping to top if at bottom
    fn move_selection_down(&mut self) {
        let count = self.beach_count();
        if count == 0 {
            return;
        }
        self.selected_index = (self.selected_index + 1) % count;
    }

    /// Gets the beach conditions for a specific beach ID
    pub fn get_conditions(&self, beach_id: &str) -> Option<&BeachConditions> {
        self.beach_conditions.get(beach_id)
    }

    /// Gets the conditions for the currently selected beach
    #[allow(dead_code)]
    pub fn get_selected_conditions(&self) -> Option<&BeachConditions> {
        self.selected_beach()
            .and_then(|beach| self.beach_conditions.get(beach.id))
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Helper to create a KeyEvent for testing
    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_initial_state_is_loading() {
        let app = App::new();
        assert_eq!(app.state, AppState::Loading);
        assert_eq!(app.selected_index, 0);
        assert!(!app.should_quit);
        assert!(app.beach_conditions.is_empty());
    }

    #[test]
    fn test_state_transition_loading_to_beach_list() {
        let mut app = App::new();
        assert_eq!(app.state, AppState::Loading);

        // Manually transition (simulating load_all_data completion)
        app.state = AppState::BeachList;
        assert_eq!(app.state, AppState::BeachList);
    }

    #[test]
    fn test_state_transition_beach_list_to_detail() {
        let mut app = App::new();
        app.state = AppState::BeachList;

        // Press Enter to go to detail
        app.handle_key(key_event(KeyCode::Enter));

        match &app.state {
            AppState::BeachDetail(id) => {
                assert!(!id.is_empty());
            }
            _ => panic!("Expected BeachDetail state"),
        }
    }

    #[test]
    fn test_state_transition_detail_to_beach_list() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        // Press Esc to go back
        app.handle_key(key_event(KeyCode::Esc));
        assert_eq!(app.state, AppState::BeachList);
    }

    #[test]
    fn test_navigation_down_increases_index() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        assert_eq!(app.selected_index, 0);

        app.handle_key(key_event(KeyCode::Down));
        assert_eq!(app.selected_index, 1);

        app.handle_key(key_event(KeyCode::Down));
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_navigation_up_decreases_index() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        app.selected_index = 2;

        app.handle_key(key_event(KeyCode::Up));
        assert_eq!(app.selected_index, 1);

        app.handle_key(key_event(KeyCode::Up));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_navigation_wraps_at_bottom() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        let count = app.beach_count();
        app.selected_index = count - 1;

        app.handle_key(key_event(KeyCode::Down));
        assert_eq!(app.selected_index, 0, "Should wrap to top");
    }

    #[test]
    fn test_navigation_wraps_at_top() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        app.selected_index = 0;

        app.handle_key(key_event(KeyCode::Up));
        let count = app.beach_count();
        assert_eq!(app.selected_index, count - 1, "Should wrap to bottom");
    }

    #[test]
    fn test_vim_navigation_j_moves_down() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        assert_eq!(app.selected_index, 0);

        app.handle_key(key_event(KeyCode::Char('j')));
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn test_vim_navigation_k_moves_up() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        app.selected_index = 1;

        app.handle_key(key_event(KeyCode::Char('k')));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_q_quits_from_beach_list() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        assert!(!app.should_quit);

        app.handle_key(key_event(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_q_quits_from_beach_detail() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        assert!(!app.should_quit);

        app.handle_key(key_event(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_esc_quits_from_beach_list() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        assert!(!app.should_quit);

        app.handle_key(key_event(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn test_esc_goes_back_from_detail() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        app.handle_key(key_event(KeyCode::Esc));
        assert_eq!(app.state, AppState::BeachList);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_selected_beach_returns_correct_beach() {
        let mut app = App::new();
        app.selected_index = 0;

        let beach = app.selected_beach();
        assert!(beach.is_some());
        assert_eq!(beach.unwrap().id, "kitsilano");

        app.selected_index = 1;
        let beach = app.selected_beach();
        assert!(beach.is_some());
        assert_eq!(beach.unwrap().id, "english-bay");
    }

    #[test]
    fn test_beach_count_returns_12() {
        let app = App::new();
        assert_eq!(app.beach_count(), 12);
    }

    #[test]
    fn test_default_creates_same_as_new() {
        let app1 = App::new();
        let app2 = App::default();

        assert_eq!(app1.state, app2.state);
        assert_eq!(app1.selected_index, app2.selected_index);
        assert_eq!(app1.should_quit, app2.should_quit);
    }

    #[test]
    fn test_keys_ignored_during_loading() {
        let mut app = App::new();
        assert_eq!(app.state, AppState::Loading);

        // Navigation should be ignored
        app.handle_key(key_event(KeyCode::Down));
        assert_eq!(app.selected_index, 0);

        app.handle_key(key_event(KeyCode::Enter));
        assert_eq!(app.state, AppState::Loading);

        // But q should still work
        app.handle_key(key_event(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_get_conditions_returns_none_when_empty() {
        let app = App::new();
        assert!(app.get_conditions("kitsilano").is_none());
    }

    #[test]
    fn test_app_state_equality() {
        assert_eq!(AppState::Loading, AppState::Loading);
        assert_eq!(AppState::BeachList, AppState::BeachList);
        assert_eq!(
            AppState::BeachDetail("test".to_string()),
            AppState::BeachDetail("test".to_string())
        );
        assert_ne!(
            AppState::BeachDetail("test1".to_string()),
            AppState::BeachDetail("test2".to_string())
        );
        assert_ne!(AppState::Loading, AppState::BeachList);
    }
}
