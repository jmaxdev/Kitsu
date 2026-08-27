#!/bin/sh
# Kitsu Installer for Linux and macOS (POSIX sh compatible)
# Supports specific versions and prereleases
# https://github.com/jmaxdev/Kitsu

set -eu

REPO="jmaxdev/Kitsu"
INSTALL_DIR="${KITSU_INSTALL_DIR:-$HOME/.kitsu/bin}"
REQUESTED_VERSION="${KITSU_VERSION:-latest}"

# Check environment variable overrides for prerelease
if [ "${KITSU_PRERELEASE:-false}" = "true" ] || [ "${KITSU_PRE:-false}" = "true" ]; then
    REQUESTED_VERSION="pre"
fi

# Parse CLI arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --version|-v)
            if [ -n "${2:-}" ]; then
                REQUESTED_VERSION="$2"
                shift 2
            else
                shift 1
            fi
            ;;
        --prerelease|--pre)
            REQUESTED_VERSION="pre"
            shift 1
            ;;
        --dir|-d)
            if [ -n "${2:-}" ]; then
                INSTALL_DIR="$2"
                shift 2
            else
                shift 1
            fi
            ;;
        --help|-h)
            echo "Kitsu Installer for Linux and macOS"
            echo ""
            echo "Usage:"
            echo "  install.sh [options]"
            echo "  curl -fsSL https://raw.githubusercontent.com/jmaxdev/Kitsu/dev/install.sh | sh -s -- [options]"
            echo ""
            echo "Options:"
            echo "  -v, --version <tag>    Install a specific version (e.g. v0.0.3-alpha or 0.0.2-alpha)"
            echo "      --pre, --prerelease Install the latest prerelease (alpha/beta/rc)"
            echo "  -d, --dir <path>       Custom installation directory (default: ~/.kitsu/bin)"
            echo "  -h, --help             Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  KITSU_VERSION=<tag>    Target version or 'pre' / 'latest'"
            echo "  KITSU_PRERELEASE=true  Fetch latest prerelease"
            echo "  KITSU_INSTALL_DIR=<dir> Custom target binary directory"
            exit 0
            ;;
        *)
            if [ "$REQUESTED_VERSION" = "latest" ] || [ -z "$REQUESTED_VERSION" ]; then
                REQUESTED_VERSION="$1"
            fi
            shift 1
            ;;
    esac
done

bold="$(tput bold 2>/dev/null || true)"
green="$(tput setaf 2 2>/dev/null || true)"
yellow="$(tput setaf 3 2>/dev/null || true)"
red="$(tput setaf 1 2>/dev/null || true)"
reset="$(tput sgr0 2>/dev/null || true)"

info() {
    printf "%s%s==>%s %s%s%s\n" "${bold}" "${green}" "${reset}" "${bold}" "$*" "${reset}"
}

warn() {
    printf "%s%swarning:%s %s\n" "${bold}" "${yellow}" "${reset}" "$*"
}

error() {
    printf "%s%serror:%s %s\n" "${bold}" "${red}" "${reset}" "$*" >&2
    exit 1
}

# Detect operating system
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
    linux*)  PLATFORM="linux" ;;
    darwin*) PLATFORM="macos" ;;
    *) error "Unsupported operating system: $OS. Kitsu installer supports Linux and macOS." ;;
esac

# Detect machine architecture
ARCH="$(uname -m | tr '[:upper:]' '[:lower:]')"
case "$ARCH" in
    x86_64|amd64) TARGET_ARCH="x86_64" ;;
    aarch64|arm64) TARGET_ARCH="aarch64" ;;
    *) error "Unsupported architecture: $ARCH" ;;
esac

# Target asset pattern resolution
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

# Fetch release metadata from GitHub API
TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'kitsu-install')"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

HTTP_CLIENT=""
if command -v curl >/dev/null 2>&1; then
    HTTP_CLIENT="curl"
elif command -v wget >/dev/null 2>&1; then
    HTTP_CLIENT="wget"
else
    error "Neither curl nor wget was found. Please install either curl or wget to continue."
fi

fetch_url() {
    url="$1"
    if [ "$HTTP_CLIENT" = "curl" ]; then
        curl -sSL -H "User-Agent: kitsu-installer" "$url"
    else
        wget -qO- --user-agent="kitsu-installer" "$url"
    fi
}

