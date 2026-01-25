# Technical Design: Best Time to Go Feature

## Architecture Overview

```
                    CLI Entry (main.rs)
                         │
                         │ --plan [activity]
                         ▼
┌────────────────────────────────────────────────────────────────┐
│                      App State (app.rs)                        │
│  ┌──────────┐  ┌───────────┐  ┌────────────┐  ┌────────────┐  │
│  │ Loading  │  │ BeachList │  │BeachDetail │  │  PlanTrip  │  │
│  └──────────┘  └───────────┘  └────────────┘  └────────────┘  │
│       │              │              │               │          │
│       └──────────────┴──────────────┴───────────────┘          │
│                              │                                  │
│                    current_activity: Option<Activity>          │
└────────────────────────────────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ Scoring     │  │ Data Layer  │  │ UI Layer    │
│ Engine      │  │             │  │             │
│             │  │ - Weather   │  │ - List      │
│ activities  │◄─┤ - Tides     │  │ - Detail    │
│ .rs         │  │ - Crowd     │  │ - PlanTrip  │
└─────────────┘  └─────────────┘  └─────────────┘
```

## Project Structure Changes

```
src/
├── main.rs                 # Add CLI arg parsing
├── app.rs                  # Add PlanTrip state, activity tracking
├── activities.rs           # NEW: Activity profiles, scoring engine
├── crowd.rs                # NEW: Crowd heuristics
├── data/
│   ├── mod.rs              # Export new types
│   ├── weather.rs          # Add hourly forecast fetching
│   └── tides.rs            # Add height interpolation
└── ui/
    ├── mod.rs              # Export PlanTrip
    ├── beach_list.rs       # Add contextual hints
    ├── beach_detail.rs     # Add activity filter, best window
    └── plan_trip.rs        # NEW: Plan Trip screen
```

## Data Models

### Activity and Profile

```rust
// src/activities.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Swimming,
    Sunbathing,
    Sailing,
    Sunset,
    Peace,
}

impl Activity {
    pub fn all() -> &'static [Activity] {
        &[Activity::Swimming, Activity::Sunbathing,
          Activity::Sailing, Activity::Sunset, Activity::Peace]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Activity::Swimming => "Swimming",
            Activity::Sunbathing => "Sunbathing",
            Activity::Sailing => "Sailing",
            Activity::Sunset => "Sunset",
            Activity::Peace => "Peace",
        }
    }

    pub fn from_str(s: &str) -> Option<Activity> {
        match s.to_lowercase().as_str() {
            "swim" | "swimming" => Some(Activity::Swimming),
            "sun" | "sunbathing" | "sunbathe" => Some(Activity::Sunbathing),
            "sail" | "sailing" => Some(Activity::Sailing),
            "sunset" => Some(Activity::Sunset),
            "peace" | "quiet" => Some(Activity::Peace),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TidePreference {
    High,        // Sailing - deeper water
    Mid,         // Swimming - good depth, not too shallow
    Low,         // Tide pools, beach walking
    Any,         // Doesn't matter
}

#[derive(Debug, Clone, Copy)]
pub enum UvPreference {
    High,        // Sunbathing
    Moderate,    // Swimming
    Low,         // Peace (shady, less harsh)
    Any,         // Doesn't matter
}

pub struct ActivityProfile {
    pub activity: Activity,
    pub temp_weight: f32,
    pub temp_ideal_range: (f32, f32),      // Celsius
    pub water_quality_weight: f32,          // 0 = ignore, 1 = critical
    pub wind_weight: f32,
    pub wind_ideal_range: (f32, f32),       // km/h
    pub uv_weight: f32,
    pub uv_preference: UvPreference,
    pub tide_weight: f32,
    pub tide_preference: TidePreference,
    pub crowd_weight: f32,                  // Higher = more crowd-averse
    pub time_of_day_scorer: Option<fn(u8) -> f32>,  // Custom time scoring
}
```

### Hourly Weather

