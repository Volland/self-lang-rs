#!/bin/sh
# Builds "Self World.app" and the installer disk image SelfWorld-macOS.dmg.
#
#   ./scripts/package-mac.sh            # universal app (Apple silicon + Intel)
#   ./scripts/package-mac.sh --native   # only for the kind of Mac this runs on (faster)
#
# Results:  target/Self World.app   and   target/dist/SelfWorld-macOS.dmg
# Needs Rust (https://rustup.rs); the universal build also needs
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin
set -e
cd "$(dirname "$0")/.."

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
APP="target/Self World.app"
DIST="target/dist"
DMG="$DIST/SelfWorld-macOS.dmg"
BIN="target/self-world-macos"

if [ "$1" = "--native" ]; then
  cargo build --release --bin self-world
  cp target/release/self-world "$BIN"
else
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$t" >/dev/null 2>&1 || true
    cargo build --release --bin self-world --target "$t"
  done
  lipo -create -output "$BIN" \
    target/aarch64-apple-darwin/release/self-world \
    target/x86_64-apple-darwin/release/self-world
fi
lipo -info "$BIN"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/self-world"
cp assets/icons/SelfWorld.icns "$APP/Contents/Resources/SelfWorld.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Self World</string>
  <key>CFBundleDisplayName</key><string>Self World</string>
  <key>CFBundleIdentifier</key><string>org.selfworld.desktop</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleExecutable</key><string>self-world</string>
  <key>CFBundleIconFile</key><string>SelfWorld</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
</dict></plist>
PLIST
# Not signed with a developer certificate: an ad-hoc signature is still
# needed for Apple silicon Macs to run it.
codesign --force --deep -s - "$APP"
echo "Made $APP ($VERSION)"

# The disk image: the app next to a shortcut to /Applications.
mkdir -p "$DIST"
STAGE="target/dmg-stage"
rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
# small apps written in Self, next to the app:
#   "/Applications/Self World.app/Contents/MacOS/self-world" --app Examples/todo.self
[ -d examples ] && cp -R examples "$STAGE/Examples"
hdiutil create -volname "Self World $VERSION" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$DMG" >/dev/null
rm -rf "$STAGE"
echo "Made $DMG"
