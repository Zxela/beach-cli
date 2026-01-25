# UI Wireframes: Best Time to Go Feature

## Screen Flow

```
                 ┌─────────────┐
                 │   Loading   │
                 └──────┬──────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│                    Beach List                            │
│                                                         │
│  • Shows contextual hints based on time of day          │
│  • Press 'p' to go to Plan Trip                        │
│  • Press Enter to go to Beach Detail                   │
└───────────┬───────────────────────────┬─────────────────┘
            │                           │
            │ Enter                     │ p
            ▼                           ▼
┌─────────────────────┐       ┌─────────────────────┐
│   Beach Detail      │◄──────│     Plan Trip       │
│                     │ Enter │                     │
│ • Activity chips    │       │ • Heatmap grid      │
│ • Best window       │       │ • Activity selector │
│ • Press 'p' to plan │───────►│ • Best recommendation│
└─────────────────────┘   p   └─────────────────────┘
```

## Beach List with Contextual Hints

```
╭─ Vancouver Beaches ─────────────────────────────────────────────────╮
│                                                                     │
│  ▸ Kitsilano Beach         22°C  🌤️   🟢   Good for swimming      │
│    English Bay Beach       21°C  ☀️   🟢   Peak sun hours          │
│    Jericho Beach           20°C  🌤️   🟡   Quieter than Kits      │
│    Spanish Banks East      19°C  ☁️   🟢   Windy - good sailing    │
│    Spanish Banks West      19°C  ☁️   🟢   Windy - good sailing    │
│    Locarno Beach           19°C  ☁️   🟢   Moderate crowds         │
│    Wreck Beach             18°C  🌤️   ⚪   Sunset in 2h 15m        │
│    Second Beach            21°C  ☀️   🟢   Crowded now             │
│    Third Beach             20°C  ☀️   🟢   Good for swimming       │
│    Sunset Beach            21°C  ☀️   🟢   Peak sun hours          │
│    Trout Lake              23°C  ☀️   🟢   Warm & calm             │
│    New Brighton            20°C  🌤️   🟢   Quiet morning spot      │
│                                                                     │
│  ↑/↓ Navigate  Enter Select  p Plan Trip  q Quit                   │
╰─────────────────────────────────────────────────────────────────────╯
```

### Contextual Hint Logic

| Time of Day | Possible Hints |
|-------------|----------------|
| 6-9am | "Quiet morning spot", "Good for peace", "Warming up" |
| 9am-12pm | "Good for swimming", "Heating up", "Moderate crowds" |
| 12pm-4pm | "Peak sun hours", "Crowded now", "Best swimming" |
| 4pm-7pm | "Sunset in Xh Ym", "Cooling down", "Evening calm" |
| 7pm+ | "Sunset soon", "Evening walk", "Winding down" |

Additional factors:
- High wind → "Windy - good sailing"
- Water advisory → "Water advisory"
- Weekend afternoon → "Crowded now"

## Beach Detail with Activity Filter

```
╭─ Kitsilano Beach ───────────────────────────────────────────────────╮
│                                                                     │
│  Activity: [●Swimming] [○Sunbathing] [○Sailing] [○Sunset] [○Peace] │
│                                                                     │
│  BEST WINDOW TODAY                                                  │
│  ─────────────────                                                  │
│  🥇 11:00 AM - 1:00 PM    Score: 92/100                            │
│     Warm (24°C), safe water, mid-tide, moderate crowds             │
│                                                                     │
│  🥈 2:00 PM - 4:00 PM     Score: 85/100                            │
│     Hot (26°C), safe water, high tide, busier                      │
│                                                                     │
│  🥉 9:00 AM - 11:00 AM    Score: 78/100                            │
│     Warming (21°C), safe water, rising tide, quiet                 │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│  CURRENT CONDITIONS                                                 │
│  ──────────────────                                                 │
│  🌤️  22°C (feels 24°C)        ↗ Rising                             │
│  💨 12 km/h W                  High: 2:34 PM (4.3m)                 │
│  💧 65% humidity               Low:  8:45 PM (1.2m)                 │
│  ☀️  UV Index: 6 (High)                                            │
│  🌅 5:42 AM  🌇 9:12 PM                                            │
│                                                                     │
│  WATER QUALITY                                                      │
│  ─────────────                                                      │
│  🟢 Safe to swim                                                   │
│  Last tested: Jan 24, 2026                                         │
│                                                                     │
│  1-5 Activity  p Plan Trip  ← Back  q Quit                         │
╰─────────────────────────────────────────────────────────────────────╯
```

### Activity Chips Interaction

- Numbers `1-5` toggle the corresponding activity
- Selected activity shown with `●`, others with `○`
- Selecting an activity updates the "Best Window" section
- Activity selection persists when navigating to Plan Trip

### Best Window Section

Shows top 3 time slots for the selected activity:
- Medal emoji indicates rank
- Time range (grouped into 2-hour windows)
- Numeric score out of 100
- Key factors explaining the score

## Plan Trip Screen - Heatmap Grid

