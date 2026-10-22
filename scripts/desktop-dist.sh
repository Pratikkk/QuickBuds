#!/usr/bin/env bash
# Builds the desktop release archives into desktop/dist/:
#   QuickBuds<version>-windows-x64.zip   quickbuds.exe only (portable, system DLLs only)
#   QuickBuds<version>-linux-x64.tar.gz  binary + .desktop + icon + README
#   QuickBuds-linux-x64.AppImage         the same binary with its GTK libraries inside (scripts/desktop-appimage.sh).
#                                        No version in the name: releases/latest/download/ always finds it.
# Usage: scripts/desktop-dist.sh <version>   (e.g. 4.3.0). QB_APPIMAGE_ONLY=1 builds the AppImage only.
# Needs: rustup target x86_64-pc-windows-gnu, mingw-w64-gcc, podman (unless QB_NATIVE=1), zip, python3.
# The Windows build and the Linux build run at the same time. The podman image (scripts/desktop-build.Containerfile) is built once.
# Linux builds in Ubuntu 22.04 (glibc 2.35) so the binary runs on older distros than the build PC.
set -euo pipefail
ver=${1:?version}
root=$(cd "$(dirname "$0")/.." && pwd)
dist=$root/desktop/dist
rm -rf "$dist" && mkdir -p "$dist"

# Windows, in the background while Linux builds; its output goes to a log.
if [ -z "${QB_APPIMAGE_ONLY:-}" ]; then
  winlog=$root/desktop/target/windows-build.log
  mkdir -p "$root/desktop/target"
  (
    cd "$root/desktop" && cargo build --release --target x86_64-pc-windows-gnu
    cd "$root/desktop/target/x86_64-pc-windows-gnu/release" && zip -q "$dist/QuickBuds$ver-windows-x64.zip" quickbuds.exe
  ) >"$winlog" 2>&1 &
  winpid=$!
fi

# The build image: rebuilt only when the Containerfile changes.
if [ -z "${QB_NATIVE:-}" ]; then
  img=quickbuds-build:22.04
  want=$(sha256sum "$root/scripts/desktop-build.Containerfile" | cut -c1-12)
  [ "$(podman image inspect -f '{{index .Labels "cf"}}' "$img" 2>/dev/null)" = "$want" ] ||
    podman build -q -t "$img" --label "cf=$want" -f "$root/scripts/desktop-build.Containerfile" "$root/scripts" >/dev/null
fi

# Linux, in the build image; the cargo registry and target dir persist between runs. QB_NATIVE=1 builds in place
# instead, for CI, which already runs in Ubuntu 22.04 with the packages below.
if [ -n "${QB_NATIVE:-}" ]; then
  (cd "$root/desktop" && CARGO_TARGET_DIR="$root/desktop/target/ubuntu22" cargo build --release)
else
podman run --rm --userns=keep-id -v "$root:/repo" -v quickbuds-cargo:/cargo-reg -w /repo/desktop \
  -e CARGO_HOME=/cargo-reg -e CARGO_TARGET_DIR=/repo/desktop/target/ubuntu22 \
  --user root "$img" bash -c '
    set -e
    export PATH=/cargo/bin:$PATH
    cargo build --release
    chown -R '"$(id -u):$(id -g)"' /repo/desktop/target/ubuntu22 /cargo-reg'
fi

pkg=$dist/QuickBuds$ver-linux-x64
mkdir -p "$pkg"
cp "$root/desktop/target/ubuntu22/release/quickbuds" "$pkg/"
# The launcher icon the build already makes for the window (build.rs LAUNCHER).
icons=$(ls -t "$root"/desktop/target/ubuntu22/release/build/quickbuds-*/out/icons.rs | head -1)
python3 - "$icons" "$pkg/quickbuds.svg" <<'EOF'
import ast, re, sys
line = next(l for l in open(sys.argv[1]) if l.startswith('pub const LAUNCHER'))
lit = re.sub(r'\\u\{([0-9a-fA-F]+)\}', lambda m: '\\U%08x' % int(m.group(1), 16), line.split('=', 1)[1].strip().rstrip(';'))
open(sys.argv[2], 'w').write(ast.literal_eval(lit))
EOF
cat > "$pkg/quickbuds.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=QuickBuds
Comment=Control OnePlus / OPPO / realme earbuds
Exec=quickbuds
Icon=quickbuds
Terminal=false
Categories=AudioVideo;Audio;
EOF
cat > "$pkg/README.txt" <<'EOF'
QuickBuds for Linux

Needs BlueZ and, from your distro: GTK 3, libayatana-appindicator3, D-Bus
(Debian/Ubuntu: libgtk-3-0 libayatana-appindicator3-1; Arch: gtk3 libayatana-appindicator).
Pair the buds in your system's Bluetooth settings first.

Run in place: ./quickbuds
Install for your user:
  install -Dm755 quickbuds ~/.local/bin/quickbuds
  install -Dm644 quickbuds.desktop ~/.local/share/applications/quickbuds.desktop
  install -Dm644 quickbuds.svg ~/.local/share/icons/hicolor/scalable/apps/quickbuds.svg
EOF
[ -n "${QB_APPIMAGE_ONLY:-}" ] || tar -C "$dist" -czf "$pkg.tar.gz" "$(basename "$pkg")"

# The AppImage takes its libraries from the system that built the binary: Ubuntu 22.04 again.
appimage=QuickBuds-linux-x64.AppImage
if [ -n "${QB_NATIVE:-}" ]; then
  "$root/scripts/desktop-appimage.sh" "$pkg" "$dist/$appimage"
else
podman run --rm --userns=keep-id -v "$root:/repo" -w /repo --user root "$img" bash -c '
    set -e
    scripts/desktop-appimage.sh "desktop/dist/'"$(basename "$pkg")"'" "desktop/dist/'"$appimage"'"
    chown -R '"$(id -u):$(id -g)"' desktop/dist desktop/target/appimage-tools'
fi
rm -rf "$pkg"
if [ -n "${winpid:-}" ]; then wait "$winpid" || { cat "$winlog"; exit 1; }; tail -2 "$winlog"; fi
ls -la "$dist"
