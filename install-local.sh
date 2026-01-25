#!/bin/bash
# Install Epic Asset Manager desktop entry and icon for local development
# This allows the app to appear in the application menu without a full Meson install

set -e

APP_ID="io.github.achetagames.epic_asset_manager"
PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Directories
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
SYMBOLIC_ICON_DIR="$HOME/.local/share/icons/hicolor/symbolic/apps"
BIN_DIR="$HOME/.local/bin"

# Create directories
mkdir -p "$DESKTOP_DIR"
mkdir -p "$ICON_DIR"
mkdir -p "$SYMBOLIC_ICON_DIR"
mkdir -p "$BIN_DIR"

# Generate desktop file from template
echo "Installing desktop file..."
sed "s/@icon@/$APP_ID/" "$PROJECT_DIR/data/$APP_ID.desktop.in.in" > "$DESKTOP_DIR/$APP_ID.desktop"

# Update exec path to point to built binary
sed -i "s|Exec=epic_asset_manager|Exec=$PROJECT_DIR/target/debug/epic_asset_manager|" "$DESKTOP_DIR/$APP_ID.desktop"

# Install icons
echo "Installing icons..."
cp "$PROJECT_DIR/data/icons/$APP_ID.svg" "$ICON_DIR/"
cp "$PROJECT_DIR/data/icons/$APP_ID-symbolic.svg" "$SYMBOLIC_ICON_DIR/"

# Update icon cache
echo "Updating icon cache..."
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

# Update desktop database
echo "Updating desktop database..."
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo ""
echo "Installation complete!"
echo "Epic Asset Manager should now appear in your application menu."
echo ""
echo "Desktop file: $DESKTOP_DIR/$APP_ID.desktop"
echo "Icon: $ICON_DIR/$APP_ID.svg"
echo ""
echo "Note: You may need to log out and log back in for the icon to appear in some desktop environments."