```
╭─ Plan Your Trip ────────────────────────────────────────────────────╮
│                                                                     │
│  Activity: [●Swimming] [○Sunbathing] [○Sailing] [○Sunset] [○Peace] │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│              6am   8am  10am  12pm   2pm   4pm   6pm   8pm          │
│            ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐        │
│ Kitsilano  │ ░░  │ ▒▒  │ ▓▓  │[██] │ ██  │ ▓▓  │ ▒▒  │ ░░  │        │
│            ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤        │
│ English B  │ ░░  │ ▒▒  │ ▓▓  │ ██  │ ▓▓  │ ▓▓  │ ▒▒  │ ░░  │        │
│            ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤        │
│ Jericho    │ ░░  │ ▓▓  │ ██  │ ██  │ ▓▓  │ ▒▒  │ ▒▒  │ ░░  │        │
│            ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤        │
│ Spanish E  │ ░░  │ ▒▒  │ ▓▓  │ ▓▓  │ ▓▓  │ ▒▒  │ ░░  │ ░░  │        │
│            ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤        │
│ Spanish W  │ ░░  │ ▒▒  │ ▓▓  │ ▓▓  │ ▒▒  │ ▒▒  │ ░░  │ ░░  │        │
│            ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤        │
│ Locarno    │ ░░  │ ▒▒  │ ▓▓  │ ▓▓  │ ▓▓  │ ▒▒  │ ░░  │ ░░  │        │
│            └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘        │
│                                                                     │
│  Legend: ██ Excellent (80+)  ▓▓ Good (60-79)  ▒▒ Fair (40-59)      │
│          ░░ Poor (<40)       [ ] Cursor                             │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│  BEST RECOMMENDATION                                                │
│  ───────────────────                                                │
│  Jericho Beach @ 10:00 AM                                          │
│  Score: 94/100 - Warm (23°C), safe water, rising tide, uncrowded   │
│                                                                     │
│  SELECTED: Kitsilano @ 12:00 PM                                    │
│  Score: 88/100 - Hot (25°C), safe water, high tide, moderate crowd │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│  ←/h →/l Hours  ↑/k ↓/j Beaches  1-5 Activity  Enter Go  Esc Back  │
╰─────────────────────────────────────────────────────────────────────╯
```

### Grid Cells

Each cell represents a 2-hour window:
- Block character indicates score range
- Color overlay (when terminal supports it):
  - Green: 80-100
  - Light Green: 60-79
  - Yellow: 40-59
  - Light Red: 20-39
  - Red: 0-19

### Cursor

- Highlighted with `[ ]` brackets
- Moves with arrow keys or `h/j/k/l`
- Pressing Enter navigates to Beach Detail for selected beach

### Best Recommendation

Always shows the single best beach + time combination:
- Updated when activity changes
- Shows score and key factors

### Selected Cell

Shows details for wherever the cursor is:
- Beach name and time
- Score and factors
- Allows comparison with best recommendation

## Alternative Layouts

### Compact List View (If Grid Too Wide)

```
╭─ Plan: Swimming ────────────────────────────────╮
│                                                 │
│  BEST TIMES TODAY                              │
│  ───────────────                               │
│  1. Jericho @ 10am      94/100  ████████████   │
│  2. Kitsilano @ 12pm    88/100  ██████████     │
│  3. English Bay @ 11am  85/100  █████████      │
│  4. Third Beach @ 1pm   82/100  █████████      │
│  5. Locarno @ 10am      78/100  ████████       │
│                                                 │
│  1-5 Activity  Enter Details  Esc Back         │
╰─────────────────────────────────────────────────╯
```

### Single Beach Timeline (From Detail Screen)

```
╭─ Kitsilano: Swimming ───────────────────────────╮
│                                                 │
│  HOURLY FORECAST                               │
│  ───────────────                               │
│   6am  ░░░░░░░░  32  Too cold, quiet           │
│   8am  ▒▒▒▒▒▒▒▒  54  Warming up                │
│  10am  ▓▓▓▓▓▓▓▓  72  Good                      │
│ >12pm  ████████  88  Excellent ◄ BEST          │
│   2pm  ████████  85  Great                     │
│   4pm  ▓▓▓▓▓▓▓▓  71  Crowded                   │
│   6pm  ▒▒▒▒▒▒▒▒  58  Cooling                   │
│   8pm  ░░░░░░░░  35  Getting dark              │
│                                                 │
│  ↑/↓ Scroll  1-5 Activity  Esc Back            │
╰─────────────────────────────────────────────────╯
```

## Color Palette

| Score Range | Block | Background | Foreground |
|-------------|-------|------------|------------|
| 80-100 | `██` | Green | White |
| 60-79 | `▓▓` | Light Green | Black |
| 40-59 | `▒▒` | Yellow | Black |
| 20-39 | `░░` | Light Red | Black |
| 0-19 | `  ` | Red | White |

## Responsive Considerations

- Beach names truncated if terminal too narrow (e.g., "Spanish Banks East" → "Spanish E")
- Hour columns can be reduced (show every 2 hours instead of every hour)
- On very narrow terminals, fall back to compact list view
- Minimum usable width: 60 characters

## Keyboard Shortcuts Summary

| Screen | Key | Action |
|--------|-----|--------|
| All | `q` | Quit |
| All | `Esc` | Back / Close |
| List | `p` | Open Plan Trip |
| List | `↑/k`, `↓/j` | Navigate beaches |
| List | `Enter` | Open beach detail |
| Detail | `1-5` | Select activity |
| Detail | `p` | Open Plan Trip |
| Plan | `1-5` | Select activity |
| Plan | `←/h`, `→/l` | Navigate hours |
| Plan | `↑/k`, `↓/j` | Navigate beaches |
| Plan | `Enter` | Go to selected beach detail |
