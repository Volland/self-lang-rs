#!/bin/sh
# Builds "Self World.app" (for the kind of Mac this runs on) into target/.
set -e
cd "$(dirname "$0")/.."
cargo build --release --bin self-world
APP="target/Self World.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/self-world "$APP/Contents/MacOS/self-world"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Self World</string>
  <key>CFBundleDisplayName</key><string>Self World</string>
  <key>CFBundleIdentifier</key><string>org.selfworld.desktop</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleExecutable</key><string>self-world</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
</dict></plist>
PLIST
echo "Made $APP"
