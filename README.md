# Vanbeach

A terminal UI application to view Vancouver beach conditions including weather, tides, and water quality.

## Features

- View conditions for 12 Vancouver beaches
- Real-time weather data with temperature, wind, UV index
- Tide information with visual chart
- Water quality status from City of Vancouver
- Activity scoring for Swimming, Sunbathing, Sailing, Sunset viewing, and Peace & quiet
- Plan Trip view to compare beaches across time slots
- Vim-style navigation (j/k/h/l) and arrow keys

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
vanbeach                        # Launch the TUI
vanbeach --plan                 # Start in Plan Trip view
vanbeach --plan --activity swim # Plan Trip with Swimming selected
vanbeach --help                 # Show all options
```

### Key Bindings

#### Beach List
| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` | View beach details |
| `p` | Open Plan Trip view |
| `1-5` | Select activity (1=Swim, 2=Sun, 3=Sail, 4=Sunset, 5=Peace) |
| `r` | Refresh data |
| `?` | Show help |
| `q` / `Esc` | Quit |

#### Beach Detail
| Key | Action |
|-----|--------|
| `1-5` | Select activity for scoring |
| `r` | Refresh data |
| `?` | Show help |
| `Esc` | Go back to list |
| `q` | Quit |

#### Plan Trip
| Key | Action |
|-----|--------|
| `h` / `←` | Previous hour |
| `l` / `→` | Next hour |
| `j` / `↓` | Next beach |
| `k` / `↑` | Previous beach |
| `1-5` | Select activity |
| `Enter` | View beach details |
| `Esc` | Go back to list |
| `q` | Quit |

## Building

```sh
make build          # Debug build
make build-release  # Release build
make test           # Run tests
make check          # Run all checks (fmt, clippy, test)
make help           # Show all available commands
```

## Releasing

Releases are automated via GitHub Actions. To create a new release:

```sh
make release-patch  # Bump 0.1.0 -> 0.1.1, run checks, commit, push, tag
make release-minor  # Bump 0.1.0 -> 0.2.0
make release-major  # Bump 0.1.0 -> 1.0.0
```

This will:
1. Run all checks (fmt, clippy, tests)
2. Bump the version in Cargo.toml
3. Commit and push to main
4. Create and push a git tag
5. GitHub Actions builds binaries for all platforms and creates a release

### Supported Platforms

- Linux x86_64 (glibc and musl)
- Linux aarch64
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)

## License

MIT