```rust
// src/data/weather.rs (additions)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HourlyForecast {
    pub time: DateTime<Utc>,
    pub temperature: f32,
    pub weather_code: u8,
    pub wind_speed: f32,
    pub uv_index: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeatherData {
    pub current: Weather,
    pub hourly: Vec<HourlyForecast>,  // Next 48 hours
}
```

### Time Slot Score

```rust
// src/activities.rs

#[derive(Debug, Clone)]
pub struct ScoreFactors {
    pub temperature: f32,     // 0.0-1.0
    pub water_quality: f32,
    pub wind: f32,
    pub uv: f32,
    pub tide: f32,
    pub crowd: f32,
    pub time_of_day: f32,
}

#[derive(Debug, Clone)]
pub struct TimeSlotScore {
    pub time: DateTime<Local>,
    pub beach_id: String,
    pub activity: Activity,
    pub score: u8,            // 0-100
    pub factors: ScoreFactors,
}
```

## Scoring Engine

### Factor Evaluation

Each factor is evaluated to a 0.0-1.0 "goodness" score:

```rust
impl ActivityProfile {
    /// Score temperature based on ideal range
    pub fn score_temperature(&self, temp: f32) -> f32 {
        let (min, max) = self.temp_ideal_range;
        if temp < min - 5.0 || temp > max + 5.0 {
            0.0  // Way outside range
        } else if temp >= min && temp <= max {
            1.0  // Perfect
        } else if temp < min {
            // Below ideal, scale from 0 to 1
            ((temp - (min - 5.0)) / 5.0).clamp(0.0, 1.0)
        } else {
            // Above ideal, scale from 1 to 0
            ((max + 5.0 - temp) / 5.0).clamp(0.0, 1.0)
        }
    }

    /// Score wind based on ideal range
    pub fn score_wind(&self, wind: f32) -> f32 {
        let (min, max) = self.wind_ideal_range;
        if wind >= min && wind <= max {
            1.0
        } else if wind < min {
            (wind / min).clamp(0.0, 1.0)
        } else {
            ((max * 1.5 - wind) / (max * 0.5)).clamp(0.0, 1.0)
        }
    }

    /// Score water quality (binary for safety)
    pub fn score_water_quality(&self, status: WaterStatus) -> f32 {
        match status {
            WaterStatus::Safe => 1.0,
            WaterStatus::Advisory => 0.3,
            WaterStatus::Closed => 0.0,
            WaterStatus::Unknown => 0.5,
        }
    }

    /// Score UV index
    pub fn score_uv(&self, uv: f32) -> f32 {
        match self.uv_preference {
            UvPreference::High => (uv / 8.0).clamp(0.0, 1.0),
            UvPreference::Moderate => 1.0 - ((uv - 5.0).abs() / 5.0).clamp(0.0, 1.0),
            UvPreference::Low => (1.0 - uv / 6.0).clamp(0.0, 1.0),
            UvPreference::Any => 1.0,
        }
    }

    /// Score tide state
    pub fn score_tide(&self, height: f32, max_height: f32) -> f32 {
        let normalized = height / max_height;  // 0.0 = low, 1.0 = high
        match self.tide_preference {
            TidePreference::High => normalized,
            TidePreference::Mid => 1.0 - (normalized - 0.5).abs() * 2.0,
            TidePreference::Low => 1.0 - normalized,
            TidePreference::Any => 1.0,
        }
    }

    /// Score crowd level (inverted - high crowd = low score)
    pub fn score_crowd(&self, crowd_level: f32) -> f32 {
        1.0 - (crowd_level * self.crowd_weight).clamp(0.0, 1.0)
    }
}
```

### Combined Scoring

