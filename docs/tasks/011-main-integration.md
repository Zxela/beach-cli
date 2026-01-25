---
id: "011"
title: "Wire up main.rs with event loop"
status: pending
depends_on: ["009", "010"]
test_file: null
no_test_reason: "integration - verified by cargo run and manual testing"
---

# 011: Wire up main.rs with event loop

## Objective

Connect all components in main.rs to create the working CLI application with event loop.

## Acceptance Criteria

- [ ] Initialize terminal with crossterm backend
- [ ] Set up panic hook to restore terminal on crash
- [ ] Create App instance and trigger initial data load
- [ ] Main event loop: render UI, poll for keyboard events, update state
- [ ] Route to correct screen based on AppState (Loading, BeachList, BeachDetail)
- [ ] Handle keyboard events and dispatch to App
- [ ] Clean terminal restoration on exit
- [ ] Async runtime setup with tokio::main

## Technical Notes

Standard ratatui pattern:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new();
    app.load_all_data().await;

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
```

## Verification

```bash
cargo run
# Should show loading, then beach list
# Arrow keys navigate, Enter selects, Esc goes back, q quits
```
