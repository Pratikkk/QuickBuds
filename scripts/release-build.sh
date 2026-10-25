#!/usr/bin/env bash
# The Build workflow on this PC, many times faster than GitHub Actions: checks the locales, builds the signed APK
# and AAB, runs the desktop tests and builds the desktop archives. Every release file lands in
# local/release/<version>/ (git-ignored). GitHub Actions stays for builds from the phone.
# Usage: scripts/release-build.sh
# Needs: the release key in local/keys/, and what scripts/desktop-dist.sh needs.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
ver=$(sed -n 's/.*versionName = "\(.*\)"/\1/p' app/build.gradle.kts)
dver=$(sed -n 's/^version = "\(.*\)"/\1/p' desktop/Cargo.toml | head -1)
out=local/release/$ver
mkdir -p "$out"

python3 scripts/check-locales.py

# The desktop tests and the desktop archives build in the background while Gradle builds the APK and the AAB.
log=$(mktemp -d)
# Own target dir: cargo locks a target dir, so the tests would wait for the Windows build in desktop/target.
(cd desktop && CARGO_TARGET_DIR=target/test cargo test -q) >"$log/test.txt" 2>&1 & tpid=$!
scripts/desktop-dist.sh "$dver" >"$log/dist.txt" 2>&1 & dpid=$!
fail=0
./gradlew -q assembleRelease bundleRelease || fail=1
wait $tpid || { fail=1; cat "$log/test.txt"; }
wait $dpid || { fail=1; cat "$log/dist.txt"; }
[ $fail = 0 ] || exit 1
tail -3 "$log/test.txt"
cp app/build/outputs/apk/release/app-release.apk "$out/QuickBuds$ver.apk"
cp app/build/outputs/bundle/release/app-release.aab "$out/QuickBuds$ver.aab"
cp desktop/dist/* "$out/"
# The updater and the Linux installer verify the download against this file.
( cd "$out" && sha256sum QuickBuds*.apk QuickBuds*.aab QuickBuds*.zip QuickBuds*.tar.gz QuickBuds*.AppImage > SHA256SUMS )
ls -la "$out"