```rust
impl ActivityProfile {
    pub fn score_time_slot(
        &self,
        hour: u8,
        weather: &HourlyForecast,
        tide_height: f32,
        max_tide: f32,
        water_quality: WaterStatus,
        crowd_level: f32,
    ) -> TimeSlotScore {
        let factors = ScoreFactors {
            temperature: self.score_temperature(weather.temperature),
            water_quality: self.score_water_quality(water_quality),
            wind: self.score_wind(weather.wind_speed),
            uv: self.score_uv(weather.uv_index),
            tide: self.score_tide(tide_height, max_tide),
            crowd: self.score_crowd(crowd_level),
            time_of_day: self.time_of_day_scorer
                .map(|f| f(hour))
                .unwrap_or(1.0),
        };

        let weighted_sum =
            factors.temperature * self.temp_weight +
            factors.water_quality * self.water_quality_weight +
            factors.wind * self.wind_weight +
            factors.uv * self.uv_weight +
            factors.tide * self.tide_weight +
            factors.crowd * self.crowd_weight +
            factors.time_of_day * 0.1;  // Slight time preference

        let total_weight =
            self.temp_weight + self.water_quality_weight +
            self.wind_weight + self.uv_weight +
            self.tide_weight + self.crowd_weight + 0.1;

        let score = ((weighted_sum / total_weight) * 100.0) as u8;

        TimeSlotScore {
            time: /* construct from hour */,
            beach_id: /* passed in */,
            activity: self.activity,
            score,
            factors,
        }
    }
}
```

### Default Profiles

```rust
pub fn get_profile(activity: Activity) -> ActivityProfile {
    match activity {
        Activity::Swimming => ActivityProfile {
            activity: Activity::Swimming,
            temp_weight: 0.3,
            temp_ideal_range: (20.0, 28.0),
            water_quality_weight: 0.4,  // Critical
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
            crowd_weight: 0.7,  // Highly crowd-averse
            time_of_day_scorer: Some(peace_time_scorer),
        },
    }
}

fn sunset_time_scorer(hour: u8) -> f32 {
    // Peak score 1-2 hours before sunset (roughly 7-8pm in summer)
    match hour {
        18..=20 => 1.0,
        17 | 21 => 0.7,
        16 | 22 => 0.3,
        _ => 0.1,
    }
}

fn peace_time_scorer(hour: u8) -> f32 {
    // Early morning is best
    match hour {
        6..=7 => 1.0,
        8 => 0.8,
        5 | 9 => 0.5,
        _ => 0.2,
    }
}
```

## Crowd Heuristics

```rust
// src/crowd.rs

use chrono::Weekday;

/// Estimate crowd level from 0.0 (empty) to 1.0 (packed)
pub fn estimate_crowd(month: u32, weekday: Weekday, hour: u32) -> f32 {
    let season_factor = match month {
        6..=8 => 1.0,       // Summer - busiest
        5 | 9 => 0.6,       // Shoulder season
        4 | 10 => 0.3,      // Spring/fall
        _ => 0.1,           // Winter - minimal
    };

    let day_factor = match weekday {
        Weekday::Sat | Weekday::Sun => 1.0,
        Weekday::Fri => 0.7,
        _ => 0.4,
    };

    let hour_factor = match hour {
        12..=16 => 1.0,     // Peak afternoon
        10..=11 | 17..=18 => 0.7,
        8..=9 | 19..=20 => 0.4,
        6..=7 | 21 => 0.2,
        _ => 0.1,
    };

    (season_factor * day_factor * hour_factor).clamp(0.0, 1.0)
}
```

## State Management

### Updated AppState

```rust
// src/app.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    Loading,
    BeachList,
    BeachDetail(String),
    PlanTrip,
}

pub struct App {
    pub state: AppState,
    pub selected_index: usize,
    pub beach_conditions: HashMap<String, BeachConditions>,
    pub should_quit: bool,

    // New fields
    pub current_activity: Option<Activity>,
    pub plan_cursor: (usize, usize),  // (beach_index, hour_index)
    pub plan_time_range: (u8, u8),    // Visible hour range (e.g., 6, 21)

    // Clients
    weather_client: WeatherClient,
    tides_client: TidesClient,
    water_quality_client: WaterQualityClient,
}
```

### Key Bindings

