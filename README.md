# vanbeach

A terminal UI application to view Vancouver beach conditions including weather, tides, and water quality.

## Features

- View conditions for 12 Vancouver beaches
- Real-time weather data
- Tide information
- Water quality status
- Vim-style navigation (j/k) and arrow keys

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/vanbeach`.

## Usage

```bash
cargo run
```

### Key Bindings

| Key | Action |
|-----|--------|
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `Enter` | View beach details |
| `Esc` | Go back / Quit |
| `q` | Quit |

## Dependencies

- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal backend
- [tokio](https://tokio.rs/) - Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client

## License

MIT
