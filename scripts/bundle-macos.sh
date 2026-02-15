#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

APP_NAME="DD Merge"
BINARY_NAME="dd_merge"
BUNDLE_DIR="$PROJECT_DIR/target/release/$APP_NAME.app"

SVG_PATH="$PROJECT_DIR/assets/icon.svg"
ICONSET_DIR="$PROJECT_DIR/assets/icon.iconset"
ICNS_PATH="$PROJECT_DIR/assets/icon.icns"
PLIST_PATH="$PROJECT_DIR/assets/Info.plist"

# --- Step 1: Generate .icns from SVG ---
echo "==> Generating app icon..."

if ! command -v rsvg-convert &>/dev/null; then
    echo "Error: rsvg-convert not found. Install with: brew install librsvg"
    exit 1
fi

rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

# macOS .iconset requires these exact sizes and filenames
sizes=(16 32 128 256 512)
for size in "${sizes[@]}"; do
    rsvg-convert -w "$size" -h "$size" "$SVG_PATH" -o "$ICONSET_DIR/icon_${size}x${size}.png"
    double=$((size * 2))
    rsvg-convert -w "$double" -h "$double" "$SVG_PATH" -o "$ICONSET_DIR/icon_${size}x${size}@2x.png"
done

iconutil -c icns "$ICONSET_DIR" -o "$ICNS_PATH"
rm -rf "$ICONSET_DIR"
echo "    Icon generated at $ICNS_PATH"

# --- Step 2: Build release binary ---
echo "==> Building release binary..."
cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml" -p "$BINARY_NAME"

BINARY_PATH="$PROJECT_DIR/target/release/$BINARY_NAME"
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Error: binary not found at $BINARY_PATH"
    exit 1
fi

# --- Step 3: Assemble .app bundle ---
echo "==> Assembling $APP_NAME.app..."

rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

cp "$PROJECT_DIR/target/release/$BINARY_NAME" "$BUNDLE_DIR/Contents/MacOS/$BINARY_NAME"
cp "$ICNS_PATH" "$BUNDLE_DIR/Contents/Resources/icon.icns"
cp "$PLIST_PATH" "$BUNDLE_DIR/Contents/Info.plist"

echo "==> Done! App bundle created at:"
echo "    $BUNDLE_DIR"
echo ""
echo "    Run with: open \"$BUNDLE_DIR\""
