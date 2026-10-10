#!/bin/sh
# Removes KUBUSTACK from ~/.local.
#   curl -fsSL https://raw.githubusercontent.com/nixondebantu/kubustack/main/scripts/uninstall.sh | sh
set -eu

APP_DIR="$HOME/.local/share/applications"

rm -f "$HOME/.local/bin/kubustack" \
    "$HOME/.local/share/icons/hicolor/scalable/apps/kubustack.svg" \
    "$APP_DIR/kubustack.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi

printf '%s\n' "KUBUSTACK has been removed."
