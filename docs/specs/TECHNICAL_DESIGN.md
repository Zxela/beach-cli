# Technical Design: Vancouver Beach CLI

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI Entry                            │
│                      (main.rs)                              │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                    TUI Layer                                │
│                  (ratatui)                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │Beach List│  │Beach View│  │Loading   │                  │
│  │  Screen  │  │  Screen  │  │  Screen  │                  │
│  └──────────┘  └──────────┘  └──────────┘                  │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                   Data Layer                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Weather  │  │  Tides   │  │  Water   │  │  Cache   │   │
│  │ Client   │  │  Client  │  │  Quality │  │ Manager  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
vanbeach/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI setup
│   ├── app.rs               # Application state management
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── beach_list.rs    # Beach selection screen
│   │   ├── beach_detail.rs  # Individual beach view
│   │   └── widgets.rs       # Reusable UI components
│   ├── data/
│   │   ├── mod.rs
│   │   ├── beach.rs         # Beach definitions and coordinates
│   │   ├── weather.rs       # Open-Meteo client
│   │   ├── tides.rs         # DFO tides client
│   │   └── water_quality.rs # Vancouver Open Data client
│   └── cache/
│       ├── mod.rs
│       └── manager.rs       # Cache read/write/expiry
├── docs/
│   └── specs/
└── tests/
```

## Data Models

### Beach

```rust
pub struct Beach {
    pub id: &'static str,        // "kitsilano"
    pub name: &'static str,      // "Kitsilano Beach"
    pub latitude: f64,
    pub longitude: f64,
    pub water_quality_id: Option<&'static str>,  // ID in city data
}
```

### Weather

```rust
pub struct Weather {
    pub temperature_c: f64,
    pub feels_like_c: f64,
    pub condition: WeatherCondition,
    pub humidity_percent: u8,
    pub wind_speed_kmh: f64,
    pub wind_direction: String,
    pub uv_index: u8,
    pub sunrise: NaiveTime,
    pub sunset: NaiveTime,
    pub fetched_at: DateTime<Utc>,
}

pub enum WeatherCondition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Rain,
    Showers,
    Thunderstorm,
    Snow,
    Fog,
}
```

### Tides

```rust
pub struct TideInfo {
    pub current_height_m: f64,
    pub tide_state: TideState,
    pub next_high: TideEvent,
    pub next_low: TideEvent,
    pub fetched_at: DateTime<Utc>,
}

pub struct TideEvent {
    pub time: DateTime<Local>,
    pub height_m: f64,
}

pub enum TideState {
    Rising,
    Falling,
    High,
    Low,
}
```

### Water Quality

```rust
pub struct WaterQuality {
    pub status: WaterStatus,
    pub ecoli_count: Option<u32>,      // CFU/100mL
    pub sample_date: NaiveDate,
    pub advisory_reason: Option<String>,
    pub fetched_at: DateTime<Utc>,
}

pub enum WaterStatus {
    Safe,           // Green - OK to swim
    Advisory,       // Yellow - Swim at your own risk
    Closed,         // Red - Do not swim
    Unknown,        // Gray - No recent data
}
```

### Combined Beach Data

```rust
pub struct BeachConditions {
    pub beach: &'static Beach,
    pub weather: Option<Weather>,
    pub tides: Option<TideInfo>,
    pub water_quality: Option<WaterQuality>,
}
```

## API Contracts

### Open-Meteo (Weather)

```
GET https://api.open-meteo.com/v1/forecast
    ?latitude={lat}
    &longitude={lon}
    &current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m,wind_direction_10m
    &daily=sunrise,sunset,uv_index_max
    &timezone=America/Vancouver
```

### Fisheries and Oceans Canada (Tides)

```
GET https://www.waterlevels.gc.ca/eng/station?sid={station_id}
```
Station ID for Vancouver area: `7735` (Point Atkinson)

Note: May need to parse HTML or find JSON endpoint.

### City of Vancouver (Water Quality)

```
GET https://opendata.vancouver.ca/api/explore/v2.1/catalog/datasets/beach-water-quality/records
    ?where=beach_name='{beach_name}'
    &order_by=sample_date desc
    &limit=1
```

## UI Components

### Beach List Screen
```
╭─ Vancouver Beaches ─────────────────────────────╮
│                                                 │
│  ▸ Kitsilano Beach         22°C  🌤️   🟢      │
│    English Bay Beach       21°C  ☀️   🟢      │
│    Jericho Beach           20°C  🌤️   🟡      │
│    Spanish Banks           19°C  ☁️   🟢      │
│    ...                                         │
│                                                 │
│  ↑/↓ Navigate  Enter Select  q Quit           │
╰─────────────────────────────────────────────────╯
```

### Beach Detail Screen
```
╭─ Kitsilano Beach ───────────────────────────────╮
│                                                 │
│  WEATHER                      TIDES             │
│  ───────                      ─────             │
│  🌤️  22°C (feels 24°C)       ↗ Rising          │
│  💨 12 km/h W                 High: 2:34 PM     │
│  💧 65% humidity              Low:  8:45 PM     │
│  ☀️  UV Index: 6 (High)                        │
│  🌅 5:42 AM  🌇 9:12 PM                        │
│                                                 │
│  WATER QUALITY                                  │
│  ─────────────                                  │
│  🟢 Safe to swim                               │
│  Last tested: Jan 24, 2026                     │
│  E. coli: 45 CFU/100mL                         │
│                                                 │
│  ← Back  r Refresh  q Quit                     │
╰─────────────────────────────────────────────────╯
```

## Dependencies

```toml
[dependencies]
ratatui = "0.28"           # TUI framework
crossterm = "0.28"         # Terminal backend
tokio = { version = "1", features = ["full"] }  # Async runtime
reqwest = { version = "0.12", features = ["json"] }  # HTTP client
serde = { version = "1", features = ["derive"] }    # Serialization
serde_json = "1"           # JSON parsing
chrono = { version = "0.4", features = ["serde"] }  # Date/time
directories = "5"          # XDG paths for cache
thiserror = "1"            # Error handling
```

## Error Handling

- Use `thiserror` for custom error types
- Wrap API errors with context about which data source failed
- UI shows partial data when some sources fail
- Cache manager returns stale data with warning on fetch failure

## Testing Strategy

- **Unit tests:** Data parsing, cache expiry logic
- **Integration tests:** Mock HTTP responses for API clients
- **Manual testing:** TUI rendering across different terminals

## Security Considerations

- No API keys stored (all sources are public)
- Cache stored in user's home directory with standard permissions
- No sensitive data collected or transmitted
