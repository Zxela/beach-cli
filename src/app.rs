//! Application state management for Vancouver Beach CLI
//!
//! This module contains the main application state, handling keyboard input,
//! data loading, and state transitions between different views.

use chrono::{DateTime, Local};
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
    /// Timestamp of last data refresh
    pub last_refresh: Option<DateTime<Local>>,
    /// Flag indicating a refresh has been requested
    pub refresh_requested: bool,
    /// Flag to show help overlay
    pub show_help: bool,
    /// Scroll offset for beach detail view
    pub detail_scroll_offset: u16,
    /// Whether tide chart is expanded in detail view
    pub tide_chart_expanded: bool,
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
            last_refresh: None,
            refresh_requested: false,
            show_help: false,
            detail_scroll_offset: 0,
            tide_chart_expanded: false,
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
            last_refresh: None,
            refresh_requested: false,
            show_help: false,
            detail_scroll_offset: 0,
            tide_chart_expanded: false,
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

        // Record refresh time
        self.last_refresh = Some(Local::now());

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
        // Handle help overlay - intercepts all keys when shown
        if self.show_help {
            match key_event.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                    self.show_help = false;
                }
                _ => {} // Ignore other keys when help is shown
            }
            return;
        }

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
                KeyCode::Char('r') => {
                    self.refresh_requested = true;
                }
                KeyCode::Char('?') => {
                    self.show_help = true;
                }
                _ => {}
            },
            AppState::BeachDetail(_) => match key_event.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Esc => {
                    self.reset_detail_view_state();
                    self.state = AppState::BeachList;
                }
                KeyCode::Char('p') => {
                    self.reset_detail_view_state();
                    self.state = AppState::PlanTrip;
                }
                // Scroll navigation
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll_up();
                }
                KeyCode::Char('g') => {
                    self.scroll_to_top();
                }
                KeyCode::Char('G') => {
                    self.scroll_to_bottom();
                }
                // Activity selection
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
                KeyCode::Char('r') => {
                    self.refresh_requested = true;
                }
                KeyCode::Char('?') => {
                    self.show_help = true;
                }
                KeyCode::Char('t') => {
                    self.toggle_tide_chart();
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

    /// Scrolls up in the detail view with bounds checking
    ///
    /// Decreases scroll offset by 1, stopping at 0.
    pub fn scroll_up(&mut self) {
        self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(1);
    }

    /// Scrolls down in the detail view with bounds checking
    ///
    /// Increases scroll offset by 1, with a maximum limit.
    /// The actual maximum depends on content height, but we use a reasonable upper bound.
    pub fn scroll_down(&mut self) {
        // Use a reasonable maximum scroll offset (can be adjusted based on content)
        const MAX_SCROLL: u16 = 100;
        if self.detail_scroll_offset < MAX_SCROLL {
            self.detail_scroll_offset += 1;
        }
    }

    /// Scrolls to the top of the detail view
    ///
    /// Resets scroll offset to 0.
    pub fn scroll_to_top(&mut self) {
        self.detail_scroll_offset = 0;
    }

    /// Scrolls to the bottom of the detail view
    ///
    /// Sets scroll offset to a large value that will be clamped by the renderer.
    pub fn scroll_to_bottom(&mut self) {
        // Set to a large value; the renderer will clamp to actual max
        self.detail_scroll_offset = 100;
    }

    /// Toggles the tide chart expansion state
    pub fn toggle_tide_chart(&mut self) {
        self.tide_chart_expanded = !self.tide_chart_expanded;
    }

    /// Resets detail view state when navigating away
    ///
    /// Called when leaving the detail view to reset scroll position
    /// and tide chart expansion state for the next detail view visit.
    pub fn reset_detail_view_state(&mut self) {
        self.detail_scroll_offset = 0;
        self.tide_chart_expanded = false;
    }

    /// Finds the best beach for the current activity right now
    ///
    /// Returns the best beach with a score >= 70, or None if no good options exist.
    pub fn find_best_beach_now(&self) -> Option<BestBeachNow> {
        use crate::activities::get_profile;
        use crate::crowd::estimate_crowd;
        use chrono::{Datelike, Timelike};

        let activity = self.current_activity?;
        let now = chrono::Local::now();
        let current_hour = now.hour() as u8;

        let profile = get_profile(activity);
        let beaches = all_beaches();

        let mut best: Option<BestBeachNow> = None;
        let mut best_score: u8 = 70; // Minimum threshold

        for beach in beaches {
            let conditions = self.beach_conditions.get(beach.id)?;

            // Skip if water quality is stale for swimming
            if activity == crate::activities::Activity::Swimming {
                if let Some(wq) = &conditions.water_quality {
                    if wq.is_stale() {
                        continue;
                    }
                }
            }

            let weather = conditions.weather.as_ref()?;
            let temp = weather.temperature as f32;
            let wind = weather.wind as f32;
            let uv = weather.uv as f32;

            let water_status = conditions
                .water_quality
                .as_ref()
                .map(|wq| wq.effective_status())
                .unwrap_or(crate::data::WaterStatus::Unknown);

            let (tide_height, max_tide) = conditions
                .tides
                .as_ref()
                .map(|t| (t.current_height as f32, 4.8f32))
                .unwrap_or((2.4, 4.8));

            let crowd = estimate_crowd(now.month(), now.weekday(), now.hour());

            let score_result = profile.score_time_slot(
                current_hour,
                beach.id,
                temp,
                wind,
                uv,
                water_status,
                tide_height,
                max_tide,
                crowd,
            );

            if score_result.score > best_score {
                best_score = score_result.score;

                let mut reasons = Vec::new();
                reasons.push(format!("{:.0}°C", temp));
                if wind < 10.0 {
                    reasons.push("calm winds".to_string());
                }
                if water_status == crate::data::WaterStatus::Safe {
                    reasons.push("safe water".to_string());
                }

                best = Some(BestBeachNow {
                    beach_name: beach.name.to_string(),
                    beach_id: beach.id.to_string(),
                    score: score_result.score,
                    reasons,
                });
            }
        }

        best
    }
}

