//! Application state management for Vancouver Beach CLI
//!
//! This module contains the main application state, handling keyboard input,
//! data loading, and state transitions between different views.

use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;

use crate::activities::Activity;
use crate::cache::CacheManager;
use crate::cli::StartupConfig;
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
    /// Plan trip view showing beach/hour grid for activity optimization
    PlanTrip,
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
    /// Currently selected activity for scoring/filtering
    pub current_activity: Option<Activity>,
    /// Cursor position in PlanTrip grid (beach_index, hour_index)
    pub plan_cursor: (usize, usize),
    /// Visible hour range in PlanTrip screen (start_hour, end_hour), default 6am-9pm
    pub plan_time_range: (u8, u8),
    /// Flag to transition to PlanTrip after data loads (from --plan CLI flag)
    pub pending_plan_trip: bool,
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
            current_activity: None,
            plan_cursor: (0, 0),
            plan_time_range: (6, 21),
            pending_plan_trip: false,
            weather_client: WeatherClient::new(),
            tides_client: TidesClient::new(cache.clone()),
            water_quality_client: cache
                .map(WaterQualityClient::with_cache)
                .unwrap_or_default(),
        }
    }

    /// Creates a new App instance with the given startup configuration.
    ///
    /// This is used to apply CLI arguments like --plan to set the initial state.
    ///
    /// # Arguments
    /// * `config` - The startup configuration derived from CLI arguments
    pub fn with_startup_config(config: StartupConfig) -> Self {
        let mut app = Self::new();

        // Apply startup config
        if config.start_in_plan_trip {
            // Set a flag to transition to PlanTrip after data loads
            app.pending_plan_trip = true;
        }
        if let Some(activity) = config.initial_activity {
            app.current_activity = Some(activity);
        }

        app
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
            current_activity: None,
            plan_cursor: (0, 0),
            plan_time_range: (6, 21),
            pending_plan_trip: false,
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
            weather_futures.push(
                self.weather_client
                    .fetch_weather(beach.latitude, beach.longitude),
            );
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
            let weather = weather_results
                .get(i)
                .and_then(|r| r.as_ref().ok().cloned());

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

            self.beach_conditions
                .insert(beach.id.to_string(), conditions);
        }

        // Transition to appropriate state based on startup config
        if self.pending_plan_trip {
            self.state = AppState::PlanTrip;
            self.pending_plan_trip = false;
        } else {
            self.state = AppState::BeachList;
        }
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
            self.water_quality_client
                .fetch_water_quality(wq_id)
                .await
                .ok()
        } else {
            None
        };

        let conditions = BeachConditions {
            beach: *beach,
            weather,
            tides,
            water_quality,
        };

        self.beach_conditions
            .insert(beach_id.to_string(), conditions);
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
    /// - `p`: Open PlanTrip view (from BeachList or BeachDetail)
    /// - `1`-`5`: Set current activity (in BeachDetail)
    /// - `Esc` (in BeachDetail): Go back to list view
    /// - `Esc` (in PlanTrip): Go back to list view
    pub fn handle_key(&mut self, key_event: KeyEvent) {
        match self.state {
            AppState::Loading => {
                // Only quit is allowed during loading
                if key_event.code == KeyCode::Char('q') {
                    self.should_quit = true;
                }
            }
            AppState::BeachList => match key_event.code {
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
                KeyCode::Char('p') => {
                    self.state = AppState::PlanTrip;
                }
                _ => {}
            },
            AppState::BeachDetail(_) => match key_event.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Esc => {
                    self.state = AppState::BeachList;
                }
                KeyCode::Char('p') => {
                    self.state = AppState::PlanTrip;
                }
                KeyCode::Char('1') => {
                    self.current_activity = Some(Activity::Swimming);
                }
                KeyCode::Char('2') => {
                    self.current_activity = Some(Activity::Sunbathing);
                }
                KeyCode::Char('3') => {
                    self.current_activity = Some(Activity::Sailing);
                }
                KeyCode::Char('4') => {
                    self.current_activity = Some(Activity::Sunset);
                }
                KeyCode::Char('5') => {
                    self.current_activity = Some(Activity::Peace);
                }
                _ => {}
            },
            AppState::PlanTrip => {
                match key_event.code {
                    KeyCode::Char('q') => {
                        self.should_quit = true;
                    }
                    KeyCode::Esc => {
                        self.state = AppState::BeachList;
                    }
                    // Horizontal navigation (hours): h/Left and l/Right
                    KeyCode::Char('h') | KeyCode::Left => {
                        self.move_plan_cursor_left();
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        self.move_plan_cursor_right();
                    }
                    // Vertical navigation (beaches): k/Up and j/Down
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.move_plan_cursor_up();
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.move_plan_cursor_down();
                    }
                    // Activity selection (1-5)
                    KeyCode::Char('1') => {
                        self.current_activity = Some(Activity::Swimming);
                    }
                    KeyCode::Char('2') => {
                        self.current_activity = Some(Activity::Sunbathing);
                    }
                    KeyCode::Char('3') => {
                        self.current_activity = Some(Activity::Sailing);
                    }
                    KeyCode::Char('4') => {
                        self.current_activity = Some(Activity::Sunset);
                    }
                    KeyCode::Char('5') => {
                        self.current_activity = Some(Activity::Peace);
                    }
                    // Enter navigates to beach detail
                    KeyCode::Enter => {
                        if let Some(beach) = all_beaches().get(self.plan_cursor.0) {
                            self.state = AppState::BeachDetail(beach.id.to_string());
                        }
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

    /// Moves the plan cursor up (to previous beach), wrapping at top
    fn move_plan_cursor_up(&mut self) {
        let count = self.beach_count();
        if count == 0 {
            return;
        }
        if self.plan_cursor.0 == 0 {
            self.plan_cursor.0 = count - 1;
        } else {
            self.plan_cursor.0 -= 1;
        }
    }

    /// Moves the plan cursor down (to next beach), wrapping at bottom
    fn move_plan_cursor_down(&mut self) {
        let count = self.beach_count();
        if count == 0 {
            return;
        }
        self.plan_cursor.0 = (self.plan_cursor.0 + 1) % count;
    }

    /// Moves the plan cursor left (to previous hour), wrapping at start
    fn move_plan_cursor_left(&mut self) {
        let hour_count = (self.plan_time_range.1 - self.plan_time_range.0 + 1) as usize;
        if hour_count == 0 {
            return;
        }
        if self.plan_cursor.1 == 0 {
            self.plan_cursor.1 = hour_count - 1;
        } else {
            self.plan_cursor.1 -= 1;
        }
    }

    /// Moves the plan cursor right (to next hour), wrapping at end
    fn move_plan_cursor_right(&mut self) {
        let hour_count = (self.plan_time_range.1 - self.plan_time_range.0 + 1) as usize;
        if hour_count == 0 {
            return;
        }
        self.plan_cursor.1 = (self.plan_cursor.1 + 1) % hour_count;
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
    use crate::activities::Activity;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Helper to create a KeyEvent for testing
    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // ========================================================================
    // PlanTrip State and Activity Tests (Task 017)
    // ========================================================================

    #[test]
    fn test_app_state_plan_trip_variant_exists() {
        // AppState::PlanTrip should exist as a variant
        let state = AppState::PlanTrip;
        assert_eq!(state, AppState::PlanTrip);
    }

    #[test]
    fn test_app_has_current_activity_field() {
        let app = App::new();
        // current_activity should be None initially
        assert!(app.current_activity.is_none());
    }

    #[test]
    fn test_app_has_plan_cursor_field() {
        let app = App::new();
        // plan_cursor should be (0, 0) initially - (beach_index, hour_index)
        assert_eq!(app.plan_cursor, (0, 0));
    }

    #[test]
    fn test_app_has_plan_time_range_field() {
        let app = App::new();
        // plan_time_range should be (6, 21) by default - 6am to 9pm
        assert_eq!(app.plan_time_range, (6, 21));
    }

    #[test]
    fn test_key_p_in_beach_list_transitions_to_plan_trip() {
        let mut app = App::new();
        app.state = AppState::BeachList;

        app.handle_key(key_event(KeyCode::Char('p')));

        assert_eq!(app.state, AppState::PlanTrip);
    }

    #[test]
    fn test_key_p_in_beach_detail_transitions_to_plan_trip() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        app.handle_key(key_event(KeyCode::Char('p')));

        assert_eq!(app.state, AppState::PlanTrip);
    }

    #[test]
    fn test_key_1_in_beach_detail_sets_swimming_activity() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        app.handle_key(key_event(KeyCode::Char('1')));

        assert_eq!(app.current_activity, Some(Activity::Swimming));
    }

    #[test]
    fn test_key_2_in_beach_detail_sets_sunbathing_activity() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        app.handle_key(key_event(KeyCode::Char('2')));

        assert_eq!(app.current_activity, Some(Activity::Sunbathing));
    }

    #[test]
    fn test_key_3_in_beach_detail_sets_sailing_activity() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        app.handle_key(key_event(KeyCode::Char('3')));

        assert_eq!(app.current_activity, Some(Activity::Sailing));
    }

    #[test]
    fn test_key_4_in_beach_detail_sets_sunset_activity() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        app.handle_key(key_event(KeyCode::Char('4')));

        assert_eq!(app.current_activity, Some(Activity::Sunset));
    }

    #[test]
    fn test_key_5_in_beach_detail_sets_peace_activity() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        app.handle_key(key_event(KeyCode::Char('5')));

        assert_eq!(app.current_activity, Some(Activity::Peace));
    }

    #[test]
    fn test_esc_in_plan_trip_returns_to_beach_list() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;

        app.handle_key(key_event(KeyCode::Esc));

        assert_eq!(app.state, AppState::BeachList);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_activity_persists_when_navigating_to_plan_trip() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        // Set activity
        app.handle_key(key_event(KeyCode::Char('1')));
        assert_eq!(app.current_activity, Some(Activity::Swimming));

        // Navigate to PlanTrip
        app.handle_key(key_event(KeyCode::Char('p')));
        assert_eq!(app.state, AppState::PlanTrip);

        // Activity should still be set
        assert_eq!(app.current_activity, Some(Activity::Swimming));
    }

    #[test]
    fn test_activity_persists_when_returning_from_plan_trip() {
        let mut app = App::new();
        app.current_activity = Some(Activity::Sailing);
        app.state = AppState::PlanTrip;

        // Go back to beach list
        app.handle_key(key_event(KeyCode::Esc));

        // Activity should still be set
        assert_eq!(app.current_activity, Some(Activity::Sailing));
    }

    #[test]
    fn test_activity_persists_when_navigating_between_beaches() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.current_activity = Some(Activity::Sunbathing);

        // Go back to list
        app.handle_key(key_event(KeyCode::Esc));
        assert_eq!(app.current_activity, Some(Activity::Sunbathing));

        // Select another beach
        app.handle_key(key_event(KeyCode::Down));
        app.handle_key(key_event(KeyCode::Enter));

        // Activity should persist
        assert_eq!(app.current_activity, Some(Activity::Sunbathing));
    }

    #[test]
    fn test_q_quits_from_plan_trip() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        assert!(!app.should_quit);

        app.handle_key(key_event(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_app_state_plan_trip_equality() {
        assert_eq!(AppState::PlanTrip, AppState::PlanTrip);
        assert_ne!(AppState::PlanTrip, AppState::BeachList);
        assert_ne!(AppState::PlanTrip, AppState::Loading);
    }

    // ========================================================================
    // Original Tests
    // ========================================================================

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

    // ========================================================================
    // PlanTrip Navigation Tests (Task 020)
    // ========================================================================

    #[test]
    fn test_plan_trip_cursor_down_moves_to_next_beach() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        assert_eq!(app.plan_cursor.0, 0);

        app.handle_key(key_event(KeyCode::Down));
        assert_eq!(app.plan_cursor.0, 1);

        app.handle_key(key_event(KeyCode::Char('j')));
        assert_eq!(app.plan_cursor.0, 2);
    }

    #[test]
    fn test_plan_trip_cursor_up_moves_to_previous_beach() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        app.plan_cursor.0 = 2;

        app.handle_key(key_event(KeyCode::Up));
        assert_eq!(app.plan_cursor.0, 1);

        app.handle_key(key_event(KeyCode::Char('k')));
        assert_eq!(app.plan_cursor.0, 0);
    }

    #[test]
    fn test_plan_trip_cursor_right_moves_to_next_hour() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        assert_eq!(app.plan_cursor.1, 0);

        app.handle_key(key_event(KeyCode::Right));
        assert_eq!(app.plan_cursor.1, 1);

        app.handle_key(key_event(KeyCode::Char('l')));
        assert_eq!(app.plan_cursor.1, 2);
    }

    #[test]
    fn test_plan_trip_cursor_left_moves_to_previous_hour() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        app.plan_cursor.1 = 2;

        app.handle_key(key_event(KeyCode::Left));
        assert_eq!(app.plan_cursor.1, 1);

        app.handle_key(key_event(KeyCode::Char('h')));
        assert_eq!(app.plan_cursor.1, 0);
    }

    #[test]
    fn test_plan_trip_cursor_wraps_at_bottom() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        let count = app.beach_count();
        app.plan_cursor.0 = count - 1;

        app.handle_key(key_event(KeyCode::Down));
        assert_eq!(app.plan_cursor.0, 0, "Should wrap to top");
    }

    #[test]
    fn test_plan_trip_cursor_wraps_at_top() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        app.plan_cursor.0 = 0;

        app.handle_key(key_event(KeyCode::Up));
        let count = app.beach_count();
        assert_eq!(app.plan_cursor.0, count - 1, "Should wrap to bottom");
    }

    #[test]
    fn test_plan_trip_cursor_wraps_at_last_hour() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        // plan_time_range is (6, 21), so 16 hours (indices 0-15)
        let hour_count = (app.plan_time_range.1 - app.plan_time_range.0 + 1) as usize;
        app.plan_cursor.1 = hour_count - 1;

        app.handle_key(key_event(KeyCode::Right));
        assert_eq!(app.plan_cursor.1, 0, "Should wrap to first hour");
    }

    #[test]
    fn test_plan_trip_cursor_wraps_at_first_hour() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        app.plan_cursor.1 = 0;

        app.handle_key(key_event(KeyCode::Left));
        let hour_count = (app.plan_time_range.1 - app.plan_time_range.0 + 1) as usize;
        assert_eq!(
            app.plan_cursor.1,
            hour_count - 1,
            "Should wrap to last hour"
        );
    }

    #[test]
    fn test_plan_trip_activity_selection() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;

        app.handle_key(key_event(KeyCode::Char('1')));
        assert_eq!(app.current_activity, Some(Activity::Swimming));

        app.handle_key(key_event(KeyCode::Char('2')));
        assert_eq!(app.current_activity, Some(Activity::Sunbathing));

        app.handle_key(key_event(KeyCode::Char('3')));
        assert_eq!(app.current_activity, Some(Activity::Sailing));

        app.handle_key(key_event(KeyCode::Char('4')));
        assert_eq!(app.current_activity, Some(Activity::Sunset));

        app.handle_key(key_event(KeyCode::Char('5')));
        assert_eq!(app.current_activity, Some(Activity::Peace));
    }

    #[test]
    fn test_plan_trip_enter_navigates_to_beach_detail() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        app.plan_cursor.0 = 0; // First beach (kitsilano)

        app.handle_key(key_event(KeyCode::Enter));

        match &app.state {
            AppState::BeachDetail(id) => {
                assert_eq!(id, "kitsilano");
            }
            _ => panic!("Expected BeachDetail state"),
        }
    }

    #[test]
    fn test_plan_trip_enter_navigates_to_selected_beach() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        app.plan_cursor.0 = 1; // Second beach (english-bay)

        app.handle_key(key_event(KeyCode::Enter));

        match &app.state {
            AppState::BeachDetail(id) => {
                assert_eq!(id, "english-bay");
            }
            _ => panic!("Expected BeachDetail state"),
        }
    }

    // ========================================================================
    // Startup Config Tests (Task 021)
    // ========================================================================

    #[test]
    fn test_with_startup_config_default_starts_in_loading() {
        let config = StartupConfig::default();
        let app = App::with_startup_config(config);
        assert_eq!(app.state, AppState::Loading);
        assert!(!app.pending_plan_trip);
        assert!(app.current_activity.is_none());
    }

    #[test]
    fn test_with_startup_config_plan_only_sets_pending_flag() {
        let config = StartupConfig {
            start_in_plan_trip: true,
            initial_activity: None,
        };
        let app = App::with_startup_config(config);
        assert_eq!(app.state, AppState::Loading);
        assert!(app.pending_plan_trip);
        assert!(app.current_activity.is_none());
    }

    #[test]
    fn test_with_startup_config_plan_with_activity_sets_both() {
        let config = StartupConfig {
            start_in_plan_trip: true,
            initial_activity: Some(Activity::Swimming),
        };
        let app = App::with_startup_config(config);
        assert_eq!(app.state, AppState::Loading);
        assert!(app.pending_plan_trip);
        assert_eq!(app.current_activity, Some(Activity::Swimming));
    }

    #[test]
    fn test_pending_plan_trip_cleared_after_data_load() {
        let config = StartupConfig {
            start_in_plan_trip: true,
            initial_activity: None,
        };
        let mut app = App::with_startup_config(config);
        assert!(app.pending_plan_trip);

        // Simulate data load completion by manually setting state
        // (In real usage, load_all_data would do this)
        if app.pending_plan_trip {
            app.state = AppState::PlanTrip;
            app.pending_plan_trip = false;
        }

        assert_eq!(app.state, AppState::PlanTrip);
        assert!(!app.pending_plan_trip);
    }

    #[test]
    fn test_app_new_has_pending_plan_trip_false() {
        let app = App::new();
        assert!(!app.pending_plan_trip);
    }
}
