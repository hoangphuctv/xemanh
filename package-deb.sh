#!/usr/bin/env bash
# Build a .deb installer for XemAnh (Debian / Ubuntu / Mint / Pop!_OS, …).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "[ERROR] Missing tool: $1 (hint: $2)" >&2
    exit 1
  }
}

if [[ -f "$HOME/.cargo/env" ]]; then
  source "$HOME/.cargo/env"
fi

need cargo "rustup / cargo (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)"
need dpkg-deb "dpkg (apt install dpkg)"
need fakeroot "fakeroot (apt install fakeroot)"
need strip "binutils (apt install binutils)"

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1 | tr -d '\r')"
ARCH="$(dpkg --print-architecture)"
PKG_NAME="xemanh"
DEB_NAME="${PKG_NAME}_${VERSION}_${ARCH}.deb"
OUT_DIR="${ROOT}/installer"
STAGE="${ROOT}/target/deb-root"

echo "============================================"
echo "  XemAnh - Build Release + Create .deb"
echo "============================================"
echo

echo "[1/4] Building release..."
cargo build --release
BIN="${ROOT}/target/release/xemanh"
if [[ ! -f "$BIN" ]]; then
  echo "[ERROR] $BIN not found." >&2
  exit 1
fi
echo "[OK] Release build succeeded."
echo

echo "[2/4] Staging package tree..."
rm -rf "$STAGE"
install -d \
  "$STAGE/DEBIAN" \
  "$STAGE/usr/bin" \
  "$STAGE/usr/share/applications" \
  "$STAGE/usr/share/icons/hicolor/256x256/apps" \
  "$STAGE/usr/share/doc/${PKG_NAME}"

install -m 755 "$BIN" "$STAGE/usr/bin/xemanh"
strip --strip-unneeded "$STAGE/usr/bin/xemanh"

install -m 644 packaging/linux/xemanh.desktop \
  "$STAGE/usr/share/applications/xemanh.desktop"
install -m 644 assets/xemanh.png \
  "$STAGE/usr/share/icons/hicolor/256x256/apps/xemanh.png"

{
  echo "XemAnh ${VERSION}"
  echo
  echo "Lightweight image viewer."
  echo
  echo "Homepage: https://github.com/hoangphuctv/xemanh"
} >"$STAGE/usr/share/doc/${PKG_NAME}/README"
gzip -9n -f "$STAGE/usr/share/doc/${PKG_NAME}/README"
chmod 644 "$STAGE/usr/share/doc/${PKG_NAME}/README.gz"

# Installed-Size is in KiB (exclude DEBIAN control dir)
INSTALLED_SIZE="$(du -sk --exclude=DEBIAN "$STAGE" | awk '{print $1}')"

cat >"$STAGE/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${VERSION}
Section: graphics
Priority: optional
Architecture: ${ARCH}
Installed-Size: ${INSTALLED_SIZE}
Depends: libx11-6, libgl1 | libgl1-mesa-glx, libxi6, libxkbcommon0
Recommends: libegl1, libwayland-client0, libwayland-egl1, libdecor-0-0
Maintainer: hoangphuctv <hoangphuctv@users.noreply.github.com>
Description: Lightweight image viewer
 XemAnh opens images quickly with a window sized to the picture.
 Supports JPEG, PNG, BMP, GIF and TGA, folder browsing, zoom/pan,
 EXIF orientation, and fullscreen viewing.
EOF

cat >"$STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi
exit 0
EOF

cat >"$STAGE/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi
exit 0
EOF

chmod 755 "$STAGE/DEBIAN/postinst" "$STAGE/DEBIAN/postrm"
chmod 755 "$STAGE/DEBIAN"
chmod 644 "$STAGE/DEBIAN"/*
chmod 755 "$STAGE/DEBIAN/postinst" "$STAGE/DEBIAN/postrm"
echo "[OK] Staged under target/deb-root/"
echo

echo "[3/4] Building .deb with fakeroot + dpkg-deb..."
mkdir -p "$OUT_DIR"
TMP_BUILD_DIR="$(mktemp -d /tmp/deb-build.XXXXXX)"
cp -a "$STAGE/." "$TMP_BUILD_DIR/"
chmod -R 755 "$TMP_BUILD_DIR/DEBIAN"
chmod 644 "$TMP_BUILD_DIR/DEBIAN"/* 2>/dev/null || true
chmod 755 "$TMP_BUILD_DIR/DEBIAN/postinst" "$TMP_BUILD_DIR/DEBIAN/postrm" 2>/dev/null || true
fakeroot dpkg-deb --root-owner-group --build "$TMP_BUILD_DIR" "${OUT_DIR}/${DEB_NAME}"
rm -rf "$TMP_BUILD_DIR"
echo "[OK] Created ${OUT_DIR}/${DEB_NAME}"
echo

echo "[4/4] Done!"
echo "Install with:"
echo "  sudo apt install ./${OUT_DIR##*/}/${DEB_NAME}"
echo "  # or: sudo dpkg -i ${OUT_DIR}/${DEB_NAME}"
echo
