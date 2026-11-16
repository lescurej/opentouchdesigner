#!/usr/bin/env bash
# Build OpenTouchDesigner.app and a .dmg from it.
#
#   scripts/package-macos.sh [version]
#
# The app is *unsigned*. Signing needs a paid Apple Developer account, and
# pretending otherwise would be worse than saying so: Gatekeeper will refuse
# the first launch, and the README tells people how to get past it. Everything
# here is reproducible from a clean checkout — no assets are committed that
# this script cannot regenerate.
set -euo pipefail

VERSION="${1:-0.1.0}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
APP="$DIST/OpenTouchDesigner.app"
DMG="$DIST/OpenTouchDesigner-$VERSION-macos.dmg"
FETCH="$ROOT/scripts/fetch-bundled-python.sh"

cd "$ROOT"
rm -rf "$DIST"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

echo "==> building"
HOST="$(rustc -vV | awk '/^host:/ {print $2}')"
BIN_APP=()
BIN_CLI=()
PY_RESOURCES=()

python_rpath_subdir() {
    local t="$1"
    if [ "${OTD_UNIVERSAL:-0}" = "1" ]; then
        case "$t" in
            aarch64-apple-darwin) echo "python-aarch64" ;;
            x86_64-apple-darwin) echo "python-x86_64" ;;
            *) echo "python" ;;
        esac
    else
        echo "python"
    fi
}

build() {
    local t="$1"
    local py_root
    py_root="$("$FETCH" "$t")"
    local subdir
    subdir="$(python_rpath_subdir "$t")"
    PY_RESOURCES+=("${py_root}:${subdir}")

    export OTD_BUNDLE_PYTHON=1
    export OTD_PYTHON_RPATH_SUBDIR="$subdir"
    export PYO3_PYTHON="$py_root/bin/python3"
    export RUSTFLAGS="-C link-arg=-Wl,-rpath,@executable_path/../Resources/${subdir}/lib -L ${py_root}/lib"

    cargo build --release --target "$t" -p otd-app -p otd-cli
    BIN_APP+=("target/$t/release/otd-app")
    BIN_CLI+=("target/$t/release/otd")
}
build "$HOST"
if [ "${OTD_UNIVERSAL:-0}" = "1" ]; then
    OTHER=x86_64-apple-darwin
    [ "$HOST" = "$OTHER" ] && OTHER=aarch64-apple-darwin
    if build "$OTHER"; then
        echo "==> universal ($HOST + $OTHER)"
    else
        echo "!! $OTHER failed to build — shipping $HOST only."
        echo "!! Usually PyO3: linking it for another architecture needs a"
        echo "!! Python for that architecture as well."
    fi
fi

echo "==> bundling"
lipo -create "${BIN_APP[@]}" -output "$APP/Contents/MacOS/OpenTouchDesigner"
lipo -create "${BIN_CLI[@]}" -output "$APP/Contents/MacOS/otd"

for entry in "${PY_RESOURCES[@]}"; do
    src="${entry%%:*}"
    subdir="${entry##*:}"
    dest="$APP/Contents/Resources/$subdir"
    rm -rf "$dest"
    cp -R "$src" "$dest"
    echo "    python -> Contents/Resources/$subdir"
done

echo "==> verifying python link"
for bin in OpenTouchDesigner otd; do
    exe="$APP/Contents/MacOS/$bin"
    if otool -L "$exe" | grep -qE '/opt/homebrew|/usr/local/Cellar'; then
        echo "!! $bin still links to a build-machine Python — refusing to ship"
        otool -L "$exe" | grep -i python || true
        exit 1
    fi
done

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>OpenTouchDesigner</string>
  <key>CFBundleDisplayName</key><string>OpenTouchDesigner</string>
  <key>CFBundleIdentifier</key><string>dev.opentouchdesigner.editor</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>OpenTouchDesigner</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <!-- The Audio Device In CHOP opens the microphone, and Video Device In
       runs ffmpeg against the camera. Without these the system kills the
       request instead of asking, and the node reports a missing device. -->
  <key>NSMicrophoneUsageDescription</key>
  <string>The Audio Device In CHOP listens to an input so patches can react to sound.</string>
  <key>NSCameraUsageDescription</key>
  <string>The Video Device In TOP reads a camera as a texture.</string>
  <key>UTExportedTypeDeclarations</key>
  <array>
    <dict>
      <key>UTTypeIdentifier</key><string>dev.opentouchdesigner.project</string>
      <key>UTTypeDescription</key><string>OpenTouchDesigner Project</string>
      <key>UTTypeConformsTo</key>
      <array>
        <string>public.utf8-plain-text</string>
        <string>public.text</string>
      </array>
      <key>UTTypeTagSpecification</key>
      <dict>
        <key>public.filename-extension</key><array><string>otd</string></array>
      </dict>
    </dict>
    <dict>
      <key>UTTypeIdentifier</key><string>dev.opentouchdesigner.component</string>
      <key>UTTypeDescription</key><string>OpenTouchDesigner Component</string>
      <key>UTTypeConformsTo</key>
      <array>
        <string>public.utf8-plain-text</string>
        <string>public.text</string>
      </array>
      <key>UTTypeTagSpecification</key>
      <dict>
        <key>public.filename-extension</key><array><string>otdc</string></array>
      </dict>
    </dict>
  </array>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>OpenTouchDesigner Project</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>LSHandlerRank</key><string>Owner</string>
      <key>LSItemContentTypes</key><array><string>dev.opentouchdesigner.project</string></array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>OpenTouchDesigner Component</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>LSHandlerRank</key><string>Owner</string>
      <key>LSItemContentTypes</key><array><string>dev.opentouchdesigner.component</string></array>
    </dict>
  </array>
</dict>
</plist>
PLIST

echo "==> icon"
OTD_BIN="$APP/Contents/MacOS/otd" \
    "$ROOT/scripts/make-icon.sh" "$APP/Contents/Resources/AppIcon.icns"

if command -v ffmpeg >/dev/null && command -v ffprobe >/dev/null; then
  echo "==> bundling ffmpeg (from PATH)"
  mkdir -p "$APP/Contents/Resources/bin"
  cp "$(command -v ffmpeg)" "$APP/Contents/Resources/bin/ffmpeg"
  cp "$(command -v ffprobe)" "$APP/Contents/Resources/bin/ffprobe"
  chmod +x "$APP/Contents/Resources/bin/ffmpeg" "$APP/Contents/Resources/bin/ffprobe"
else
  echo "!! ffmpeg not on PATH — movies need brew install ffmpeg or rebuild after installing it"
fi

echo "==> default media"
cp "$ROOT/examples/media/apple.png" "$APP/Contents/Resources/apple.png"

echo "==> signing (ad-hoc)"
codesign --force --deep --sign - \
    --identifier dev.opentouchdesigner.editor \
    "$APP"
codesign --verify --deep --strict "$APP" && echo "    sealed"

echo "==> dmg"
STAGE="$DIST/stage"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
cp "$ROOT/docs/INSTALL-macOS.txt" "$STAGE/READ ME FIRST.txt"
hdiutil create -volname "OpenTouchDesigner" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"

echo "==> $DMG"
du -h "$DMG"
