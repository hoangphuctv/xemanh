#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "[ERROR] This script must be run on macOS."
  exit 1
fi

if [[ -f "$HOME/.cargo/env" ]]; then
  source "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "[ERROR] Missing tool: cargo"
  echo "Install Rust from https://rustup.rs/"
  exit 1
fi

IS_RELEASE=false
if [[ "${1:-}" == "release" ]]; then
  IS_RELEASE=true
fi

if [[ "$IS_RELEASE" == "true" ]]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "[ERROR] Missing tool: gh (GitHub CLI)"
    exit 1
  fi

  echo "[INFO] Release mode enabled. Fetching latest tags..."
  git fetch --tags

  LATEST_TAG="$(git tag --sort=-v:refname | head -n 1)"
  if [[ -z "$LATEST_TAG" ]]; then
    echo "[ERROR] No git tags found in repository."
    exit 1
  fi

  echo "[INFO] Latest tag found: $LATEST_TAG. Checking out..."
  git checkout "$LATEST_TAG"
fi

echo "[INFO] Building XemAnh for macOS..."
echo "[INFO] Architecture: $(uname -m)"

cargo build --release

BINARY="$ROOT/target/release/xemanh"

if [[ ! -f "$BINARY" ]]; then
  echo "[ERROR] Build completed but binary was not found:"
  echo "        $BINARY"
  exit 1
fi

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
ARCH="$(uname -m)"
APP_NAME="XemAnh"
APP_DIR="$ROOT/target/release/${APP_NAME}.app"
DMG_NAME="${APP_NAME}-${VERSION}-macos-${ARCH}.dmg"
DMG_PATH="$ROOT/target/release/$DMG_NAME"
DMG_ROOT="$ROOT/target/release/dmg-root"

if [[ -z "$VERSION" ]]; then
  echo "[ERROR] Could not determine package version from Cargo.toml."
  exit 1
fi

echo "[INFO] Creating macOS application bundle..."

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

ICON_SOURCE="$ROOT/assets/xemanh.png"
ICONSET_DIR="$ROOT/target/release/xemanh.iconset"
ICON_FILE="$APP_DIR/Contents/Resources/xemanh.icns"

if [[ ! -f "$ICON_SOURCE" ]]; then
  echo "[ERROR] macOS icon source was not found:"
  echo "        $ICON_SOURCE"
  exit 1
fi

if ! command -v sips >/dev/null 2>&1; then
  echo "[ERROR] Missing macOS tool: sips"
  exit 1
fi

if ! command -v iconutil >/dev/null 2>&1; then
  echo "[ERROR] Missing macOS tool: iconutil"
  exit 1
fi

echo "[INFO] Creating macOS app icon..."
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

sips -z 16 16 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null
sips -z 32 32 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null
sips -z 64 64 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null
sips -z 256 256 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null
sips -z 512 512 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_512x512.png" >/dev/null
sips -z 1024 1024 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null

iconutil -c icns "$ICONSET_DIR" -o "$ICON_FILE"
rm -rf "$ICONSET_DIR"

cp "$BINARY" "$APP_DIR/Contents/MacOS/xemanh"

cat > "$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>XemAnh</string>
    <key>CFBundleExecutable</key>
    <string>xemanh</string>
    <key>CFBundleIdentifier</key>
    <string>com.xemanh.app</string>
    <key>CFBundleName</key>
    <string>XemAnh</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>xemanh</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Image</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.image</string>
                <string>public.jpeg</string>
                <string>public.png</string>
                <string>com.compuserve.gif</string>
                <string>com.microsoft.bmp</string>
                <string>com.truevision.tga-image</string>
            </array>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>jpg</string>
                <string>jpeg</string>
                <string>jpe</string>
                <string>jfif</string>
                <string>png</string>
                <string>bmp</string>
                <string>dib</string>
                <string>gif</string>
                <string>tga</string>
            </array>
        </dict>
    </array>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
</dict>
</plist>
EOF

chmod +x "$APP_DIR/Contents/MacOS/xemanh"

echo "[INFO] Preparing DMG layout..."
rm -rf "$DMG_ROOT"
mkdir -p "$DMG_ROOT"
cp -R "$APP_DIR" "$DMG_ROOT/"
ln -s /Applications "$DMG_ROOT/Applications"

echo "[INFO] Creating DMG: $DMG_NAME..."
rm -f "$DMG_PATH"
hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$DMG_ROOT" \
  -ov \
  -format UDZO \
  "$DMG_PATH"

rm -rf "$DMG_ROOT"

echo "[SUCCESS] macOS release build completed."
echo "[INFO] Binary: $BINARY"
echo "[INFO] App: $APP_DIR"
echo "[INFO] DMG: $DMG_PATH"

if [[ "$IS_RELEASE" == "true" ]]; then
  echo "[INFO] Checking latest GitHub release tag..."
  GH_RELEASE_TAG="$(gh release list --limit 1 | awk '{print $1}')"

  if [[ -z "$GH_RELEASE_TAG" ]]; then
    echo "[ERROR] Could not find any release on GitHub via 'gh release list'."
    exit 1
  fi

  if [[ "$GH_RELEASE_TAG" != "$LATEST_TAG" ]]; then
    echo "[ERROR] Tag mismatch! Checked out git tag '$LATEST_TAG', but latest GitHub release is '$GH_RELEASE_TAG'."
    exit 1
  fi

  echo "[INFO] Tag matches ($LATEST_TAG). Uploading $DMG_NAME to GitHub release..."
  gh release upload "$LATEST_TAG" "$DMG_PATH" --clobber
  echo "[SUCCESS] DMG uploaded successfully to release $LATEST_TAG."
fi
