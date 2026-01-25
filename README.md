# Vanbeach

A terminal UI application to view Vancouver beach conditions including weather, tides, and water quality.

## Features

- View conditions for 12 Vancouver beaches
- Real-time weather data
- Tide information
- Water quality status
- Vim-style navigation (j/k) and arrow keys

## Installation

### Quick Install (Linux/macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/Zxela/beach-cli/main/install.sh | sh
```

Or with wget:

```sh
wget -qO- https://raw.githubusercontent.com/Zxela/beach-cli/main/install.sh | sh
```

### From Source

Requires [Rust](https://rustup.rs/) toolchain.

```sh
git clone https://github.com/Zxela/beach-cli.git
cd beach-cli
make install
```

### Manual Download

Download the latest release from [GitHub Releases](https://github.com/Zxela/beach-cli/releases), extract, and place in your PATH.

## Usage

```sh
vanbeach                      # Launch the TUI
vanbeach --beach "Kitsilano"  # Start with a specific beach selected
vanbeach --help               # Show all options
```

### Key Bindings

| Key | Action |
|-----|--------|
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `Enter` | View beach details |
| `Esc` | Go back / Quit |
| `q` | Quit |

## Building

```sh
make build          # Debug build
make build-release  # Release build
make test           # Run tests
make check          # Run all checks (fmt, lint, test)
make help           # Show all available commands
```

## Releasing

Build and package for all platforms:

```sh
make release
```

Upload to GitHub (requires [gh CLI](https://cli.github.com/)):

```sh
make release-upload
```

Or create a draft release:

```sh
make release-draft
```

### Supported Platforms

- Linux x86_64 (glibc and musl)
- Linux aarch64
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)

## Dependencies

- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal backend
- [tokio](https://tokio.rs/) - Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client

## License

MIT
