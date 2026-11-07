#!/bin/sh
# Builds the Linux packages of Self World into target/dist/:
#   SelfWorld-Linux-x86_64.tar.gz    the program, its icon and launcher entry
#   SelfWorld-Linux-amd64.deb        for Debian, Ubuntu and friends (cargo-deb)
#   SelfWorld-Linux-x86_64.AppImage  one file that runs on most distributions
#
#   ./scripts/package-linux.sh
#
# Needs Rust (https://rustup.rs), curl, and on Debian/Ubuntu the packages
#   build-essential libxkbcommon-dev libgl1-mesa-dev libwayland-dev libx11-dev
# cargo-deb and appimagetool are fetched if they are missing.
set -e
cd "$(dirname "$0")/.."

ARCH="$(uname -m)"            # x86_64 or aarch64
case "$ARCH" in
  x86_64) DEB_ARCH=amd64 ;;
  aarch64) DEB_ARCH=arm64 ;;
  *) DEB_ARCH="$ARCH" ;;
esac
DIST="target/dist"
mkdir -p "$DIST"

cargo build --release --bin self-world
strip target/release/self-world 2>/dev/null || true

# ---- tar.gz
TGZ_DIR="target/SelfWorld-Linux-$ARCH"
rm -rf "$TGZ_DIR"
mkdir -p "$TGZ_DIR"
cp target/release/self-world "$TGZ_DIR/"
cp assets/linux/self-world.desktop "$TGZ_DIR/"
cp assets/icons/icon-256.png "$TGZ_DIR/self-world.png"
cp README.md LICENSE "$TGZ_DIR/"
# small apps written in Self:  ./self-world --app examples/todo.self
[ -d examples ] && cp -R examples "$TGZ_DIR/examples"
tar -C target -czf "$DIST/SelfWorld-Linux-$ARCH.tar.gz" "SelfWorld-Linux-$ARCH"
echo "Made $DIST/SelfWorld-Linux-$ARCH.tar.gz"

# ---- .deb (the file list is in Cargo.toml under [package.metadata.deb])
if ! command -v cargo-deb >/dev/null 2>&1; then
  cargo install cargo-deb --locked
fi
cargo deb --no-build --no-strip --output "$DIST/SelfWorld-Linux-$DEB_ARCH.deb"
echo "Made $DIST/SelfWorld-Linux-$DEB_ARCH.deb"

# ---- AppImage
APPDIR="target/SelfWorld.AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
cp target/release/self-world "$APPDIR/usr/bin/"
cp assets/linux/self-world.desktop "$APPDIR/usr/share/applications/"
cp assets/linux/self-world.desktop "$APPDIR/self-world.desktop"
cp assets/icons/icon-256.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/self-world.png"
cp assets/icons/icon-256.png "$APPDIR/self-world.png"
if [ -d examples ]; then
  mkdir -p "$APPDIR/usr/share/self-world"
  cp -R examples "$APPDIR/usr/share/self-world/examples"
fi
cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/self-world" "$@"
APPRUN
chmod +x "$APPDIR/AppRun"

TOOL="target/appimagetool-$ARCH.AppImage"
if [ ! -x "$TOOL" ]; then
  curl -fsSL -o "$TOOL" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$ARCH.AppImage"
  chmod +x "$TOOL"
fi
# runs without FUSE too (containers, CI)
APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$ARCH" "$TOOL" --no-appstream "$APPDIR" "$DIST/SelfWorld-Linux-$ARCH.AppImage"
echo "Made $DIST/SelfWorld-Linux-$ARCH.AppImage"
