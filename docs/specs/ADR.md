# Architecture Decision Record: Vancouver Beach CLI

## ADR-001: Programming Language

### Context
We need to choose a language for building an interactive CLI that aggregates data from multiple APIs and displays it with colors and formatting.

### Options Considered

1. **Python** — Fast to develop, rich ecosystem (requests, rich, textual), but requires Python runtime
2. **Node.js/TypeScript** — Good CLI libraries (ink, chalk), but requires Node runtime
3. **Go** — Single binary, fast startup, good CLI libraries, but less expressive for TUI
4. **Rust** — Single binary, excellent performance, strong TUI ecosystem (ratatui), learning opportunity

### Decision
**Rust** — Chosen for:
- Single binary distribution (no runtime dependencies)
- Excellent TUI libraries (ratatui for interactive menus)
- Strong HTTP client ecosystem (reqwest)
- Learning opportunity for the developer
- Fast startup time

### Consequences
- **Positive:** Self-contained binary, fast execution, type safety
- **Negative:** Longer initial development time, steeper learning curve

---

## ADR-002: TUI Framework

### Context
The CLI needs an interactive menu system with colorful output and icons.

### Options Considered

1. **ratatui** — Full-featured TUI framework, active development, immediate mode rendering
2. **cursive** — Callback-based TUI, simpler for basic menus
3. **dialoguer** — Simple prompts only, not full TUI
4. **crossterm + manual rendering** — Low-level, maximum control

### Decision
**ratatui with crossterm backend** — Industry standard for Rust TUIs, excellent documentation, supports colors and Unicode symbols.

### Consequences
- **Positive:** Rich UI capabilities, active community, good docs
- **Negative:** Some complexity for a simple use case, but provides room to grow

---

## ADR-003: Data Sources

### Context
We need reliable, free data sources for weather, tides, and water quality in Vancouver.

### Options Considered

**Weather:**
- Open-Meteo (free, no API key) ✓
- OpenWeatherMap (free tier, requires API key)
- Environment Canada (official, complex API)

**Tides:**
- Fisheries and Oceans Canada (official, free) ✓
- WorldTides (paid)
- NOAA (US-focused)

**Water Quality:**
- Vancouver Coastal Health / City of Vancouver (official, free) ✓

### Decision
- **Weather:** Open-Meteo — free, no API key, good accuracy
- **Tides:** Fisheries and Oceans Canada — official Canadian source
- **Water Quality:** City of Vancouver Open Data — official E. coli testing results

### Consequences
- **Positive:** All free, no API keys required, official Canadian sources for tides/water
- **Negative:** May need to parse HTML or non-standard formats for some sources

---

## ADR-004: Caching Strategy

### Context
Water quality data doesn't change frequently, and we want resilience against API failures.

### Decision
- **Weather:** No caching (always fetch fresh)
- **Tides:** Cache for 24 hours (tide tables are predictable)
- **Water Quality:** Cache for 24 hours (tests done periodically)
- **Cache location:** `~/.cache/vanbeach/` or XDG-compliant path

### Consequences
- **Positive:** Faster subsequent loads, resilience to API outages
- **Negative:** Potential for stale data (mitigated by showing cache age)
