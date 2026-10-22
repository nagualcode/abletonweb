#!/usr/bin/env bash
# Builds "Live Remote.app" (Apple Silicon) into app/target/.
#   scripts/build-app.sh            build the app
#   scripts/build-app.sh --install  also copy it to /Applications
#   scripts/build-app.sh --zip      also create the release zip in dist/
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NAME="Live Remote"
TARGET="aarch64-apple-darwin"
BUNDLE="$ROOT/app/target/$NAME.app"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/app/Cargo.toml" | head -1)"

# The web app and the Remote Script are embedded into the binary.
(cd "$ROOT/web" && npm ci --no-audit --no-fund --loglevel=error && npm run build --silent)
# Keep local paths (home folder, checkout) out of the distributed binary.
(cd "$ROOT/app" && MACOSX_DEPLOYMENT_TARGET=12.0 \
  RUSTFLAGS="--remap-path-prefix=$ROOT=. --remap-path-prefix=$HOME=~" \
  cargo build --release --target "$TARGET")

rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
cp "$ROOT/app/target/$TARGET/release/live-remote" "$BUNDLE/Contents/MacOS/live-remote"

# Icon: render the web app's SVG icon at the sizes macOS wants.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
qlmanage -t -s 1024 -o "$WORK" "$ROOT/web/public/icon.svg" >/dev/null 2>&1
ICONSET="$WORK/icon.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size "$WORK/icon.svg.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) "$WORK/icon.svg.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$BUNDLE/Contents/Resources/AppIcon.icns"

cat > "$BUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$NAME</string>
  <key>CFBundleDisplayName</key><string>$NAME</string>
  <key>CFBundleIdentifier</key><string>com.abletonweb.liveremote</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleExecutable</key><string>live-remote</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

# Ad-hoc signature (no Apple Developer ID): fine for locally installed apps.
codesign --force --deep --sign - "$BUNDLE" 2>/dev/null
echo "Built: $BUNDLE"

for arg in "$@"; do
  case "$arg" in
    --install)
      rm -rf "/Applications/$NAME.app"
      ditto "$BUNDLE" "/Applications/$NAME.app"
      echo "Installed: /Applications/$NAME.app"
      ;;
    --zip)
      STAGE="$WORK/stage"
      mkdir -p "$STAGE" "$ROOT/dist"
      ditto "$BUNDLE" "$STAGE/$NAME.app"
      cp "$ROOT/scripts/README.txt" "$STAGE/README.txt"
      ZIP="$ROOT/dist/LiveRemote-v$VERSION-macos-arm64.zip"
      rm -f "$ZIP"
      (cd "$STAGE" && ditto -c -k --sequesterRsrc . "$ZIP")
      echo "Zip: $ZIP"
      ;;
  esac
done