```rust
// Beach List additions
KeyCode::Char('p') => {
    self.state = AppState::PlanTrip;
}

// Beach Detail additions
KeyCode::Char('1') => self.current_activity = Some(Activity::Swimming),
KeyCode::Char('2') => self.current_activity = Some(Activity::Sunbathing),
KeyCode::Char('3') => self.current_activity = Some(Activity::Sailing),
KeyCode::Char('4') => self.current_activity = Some(Activity::Sunset),
KeyCode::Char('5') => self.current_activity = Some(Activity::Peace),
KeyCode::Char('p') => self.state = AppState::PlanTrip,

// Plan Trip screen
KeyCode::Char('h') | KeyCode::Left => self.plan_cursor.1 = self.plan_cursor.1.saturating_sub(1),
KeyCode::Char('l') | KeyCode::Right => self.plan_cursor.1 = (self.plan_cursor.1 + 1).min(hours - 1),
KeyCode::Char('j') | KeyCode::Down => self.plan_cursor.0 = (self.plan_cursor.0 + 1) % beaches,
KeyCode::Char('k') | KeyCode::Up => /* wrap up */,
KeyCode::Enter => {
    let beach_id = all_beaches()[self.plan_cursor.0].id;
    self.state = AppState::BeachDetail(beach_id.to_string());
}
KeyCode::Esc => self.state = AppState::BeachList,
```

## CLI Arguments

```rust
// src/main.rs

use clap::Parser;

#[derive(Parser)]
#[command(name = "vanbeach")]
#[command(about = "Vancouver beach conditions and trip planning")]
struct Cli {
    /// Open directly in plan mode
    #[arg(long)]
    plan: Option<Option<String>>,  // --plan or --plan swim
}

fn main() {
    let cli = Cli::parse();

    let mut app = App::new();

    if let Some(activity_opt) = cli.plan {
        app.state = AppState::PlanTrip;
        if let Some(activity_str) = activity_opt {
            app.current_activity = Activity::from_str(&activity_str);
        }
    }

    // ... rest of app startup
}
```

## UI Components

### Plan Trip Screen Layout

```
┌─ Plan Your Trip ─────────────────────────────────────────┐
│ Activity: [●Swimming] [○Sunbathing] [○Sailing] [○Sunset] │
├──────────────────────────────────────────────────────────┤
│           6am  8am  10am 12pm  2pm  4pm  6pm  8pm        │
│ Kitsilano  ░░   ▒▒   ▓▓   ██   ██   ▓▓   ▒▒   ░░        │
│ English B  ░░   ▒▒   ▓▓   ██   ▓▓   ▓▓   ▒▒   ░░        │
│ Jericho    ░░   ▓▓   ██   ██   ▓▓   ▒▒   ▒▒   ░░        │
│ Spanish B  ░░   ▒▒   ▓▓   ▓▓   ▓▓   ▒▒   ░░   ░░        │
│ ...                                                       │
├──────────────────────────────────────────────────────────┤
│ Best: Jericho @ 11am (92/100) - warm, safe water, quiet  │
│                                                          │
│ ←/→ hours  ↑/↓ beaches  1-5 activity  Enter select  Esc  │
└──────────────────────────────────────────────────────────┘
```

### Color Coding

```rust
fn score_to_color(score: u8) -> Color {
    match score {
        80..=100 => Color::Green,
        60..=79 => Color::LightGreen,
        40..=59 => Color::Yellow,
        20..=39 => Color::LightRed,
        _ => Color::Red,
    }
}

fn score_to_block(score: u8) -> &'static str {
    match score {
        80..=100 => "██",
        60..=79 => "▓▓",
        40..=59 => "▒▒",
        _ => "░░",
    }
}
```

## Dependencies

Add to `Cargo.toml`:

```toml
clap = { version = "4", features = ["derive"] }
```

## Testing Strategy

### Unit Tests

- Scoring functions with known inputs produce expected outputs
- Activity profiles have valid weights (sum > 0)
- Crowd heuristics return values in 0.0-1.0 range
- Tide interpolation is accurate

### Integration Tests

- Hourly weather fetch returns valid data structure
- Screen state transitions work correctly
- Activity persists across screen changes

### Manual Testing

1. `cargo run` - verify existing functionality unchanged
2. Press `p` from beach list - Plan screen appears
3. Press `1-5` - activity changes, grid updates
4. Navigate grid with arrows/hjkl
5. Press Enter - goes to beach detail
6. `cargo run -- --plan swim` - starts in Plan with Swimming
