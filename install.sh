#!/usr/bin/env bash
# Live Remote installer: downloads the latest release, installs the app in
# /Applications, then the LiveWeb Remote Script and the bridge service.
#
#   curl -fsSL https://raw.githubusercontent.com/nagualcode/abletonweb/main/install.sh | bash
set -euo pipefail

REPO="nagualcode/abletonweb"
NAME="Live Remote"
APP="/Applications/$NAME.app"

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "Live Remote requires macOS on Apple Silicon (arm64)." >&2
  exit 1
fi

echo "→ Finding the latest release..."
URL="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep -o '"browser_download_url": *"[^"]*macos-arm64\.zip"' | head -1 | cut -d'"' -f4)"
if [[ -z "$URL" ]]; then
  echo "Could not find a release zip at github.com/$REPO/releases" >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
echo "→ Downloading $(basename "$URL")..."
curl -fsSL "$URL" -o "$TMP/live-remote.zip"
ditto -x -k "$TMP/live-remote.zip" "$TMP/unzipped"

# Quit a running copy before replacing it (the bridge service keeps running).
osascript -e "quit app \"$NAME\"" >/dev/null 2>&1 || true
echo "→ Installing $APP..."
rm -rf "$APP"
ditto "$TMP/unzipped/$NAME.app" "$APP"
# Downloads made by curl are not quarantined, but clear it just in case.
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true

echo "→ Installing the Remote Script and the bridge service..."
"$APP/Contents/MacOS/live-remote" install

cat <<EOF

✓ Live Remote installed.

Next steps:
  1. Restart Ableton Live → Settings → Link, Tempo & MIDI → Control Surface: LiveWeb
     (Input and Output: None).
  2. Open "Live Remote" (Applications) to see the QR code, or run:
       "$APP/Contents/MacOS/live-remote" url

EOF
open "$APP"
