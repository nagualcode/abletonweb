LIVE REMOTE — control Ableton Live from your phone or tablet
https://github.com/nagualcode/abletonweb

EASIEST: instead of this zip, run in Terminal
  curl -fsSL https://raw.githubusercontent.com/nagualcode/abletonweb/main/install.sh | bash

INSTALLING FROM THIS ZIP
  1. Drag "Live Remote.app" to the Applications folder.
  2. The app is not notarized by Apple, so macOS blocks its first launch.
     Run once in Terminal:
       xattr -dr com.apple.quarantine "/Applications/Live Remote.app"
     (or: try to open it, then System Settings → Privacy & Security → Open Anyway).
  3. Open Live Remote and click "Install".
  4. Restart Ableton Live → Settings → Link, Tempo & MIDI → Control Surface: LiveWeb
     (Input and Output: None).
  5. Scan the QR code shown in the app with your phone (same Wi-Fi).

WITHOUT THE WINDOW (Terminal only)
  "/Applications/Live Remote.app/Contents/MacOS/live-remote" install
  "/Applications/Live Remote.app/Contents/MacOS/live-remote" url
  "/Applications/Live Remote.app/Contents/MacOS/live-remote" help
