#!/bin/sh
# Installs or updates KUBUSTACK in ~/.local (no sudo needed).
#   curl -fsSL https://raw.githubusercontent.com/nixondebantu/kubustack/main/scripts/install.sh | sh
# Pin a version with KUBUSTACK_VERSION=vX.Y.Z (put it before `sh`).
set -eu

REPO="nixondebantu/kubustack"
ASSET="kubustack-x86_64-linux.tar.gz"
BIN_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
APP_DIR="$HOME/.local/share/applications"

say() { printf '%s\n' "$*"; }
warn() { printf 'Warning: %s\n' "$*" >&2; }
die() { printf 'Error: %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = Linux ] || die "KUBUSTACK runs on Linux only."
arch=$(uname -m)
[ "$arch" = x86_64 ] || die "Release builds are x86_64 only, but this machine is $arch. See the README to build from source."

if [ -n "${KUBUSTACK_VERSION:-}" ]; then
    BASE="https://github.com/$REPO/releases/download/$KUBUSTACK_VERSION"
else
    BASE="https://github.com/$REPO/releases/latest/download"
fi

if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -q -O "$2" "$1"; }
else
    die "Neither curl nor wget is installed. Install one of them and run this again."
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

say "Downloading KUBUSTACK ${KUBUSTACK_VERSION:-(latest release)}..."
fetch "$BASE/$ASSET" "$TMP/$ASSET" || die "Download failed: $BASE/$ASSET"
fetch "$BASE/SHA256SUMS" "$TMP/SHA256SUMS" || die "Download failed: $BASE/SHA256SUMS"

if command -v sha256sum >/dev/null 2>&1; then
    (cd "$TMP" && sha256sum -c SHA256SUMS >/dev/null) ||
        die "Checksum mismatch: the download is damaged. Please try again."
else
    warn "sha256sum not found, skipping checksum verification."
fi

mkdir -p "$TMP/x" "$BIN_DIR" "$ICON_DIR" "$APP_DIR"
tar -xzf "$TMP/$ASSET" -C "$TMP/x"

# Copy then rename, so updating works even while KUBUSTACK is running.
cp "$TMP/x/kubustack" "$BIN_DIR/.kubustack.new"
chmod 755 "$BIN_DIR/.kubustack.new"
mv -f "$BIN_DIR/.kubustack.new" "$BIN_DIR/kubustack"
cp "$TMP/x/kubustack.svg" "$ICON_DIR/kubustack.svg"

# Absolute Exec path: app launchers don't always have ~/.local/bin on PATH.
cat >"$APP_DIR/kubustack.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=KUBUSTACK
GenericName=Docker Manager
Comment=Lightweight viewer and manager for Docker containers and images
Exec=$BIN_DIR/kubustack
Icon=kubustack
Terminal=false
Categories=Development;Utility;
Keywords=docker;container;image;podman;
StartupWMClass=kubustack
DESKTOP

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi

case ":$PATH:" in
*":$BIN_DIR:"*) ;;
*) warn "$BIN_DIR is not on your PATH. The app menu entry works anyway; to run 'kubustack' from a terminal, add it to PATH in your shell profile." ;;
esac

if grep -q '^docker:' /etc/group 2>/dev/null && ! id -nG | tr ' ' '\n' | grep -qx docker; then
    warn "You are not in the 'docker' group, so KUBUSTACK can't reach Docker yet. Fix: sudo usermod -aG docker \$USER, then log out and back in."
fi

version=$("$BIN_DIR/kubustack" --version 2>/dev/null || echo "kubustack")
say "Installed $version to $BIN_DIR/kubustack"
say "Launch it from your app menu, or run: $BIN_DIR/kubustack"