/// Information about the best beach right now
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BestBeachNow {
    /// Name of the beach
    pub beach_name: String,
    /// ID of the beach
    pub beach_id: String,
    /// Activity score (0-100)
    pub score: u8,
    /// Reasons why this beach is recommended
    pub reasons: Vec<String>,
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

    // ========================================================================
    // find_best_beach_now Tests (Task 5)
    // ========================================================================

    #[test]
    fn test_find_best_beach_now_returns_none_without_activity() {
        let app = App::new();
        assert!(app.find_best_beach_now().is_none());
    }

    #[test]
    fn test_find_best_beach_now_returns_none_without_data() {
        let mut app = App::new();
        app.current_activity = Some(Activity::Swimming);
        // No beach conditions loaded, should return None
        assert!(app.find_best_beach_now().is_none());
    }

    // ========================================================================
    // Detail View State Tests (Task 103)
    // ========================================================================

    #[test]
    fn test_detail_scroll_offset_initial_value() {
        let app = App::new();
        assert_eq!(app.detail_scroll_offset, 0);
    }

    #[test]
    fn test_tide_chart_expanded_initial_value() {
        let app = App::new();
        assert!(!app.tide_chart_expanded);
    }

    #[test]
    fn test_scroll_up_decreases_offset() {
        let mut app = App::new();
        app.detail_scroll_offset = 5;

        app.scroll_up();
        assert_eq!(app.detail_scroll_offset, 4);

        app.scroll_up();
        assert_eq!(app.detail_scroll_offset, 3);
    }

    #[test]
    fn test_scroll_up_stops_at_zero() {
        let mut app = App::new();
        app.detail_scroll_offset = 1;

        app.scroll_up();
        assert_eq!(app.detail_scroll_offset, 0);

        // Should stay at 0, not underflow
        app.scroll_up();
        assert_eq!(app.detail_scroll_offset, 0);
    }

    #[test]
    fn test_scroll_down_increases_offset() {
        let mut app = App::new();
        assert_eq!(app.detail_scroll_offset, 0);

        app.scroll_down();
        assert_eq!(app.detail_scroll_offset, 1);

        app.scroll_down();
        assert_eq!(app.detail_scroll_offset, 2);
    }

    #[test]
    fn test_scroll_down_respects_maximum() {
        let mut app = App::new();
        app.detail_scroll_offset = 100; // MAX_SCROLL

        app.scroll_down();
        assert_eq!(app.detail_scroll_offset, 100); // Should not exceed max
    }

    #[test]
    fn test_toggle_tide_chart() {
        let mut app = App::new();
        assert!(!app.tide_chart_expanded);

        app.toggle_tide_chart();
        assert!(app.tide_chart_expanded);

        app.toggle_tide_chart();
        assert!(!app.tide_chart_expanded);
    }

    #[test]
    fn test_reset_detail_view_state() {
        let mut app = App::new();
        app.detail_scroll_offset = 10;
        app.tide_chart_expanded = true;

        app.reset_detail_view_state();

        assert_eq!(app.detail_scroll_offset, 0);
        assert!(!app.tide_chart_expanded);
    }

    #[test]
    fn test_detail_view_state_resets_on_esc() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.detail_scroll_offset = 5;
        app.tide_chart_expanded = true;

        app.handle_key(key_event(KeyCode::Esc));

        assert_eq!(app.state, AppState::BeachList);
        assert_eq!(app.detail_scroll_offset, 0);
        assert!(!app.tide_chart_expanded);
    }

    #[test]
    fn test_detail_view_state_resets_on_plan_trip_navigation() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.detail_scroll_offset = 7;
        app.tide_chart_expanded = true;

        app.handle_key(key_event(KeyCode::Char('p')));

        assert_eq!(app.state, AppState::PlanTrip);
        assert_eq!(app.detail_scroll_offset, 0);
        assert!(!app.tide_chart_expanded);
    }

    #[test]
    fn test_with_clients_initializes_detail_view_state() {
        let weather_client = WeatherClient::new();
        let tides_client = TidesClient::new(None);
        let water_quality_client = WaterQualityClient::default();

        let app = App::with_clients(weather_client, tides_client, water_quality_client);

        assert_eq!(app.detail_scroll_offset, 0);
        assert!(!app.tide_chart_expanded);
    }

    // ========================================================================
    // Scroll Support Tests (Task 105)
    // ========================================================================

    #[test]
    fn test_scroll_to_top() {
        let mut app = App::new();
        app.detail_scroll_offset = 50;

        app.scroll_to_top();

        assert_eq!(app.detail_scroll_offset, 0);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut app = App::new();
        app.detail_scroll_offset = 0;

        app.scroll_to_bottom();

        assert_eq!(app.detail_scroll_offset, 100);
    }

    #[test]
    fn test_detail_view_j_scrolls_down() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.detail_scroll_offset = 0;

        app.handle_key(key_event(KeyCode::Char('j')));

        assert_eq!(app.detail_scroll_offset, 1);
    }

    #[test]
    fn test_detail_view_k_scrolls_up() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.detail_scroll_offset = 5;

        app.handle_key(key_event(KeyCode::Char('k')));

        assert_eq!(app.detail_scroll_offset, 4);
    }

    #[test]
    fn test_detail_view_down_arrow_scrolls_down() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.detail_scroll_offset = 0;

        app.handle_key(key_event(KeyCode::Down));

        assert_eq!(app.detail_scroll_offset, 1);
    }

    #[test]
    fn test_detail_view_up_arrow_scrolls_up() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.detail_scroll_offset = 5;

        app.handle_key(key_event(KeyCode::Up));

        assert_eq!(app.detail_scroll_offset, 4);
    }

    #[test]
    fn test_detail_view_g_scrolls_to_top() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.detail_scroll_offset = 25;

        app.handle_key(key_event(KeyCode::Char('g')));

        assert_eq!(app.detail_scroll_offset, 0);
    }

    #[test]
    fn test_detail_view_capital_g_scrolls_to_bottom() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        app.detail_scroll_offset = 0;

        app.handle_key(key_event(KeyCode::Char('G')));

        assert_eq!(app.detail_scroll_offset, 100);
    }

    #[test]
    fn test_scroll_keys_dont_change_state() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());

        app.handle_key(key_event(KeyCode::Char('j')));
        assert!(matches!(app.state, AppState::BeachDetail(_)));

        app.handle_key(key_event(KeyCode::Char('k')));
        assert!(matches!(app.state, AppState::BeachDetail(_)));

        app.handle_key(key_event(KeyCode::Char('g')));
        assert!(matches!(app.state, AppState::BeachDetail(_)));

        app.handle_key(key_event(KeyCode::Char('G')));
        assert!(matches!(app.state, AppState::BeachDetail(_)));
    }

    // ========================================================================
    // Tide Chart Toggle Tests (Task 107)
    // ========================================================================

    #[test]
    fn test_t_key_toggles_tide_chart_in_beach_detail() {
        let mut app = App::new();
        app.state = AppState::BeachDetail("kitsilano".to_string());
        assert!(!app.tide_chart_expanded, "Should start collapsed");

        app.handle_key(key_event(KeyCode::Char('t')));
        assert!(app.tide_chart_expanded, "Should be expanded after 't'");

        app.handle_key(key_event(KeyCode::Char('t')));
        assert!(!app.tide_chart_expanded, "Should be collapsed after second 't'");
    }

    #[test]
    fn test_t_key_does_nothing_in_beach_list() {
        let mut app = App::new();
        app.state = AppState::BeachList;
        assert!(!app.tide_chart_expanded);

        app.handle_key(key_event(KeyCode::Char('t')));
        // 't' in BeachList should not toggle tide chart (no handler)
        assert!(!app.tide_chart_expanded, "t key should not toggle in BeachList");
    }

    #[test]
    fn test_t_key_does_nothing_in_plan_trip() {
        let mut app = App::new();
        app.state = AppState::PlanTrip;
        assert!(!app.tide_chart_expanded);

        app.handle_key(key_event(KeyCode::Char('t')));
        // 't' in PlanTrip should not toggle tide chart (no handler)
        assert!(!app.tide_chart_expanded, "t key should not toggle in PlanTrip");
    }
}
