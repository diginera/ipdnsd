#!/bin/sh
#
# ipdnsd installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/diginera/ipdnsd/main/scripts/install.sh | sh
#
# This script:
# 1. Detects your OS and architecture
# 2. Downloads the latest release binary
# 3. Installs it to /usr/local/bin (or ~/bin if no sudo)
# 4. Creates a default config file
#

set -e

REPO="diginera/ipdnsd"
BINARY_NAME="ipdnsd"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    printf "${BLUE}[INFO]${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}[OK]${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
    exit 1
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS="linux";;
        Darwin*)    OS="macos";;
        CYGWIN*|MINGW*|MSYS*) OS="windows";;
        *)          error "Unsupported operating system: $(uname -s)";;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   ARCH="amd64";;
        aarch64|arm64)  ARCH="arm64";;
        *)              error "Unsupported architecture: $(uname -m)";;
    esac
}

# Get the latest release version from GitHub
get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one of them."
    fi

    if [ -z "$VERSION" ]; then
        error "Could not determine latest version. Check https://github.com/${REPO}/releases"
    fi
}

# Download the binary
download_binary() {
    ASSET_NAME="${BINARY_NAME}-${OS}-${ARCH}"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"

    info "Downloading ${BINARY_NAME} ${VERSION} for ${OS}/${ARCH}..."

    TEMP_DIR=$(mktemp -d)
    TEMP_FILE="${TEMP_DIR}/${BINARY_NAME}"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_FILE" || error "Download failed. Check if release exists for your platform."
    else
        wget -q "$DOWNLOAD_URL" -O "$TEMP_FILE" || error "Download failed. Check if release exists for your platform."
    fi

    chmod +x "$TEMP_FILE"
}

# Install the binary
install_binary() {
    # Try /usr/local/bin first (requires sudo), then fall back to ~/bin
    if [ -w /usr/local/bin ]; then
        INSTALL_DIR="/usr/local/bin"
    elif command -v sudo >/dev/null 2>&1; then
        INSTALL_DIR="/usr/local/bin"
        USE_SUDO=1
    else
        INSTALL_DIR="$HOME/bin"
        mkdir -p "$INSTALL_DIR"
    fi

    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."

    if [ "$USE_SUDO" = "1" ]; then
        sudo mv "$TEMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        mv "$TEMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    rm -rf "$TEMP_DIR"

    # Check if INSTALL_DIR is in PATH
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            warn "${INSTALL_DIR} is not in your PATH"
            warn "Add this to your shell profile: export PATH=\"\$PATH:${INSTALL_DIR}\""
            ;;
    esac

    success "Installed ${BINARY_NAME} to ${INSTALL_DIR}"
}

# Create default config file
create_config() {
    CONFIG_DIR="/etc/ipdnsd"
    CONFIG_FILE="${CONFIG_DIR}/config.toml"

    if [ -f "$CONFIG_FILE" ]; then
        info "Config file already exists at ${CONFIG_FILE}"
        return
    fi

    $SUDO mkdir -p "$CONFIG_DIR"

    $SUDO tee "$CONFIG_FILE" > /dev/null << 'EOF'
# ipdnsd Configuration
# See https://github.com/diginera/ipdnsd for documentation

[daemon]
interval_seconds = 300  # Check every 5 minutes
log_level = "info"

# Example DNS entry - update with your domain
# Uncomment and modify the following:

# [[dns_entries]]
# provider = "godaddy"
# domain = "example.com"
# record_name = "@"
# record_type = "A"
# ip_source = "external"
EOF

    success "Created config file at ${CONFIG_FILE}"
}

# Print next steps
print_next_steps() {
    echo ""
    echo "=============================================="
    echo "  ipdnsd installed successfully!"
    echo "=============================================="
    echo ""
    echo "Next steps:"
    echo ""
    echo "1. Store your DNS provider API credentials:"
    echo "   ${BINARY_NAME} set-key godaddy"
    echo ""
    echo "2. Edit your config file:"
    echo "   /etc/ipdnsd/config.toml"
    echo ""
    echo "3. Test your configuration:"
    echo "   ${BINARY_NAME} check"
    echo ""
    echo "4. Run the daemon:"
    echo "   ${BINARY_NAME} daemon"
    echo ""
    echo "5. (Optional) Install as a system service:"
    echo "   sudo ${BINARY_NAME} install"
    echo ""
    echo "For more information: https://github.com/${REPO}"
    echo ""
}

# Main installation flow
main() {
    echo ""
    info "Installing ipdnsd - IP to DNS Updater"
    echo ""

    detect_os
    detect_arch
    get_latest_version
    download_binary
    install_binary
    create_config
    print_next_steps
}

main
