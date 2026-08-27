#!/usr/bin/env bash
# Kitsu Installer for Linux and macOS
# https://github.com/jmaxdev/Kitsu

set -euo pipefail

REPO="jmaxdev/Kitsu"
INSTALL_DIR="${KITSU_INSTALL_DIR:-$HOME/.kitsu/bin}"
REQUESTED_VERSION="${KITSU_VERSION:-latest}"

bold="$(tput bold 2>/dev/null || echo '')"
green="$(tput setaf 2 2>/dev/null || echo '')"
yellow="$(tput setaf 3 2>/dev/null || echo '')"
red="$(tput setaf 1 2>/dev/null || echo '')"
reset="$(tput sgr0 2>/dev/null || echo '')"

info() {
    echo -e "${bold}${green}==>${reset} ${bold}$*${reset}"
}

warn() {
    echo -e "${bold}${yellow}warning:${reset} $*"
}

error() {
    echo -e "${bold}${red}error:${reset} $*" >&2
    exit 1
}

# 1. Detect OS
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
    linux*)  PLATFORM="linux" ;;
    darwin*) PLATFORM="macos" ;;
    *) error "Unsupported operating system: $OS. Kitsu installer supports Linux and macOS." ;;
esac

# 2. Detect Architecture
ARCH="$(uname -m | tr '[:upper:]' '[:lower:]')"
case "$ARCH" in
    x86_64|amd64) TARGET_ARCH="x86_64" ;;
    aarch64|arm64) TARGET_ARCH="aarch64" ;;
    *) error "Unsupported architecture: $ARCH" ;;
esac

# 3. Determine target asset name pattern
if [ "$PLATFORM" = "linux" ]; then
    if [ "$TARGET_ARCH" = "x86_64" ]; then
        ASSET_PATTERN="x86_64-unknown-linux-gnu.tar.gz"
    else
        error "Pre-built Linux binaries currently only support x86_64. You can build from source using 'cargo build --release'."
    fi
elif [ "$PLATFORM" = "macos" ]; then
    if [ "$TARGET_ARCH" = "aarch64" ]; then
        ASSET_PATTERN="aarch64-apple-darwin.tar.gz"
    else
        ASSET_PATTERN="x86_64-apple-darwin.tar.gz"
    fi
fi

info "Detected platform: ${PLATFORM} (${TARGET_ARCH})"

# 4. Fetch release information from GitHub API
TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'kitsu-install')"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

HTTP_CLIENT=""
if command -v curl >/dev/null 2>&1; then
    HTTP_CLIENT="curl"
elif command -v wget >/dev/null 2>&1; then
    HTTP_CLIENT="wget"
else
    error "Neither curl nor wget was found. Please install either curl or wget to continue."
fi

fetch_url() {
    local url="$1"
    if [ "$HTTP_CLIENT" = "curl" ]; then
        curl -sSL -H "User-Agent: kitsu-installer" "$url"
    else
        wget -qO- --user-agent="kitsu-installer" "$url"
    fi
}

download_file() {
    local url="$1"
    local dest="$2"
    if [ "$HTTP_CLIENT" = "curl" ]; then
        curl -fSL --progress-bar -H "User-Agent: kitsu-installer" "$url" -o "$dest"
    else
        wget --progress=bar:force --user-agent="kitsu-installer" "$url" -O "$dest"
    fi
}

info "Fetching release information from GitHub..."

RELEASES_JSON="$TMP_DIR/releases.json"
fetch_url "https://api.github.com/repos/${REPO}/releases" > "$RELEASES_JSON"

if [ ! -s "$RELEASES_JSON" ]; then
    error "Failed to retrieve releases from https://api.github.com/repos/${REPO}/releases"
fi

# Extract asset download URL
DOWNLOAD_URL=""
RELEASE_TAG=""

if [ "$REQUESTED_VERSION" = "latest" ]; then
    # Parse the first release containing our target asset
    DOWNLOAD_URL=$(grep -oE "https://github.com/${REPO}/releases/download/[^\"]*${ASSET_PATTERN}" "$RELEASES_JSON" | head -n 1 || true)
else
    TAG_QUERY="$REQUESTED_VERSION"
    [[ "$TAG_QUERY" != v* ]] && TAG_QUERY="v$TAG_QUERY"
    DOWNLOAD_URL=$(grep -oE "https://github.com/${REPO}/releases/download/${TAG_QUERY}/[^\"]*${ASSET_PATTERN}" "$RELEASES_JSON" | head -n 1 || true)
fi

if [ -z "$DOWNLOAD_URL" ]; then
    error "Could not find a compatible binary package (${ASSET_PATTERN}) for version: ${REQUESTED_VERSION}"
fi

RELEASE_TAG=$(echo "$DOWNLOAD_URL" | sed -E 's|.*/releases/download/([^/]+)/.*|\1|')
info "Downloading Kitsu ${RELEASE_TAG}..."

ARCHIVE_FILE="$TMP_DIR/kitsu.tar.gz"
download_file "$DOWNLOAD_URL" "$ARCHIVE_FILE"

# 5. Extract binary
info "Extracting archive..."
tar -xzf "$ARCHIVE_FILE" -C "$TMP_DIR"

BINARY_PATH=""
if [ -f "$TMP_DIR/kitsu" ]; then
    BINARY_PATH="$TMP_DIR/kitsu"
else
    BINARY_PATH=$(find "$TMP_DIR" -type f -name kitsu | head -n 1 || true)
fi

if [ -z "$BINARY_PATH" ] || [ ! -f "$BINARY_PATH" ]; then
    error "Could not locate 'kitsu' binary inside the downloaded archive."
fi

# 6. Install binary
mkdir -p "$INSTALL_DIR"
cp -f "$BINARY_PATH" "$INSTALL_DIR/kitsu"
chmod +x "$INSTALL_DIR/kitsu"

info "Installed Kitsu binary to ${bold}${INSTALL_DIR}/kitsu${reset}"

# 7. Configure PATH if necessary
update_shell_profile() {
    local shell_rc="$1"
    if [ -f "$shell_rc" ]; then
        if ! grep -q "$INSTALL_DIR" "$shell_rc"; then
            echo "" >> "$shell_rc"
            echo "# Kitsu version control system" >> "$shell_rc"
            echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$shell_rc"
            info "Added $INSTALL_DIR to ${shell_rc}"
        fi
    fi
}

case "${SHELL:-}" in
    */zsh)
        update_shell_profile "$HOME/.zshrc"
        ;;
    */bash)
        if [ -f "$HOME/.bashrc" ]; then
            update_shell_profile "$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            update_shell_profile "$HOME/.bash_profile"
        fi
        ;;
    */fish)
        if command -v fish >/dev/null 2>&1; then
            fish -c "set -U fish_user_paths $INSTALL_DIR \$fish_user_paths" 2>/dev/null || true
            info "Added $INSTALL_DIR to Fish user paths"
        fi
        ;;
    *)
        update_shell_profile "$HOME/.profile"
        ;;
esac

echo ""
echo -e "${bold}${green}Kitsu ${RELEASE_TAG} was successfully installed!${reset}"
echo ""
echo "To get started, restart your terminal or run:"
echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
echo ""
echo "Verify installation by running:"
echo "  kitsu --version"
echo "  kitsu ignite"
echo ""
