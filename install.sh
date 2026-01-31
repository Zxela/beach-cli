#!/bin/sh
# Vanbeach CLI Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/Zxela/beach-cli/main/install.sh | sh

set -e

REPO="Zxela/beach-cli"
BINARY_NAME="vanbeach"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    printf "${GREEN}[INFO]${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)
            case "$ARCH" in
                x86_64)
                    PLATFORM="x86_64-unknown-linux-gnu"
                    ;;
                aarch64|arm64)
                    PLATFORM="aarch64-unknown-linux-gnu"
                    ;;
                *)
                    error "Unsupported architecture: $ARCH"
                    ;;
            esac
            ;;
        Darwin)
            case "$ARCH" in
                x86_64)
                    PLATFORM="x86_64-apple-darwin"
                    ;;
                arm64|aarch64)
                    PLATFORM="aarch64-apple-darwin"
                    ;;
                *)
                    error "Unsupported architecture: $ARCH"
                    ;;
            esac
            ;;
        *)
            error "Unsupported operating system: $OS"
            ;;
    esac

    info "Detected platform: $PLATFORM"
}

# Get latest release version from GitHub
get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
    else
        error "Neither curl nor wget found. Please install one of them."
    fi

    if [ -z "$VERSION" ]; then
        error "Could not determine latest version. Check https://github.com/$REPO/releases"
    fi

    info "Latest version: $VERSION"
}

# Download and install binary
install_binary() {
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$BINARY_NAME-$PLATFORM.tar.gz"

    info "Downloading from: $DOWNLOAD_URL"

    # Create temp directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT

    # Download
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/$BINARY_NAME.tar.gz"
    else
        wget -q "$DOWNLOAD_URL" -O "$TMP_DIR/$BINARY_NAME.tar.gz"
    fi

    # Extract
    tar -xzf "$TMP_DIR/$BINARY_NAME.tar.gz" -C "$TMP_DIR"

    # Install
    mkdir -p "$INSTALL_DIR"
    mv "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    info "Installed $BINARY_NAME to $INSTALL_DIR/$BINARY_NAME"
}

# Check if install directory is in PATH
check_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            info "$BINARY_NAME is ready to use!"
            ;;
        *)
            warn "$INSTALL_DIR is not in your PATH"
            echo ""
            echo "Add it to your shell profile:"
            echo ""
            echo "  # For bash (~/.bashrc):"
            echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
            echo ""
            echo "  # For zsh (~/.zshrc):"
            echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
            echo ""
            echo "Then restart your shell or run: source ~/.bashrc"
            ;;
    esac
}

main() {
    echo ""
    echo "  Vanbeach CLI Installer"
    echo "  ======================"
    echo ""

    detect_platform
    get_latest_version
    install_binary
    check_path

    echo ""
    info "Installation complete! Run '$BINARY_NAME' to start."
    echo ""
}

main
