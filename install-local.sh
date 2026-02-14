#!/bin/bash
# Install Epic Asset Manager for local development/testing
# Installs to ~/.local following XDG conventions

set -e

APP_ID="io.github.achetagames.epic_asset_manager"
PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"

# XDG directories
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
SYMBOLIC_ICON_DIR="$HOME/.local/share/icons/hicolor/symbolic/apps"
SCHEMA_DIR="$HOME/.local/share/glib-2.0/schemas"
DATA_DIR="$HOME/.local/share/epic_asset_manager"
BIN_DIR="$HOME/.local/bin"

# Create directories
mkdir -p "$DESKTOP_DIR"
mkdir -p "$ICON_DIR"
mkdir -p "$SYMBOLIC_ICON_DIR"
mkdir -p "$SCHEMA_DIR"
mkdir -p "$DATA_DIR"
mkdir -p "$BIN_DIR"

# Build if needed
if [ ! -f "$PROJECT_DIR/target/debug/epic_asset_manager" ]; then
    echo "Building application..."
    cd "$PROJECT_DIR"
    cargo build
fi

# Compile gresource if needed
if [ ! -f "$PROJECT_DIR/data/resources/resources.gresource" ] || \
   [ "$PROJECT_DIR/data/resources/resources.gresource.xml" -nt "$PROJECT_DIR/data/resources/resources.gresource" ]; then
    echo "Compiling resources..."
    cd "$PROJECT_DIR/data/resources"
    glib-compile-resources --sourcedir=. resources.gresource.xml --target=resources.gresource
fi

# Install gresource file
echo "Installing resources..."
cp "$PROJECT_DIR/data/resources/resources.gresource" "$DATA_DIR/"

# Generate and install GSettings schema
echo "Installing GSettings schema..."
sed -e "s/@app-id@/$APP_ID/g" -e "s/@gettext-package@/epic_asset_manager/g" \
    "$PROJECT_DIR/data/$APP_ID.gschema.xml.in" > "$SCHEMA_DIR/$APP_ID.gschema.xml"

# Compile schemas
echo "Compiling schemas..."
glib-compile-schemas "$SCHEMA_DIR"

# Create wrapper script that sets up environment
echo "Installing launcher script..."
cat > "$BIN_DIR/epic_asset_manager" << 'LAUNCHER'
#!/bin/bash
# Epic Asset Manager launcher

# Set schema directory for GSettings
if [ -z "$GSETTINGS_SCHEMA_DIR" ]; then
    export GSETTINGS_SCHEMA_DIR="$HOME/.local/share/glib-2.0/schemas"
else
    export GSETTINGS_SCHEMA_DIR="$HOME/.local/share/glib-2.0/schemas:$GSETTINGS_SCHEMA_DIR"
fi

# Launch the application
exec "$HOME/.local/share/epic_asset_manager/epic_asset_manager" "$@"
LAUNCHER
chmod +x "$BIN_DIR/epic_asset_manager"

# Copy the actual binary
cp "$PROJECT_DIR/target/debug/epic_asset_manager" "$DATA_DIR/epic_asset_manager"
chmod +x "$DATA_DIR/epic_asset_manager"

# Generate desktop file
echo "Installing desktop file..."
cat > "$DESKTOP_DIR/$APP_ID.desktop" << EOF
[Desktop Entry]
Name=Epic Asset Manager
Comment=Manage assets from Epic Games Store
Type=Application
Exec=$BIN_DIR/epic_asset_manager %u
Terminal=false
Categories=Game;Utility;
Keywords=Epic;Unreal;Assets;
Icon=$APP_ID
StartupNotify=true
MimeType=x-scheme-handler/com.epicgames.launcher
EOF

# Install icons
echo "Installing icons..."
cp "$PROJECT_DIR/data/icons/$APP_ID.svg" "$ICON_DIR/"
cp "$PROJECT_DIR/data/icons/$APP_ID-symbolic.svg" "$SYMBOLIC_ICON_DIR/"

# Update caches
echo "Updating caches..."
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo ""
echo "========================================="
echo "Installation complete!"
echo "========================================="
echo ""
echo "Installed to:"
echo "  Binary:   $BIN_DIR/epic_asset_manager"
echo "  Data:     $DATA_DIR/"
echo "  Desktop:  $DESKTOP_DIR/$APP_ID.desktop"
echo "  Schema:   $SCHEMA_DIR/$APP_ID.gschema.xml"
echo ""
echo "You can now launch Epic Asset Manager from:"
echo "  - Application menu"
echo "  - Terminal: epic_asset_manager"
echo ""
echo "Make sure $BIN_DIR is in your PATH."
