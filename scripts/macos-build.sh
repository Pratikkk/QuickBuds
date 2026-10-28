#!/bin/bash
# Build a native .app using only Apple's command line tools and Python 3.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
configuration="${1:-release}"
if [[ "$configuration" != release && "$configuration" != debug ]]; then
    echo 'Usage: scripts/macos-build.sh [release|debug]' >&2
    exit 2
fi
swift_flags=(--cache-path "$root/macOS/.build/cache")
if [[ "${QB_SWIFT_DISABLE_SANDBOX:-0}" == 1 ]]; then swift_flags+=(--disable-sandbox); fi
python3 "$root/scripts/macos-resources.py"
swift build "${swift_flags[@]}" --package-path "$root/macOS" -c "$configuration"
bin_path="$(swift build "${swift_flags[@]}" --package-path "$root/macOS" -c "$configuration" --show-bin-path)"
app="$root/macOS/dist/QuickBuds.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$root/macOS/Resources/Info.plist" "$app/Contents/Info.plist"
cp "$bin_path/QuickBuds" "$app/Contents/MacOS/QuickBuds"
# Keep SwiftPM resources in the standard app Resources directory.
for bundle in "$bin_path"/*.bundle; do
    [[ -d "$bundle" ]] && cp -R "$bundle" "$app/Contents/Resources/"
done
iconset="$root/macOS/.build/QuickBuds.iconset"
mkdir -p "$iconset"
swift "$root/scripts/macos-icon.swift" "$root/macOS/.build/icon.png"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$root/macOS/.build/icon.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$root/macOS/.build/icon.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/QuickBuds.icns"
codesign --force --sign "${MACOS_SIGN_IDENTITY:--}" "$app"
echo "Built $app"