download_file() {
    url="$1"
    dest="$2"
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

# Extract asset download URL and tag using Python if available, or shell fallback
DOWNLOAD_URL=""
RELEASE_TAG=""

PYTHON_CMD=""
if command -v python3 >/dev/null 2>&1; then
    PYTHON_CMD="python3"
elif command -v python >/dev/null 2>&1; then
    PYTHON_CMD="python"
fi

if [ -n "$PYTHON_CMD" ]; then
    PARSER_RESULT=$("$PYTHON_CMD" -c '
import sys, json

pattern = sys.argv[1]
req = sys.argv[2]

try:
    with open(sys.argv[3], "r", encoding="utf-8") as f:
        releases = json.load(f)
except Exception:
    sys.exit(1)

def find_asset(r):
    for a in r.get("assets", []):
        if pattern in a.get("name", ""):
            return a.get("browser_download_url"), r.get("tag_name")
    return None, None

selected_url, selected_tag = None, None

if req in ("pre", "prerelease"):
    for r in releases:
        if r.get("draft"): continue
        is_pre = r.get("prerelease") or any(k in r.get("tag_name", "").lower() for k in ("alpha", "beta", "rc", "dev", "-"))
        if is_pre:
            u, t = find_asset(r)
            if u:
                selected_url, selected_tag = u, t
                break
elif req != "latest":
    clean_req = req.lstrip("v")
    for r in releases:
        tag = r.get("tag_name", "")
        if tag == req or tag == "v" + req or tag.lstrip("v") == clean_req:
            u, t = find_asset(r)
            if u:
                selected_url, selected_tag = u, t
                break
else:
    # Prefer stable releases, fallback to available builds
    for r in releases:
        if r.get("draft") or r.get("prerelease"): continue
        u, t = find_asset(r)
        if u:
            selected_url, selected_tag = u, t
            break
    if not selected_url:
        for r in releases:
            if r.get("draft"): continue
            u, t = find_asset(r)
            if u:
                selected_url, selected_tag = u, t
                break

if selected_url and selected_tag:
    print(f"{selected_url} {selected_tag}")
' "$ASSET_PATTERN" "$REQUESTED_VERSION" "$RELEASES_JSON" || true)

    if [ -n "$PARSER_RESULT" ]; then
        DOWNLOAD_URL=$(echo "$PARSER_RESULT" | awk '{print $1}')
        RELEASE_TAG=$(echo "$PARSER_RESULT" | awk '{print $2}')
    fi
fi

# Shell fallback if Python is not present or parser returned empty
if [ -z "$DOWNLOAD_URL" ]; then
    if [ "$REQUESTED_VERSION" = "latest" ] || [ "$REQUESTED_VERSION" = "pre" ] || [ "$REQUESTED_VERSION" = "prerelease" ]; then
        DOWNLOAD_URL=$(grep -oE "https://github.com/${REPO}/releases/download/[^\"]*${ASSET_PATTERN}" "$RELEASES_JSON" | head -n 1 || true)
    else
        TAG_QUERY="$REQUESTED_VERSION"
        case "$TAG_QUERY" in
            v*) ;;
            *) TAG_QUERY="v$TAG_QUERY" ;;
        esac
        DOWNLOAD_URL=$(grep -oE "https://github.com/${REPO}/releases/download/${TAG_QUERY}/[^\"]*${ASSET_PATTERN}" "$RELEASES_JSON" | head -n 1 || true)
    fi
    if [ -n "$DOWNLOAD_URL" ]; then
        RELEASE_TAG=$(echo "$DOWNLOAD_URL" | sed -E 's|.*/releases/download/([^/]+)/.*|\1|')
    fi
fi

if [ -z "$DOWNLOAD_URL" ]; then
    error "Could not find a compatible binary package (${ASSET_PATTERN}) for requested version/channel: ${REQUESTED_VERSION}"
fi

info "Downloading Kitsu ${RELEASE_TAG}..."

ARCHIVE_FILE="$TMP_DIR/kitsu.tar.gz"
download_file "$DOWNLOAD_URL" "$ARCHIVE_FILE"

# Extract binary
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

# Install binary
mkdir -p "$INSTALL_DIR"
cp -f "$BINARY_PATH" "$INSTALL_DIR/kitsu"
chmod +x "$INSTALL_DIR/kitsu"

info "Installed Kitsu binary to ${bold}${INSTALL_DIR}/kitsu${reset}"

# Configure system PATH
update_shell_profile() {
    shell_rc="$1"
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
printf "%s%sKitsu %s was successfully installed!%s\n\n" "${bold}" "${green}" "${RELEASE_TAG}" "${reset}"
echo "To get started, restart your terminal or run:"
echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
echo ""
echo "Verify installation by running:"
echo "  kitsu --version"
echo "  kitsu ignite"
echo ""
