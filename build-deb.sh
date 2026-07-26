#!/bin/bash
set -euo pipefail

# Build .deb package for LibreSync
PKG_VERSION="${1:-0.1.0}"
ARCH="amd64"
PKG_NAME="libresync_${PKG_VERSION}_${ARCH}.deb"

echo "==> Building LibreSync v${PKG_VERSION}..."

# Build release binaries
cargo build --release --bin libresync-core
cargo build --release --bin get_refresh_token

# Prepare package directory
PKG_DIR="build/deb"
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN"
mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/usr/share/applications"
mkdir -p "$PKG_DIR/usr/share/icons/hicolor/48x48/apps"
mkdir -p "$PKG_DIR/usr/share/icons/hicolor/128x128/apps"
mkdir -p "$PKG_DIR/usr/share/doc/libresync"

# Copy and strip binaries
install -s -m 755 target/release/libresync-core "$PKG_DIR/usr/bin/"
install -s -m 755 target/release/get_refresh_token "$PKG_DIR/usr/bin/"

# Control file
cat > "$PKG_DIR/DEBIAN/control" << EOF
Package: libresync
Version: ${PKG_VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Depends: libgtk-3-0 (>= 3.24), libwebkit2gtk-4.1-0 (>= 2.40), libgdk-pixbuf-2.0-0 (>= 2.42), libjavascriptcoregtk-4.1-0, libsoup-3.0-0 (>= 3.0)
Maintainer: Emerson Costa <elrcosta@gmail.com>
Description: Google Drive sync client for Linux
 Native Google Drive synchronization client with
 system tray integration, desktop notifications,
 and real-time file watching via inotify.
Homepage: https://github.com/elrcosta-lab/libresync
EOF

# Desktop entry
cat > "$PKG_DIR/usr/share/applications/libresync.desktop" << 'EOF'
[Desktop Entry]
Type=Application
Name=LibreSync
Comment=Google Drive sync client
Exec=libresync-core --tray
Icon=libresync
Terminal=false
Categories=Utility;Network;FileTransfer;
StartupNotify=false
X-GNOME-UsesNotifications=true
EOF

# Icon
cp resources/icons/icon.png "$PKG_DIR/usr/share/icons/hicolor/128x128/apps/libresync.png"
cp resources/icons/icon.png "$PKG_DIR/usr/share/icons/hicolor/48x48/apps/libresync.png"

# Changelog
cat > /tmp/libresync-changelog << CHLOG
libresync (${PKG_VERSION}) stable; urgency=medium

  * Initial release
  * Google Drive OAuth2 authentication with PKCE
  * Bidirectional file synchronization
  * System tray with status icons
  * Desktop notifications
  * Real-time file watching (inotify)

 -- Emerson Costa <elrcosta@gmail.com>  Sun, 26 Jul 2026 20:00:00 -0300
CHLOG
gzip -9 -c /tmp/libresync-changelog > "$PKG_DIR/usr/share/doc/libresync/changelog.gz"
rm /tmp/libresync-changelog

# Build .deb
fakeroot dpkg-deb --build "$PKG_DIR" "$PKG_NAME"

echo "==> Created: ${PKG_NAME} ($(du -h ${PKG_NAME} | cut -f1))"
