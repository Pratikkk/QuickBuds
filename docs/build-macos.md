# Native macOS app (preview)

The `macOS/` target is a SwiftUI app for macOS 14 or later. It uses AppKit for system
integration and IOBluetooth for the Classic Bluetooth RFCOMM control link. It has no
third-party dependencies, web view, Rust toolchain, or background service.

## Build and run

Install Xcode or Apple's command line tools (Swift 5.9 or later), and Python 3. From
the repository root:

```sh
scripts/macos-build.sh
open macOS/dist/QuickBuds.app
```

The script stages the existing model catalog and Android translations, compiles the
release executable, generates the app icon, packages the resources, and signs the
bundle for local use. Copy the resulting `QuickBuds.app` to Applications if desired.
The executable targets the architecture of the Mac that builds it. Both Apple Silicon
and Intel source builds are supported; this script does not produce a universal binary.

Use `scripts/macos-build.sh debug` for a debug build. Open `macOS/Package.swift` in
Xcode after running `python3 scripts/macos-resources.py`. Run the packaged `.app`
for Bluetooth permission testing so macOS can find its usage descriptions and bundle ID.
When working inside a sandbox that disallows SwiftPM's nested sandbox, set
`QB_SWIFT_DISABLE_SANDBOX=1` for the build script.

## Use

1. Pair your earbuds in System Settings → Bluetooth and connect their audio.
2. Open QuickBuds and allow Bluetooth access when macOS asks.
3. Select the paired device in the sidebar and click Connect.
4. Battery levels and firmware appear when the earbuds report them. Noise controls
   appear only when the device's command bitmap and model catalog support them.

The menu bar item shows batteries and noise controls. Closing the main window keeps
the app running; choose Quit QuickBuds to end its Bluetooth control link. Disconnect
closes only the control channel, leaving the system's audio connection alone. After a link drop, QuickBuds retries the remembered device when macOS reports its audio connection available. Disconnect stops retries for the current app session.

An unknown device can connect for battery reads, but gets no guessed noise-control
writes. Model selection comes from the reported product ID, with a device-name fallback.
The displayed noise mode follows reports from the earbuds. Writes require both a
successful ack and a real mode push; a status read alone cannot confirm a write.
Failures leave the last reported value visible and show an error. Empty or unknown
battery levels display a dash.

## Preview scope

Implemented: paired-device selection, asynchronous SDP discovery with both vendor
UUIDs, RFCOMM transport, handshake, event subscriptions, product identification,
live battery reporting, firmware version, model-specific noise-control writes,
confirmation failures, system appearance, menu bar controls, and shared translations.

Still to port: equalizer, feature switches, gestures, dual-device management, fit and
hearing tests, feedback, app updates, and custom themes.
Generic noise-cancelling reports may not identify a particular intensity; this preview
does not infer an unreported level. Platform transport diagnostics are currently English.
Live reads were verified on the development Mac with OPPO Enco Air5, firmware
`116.116.101`: connection, both earbud batteries at 100%, and Smart noise mode.
Noise-control writes, reconnect behavior, and other models remain unverified. A
successful build or protocol test alone is not evidence of hardware compatibility.

No telemetry, feedback uploads, or update requests run in this target.

## Validation

```sh
python3 scripts/macos-resources.py
swift test --package-path macOS
scripts/macos-build.sh
macOS/dist/QuickBuds.app/Contents/MacOS/QuickBuds --check-resources
codesign --verify --deep --strict macOS/dist/QuickBuds.app
```

Tests cover captured handshake and ANC frames, long lengths, fragmented reads, sliced
messages, malformed packets, battery parsing, product-ID folding, report-only ANC
aliases, and ANC ack/push ordering. The native codec mirrors `core/src/wire.rs` and
`core/src/msg.rs`; keep changes aligned with those files and `PROTOCOL.md`. The existing
Rust core's C ABI contains only spike probes, so this target does not link it yet.

Before distributing a release, test the build with physical earbuds and record the
model, firmware, macOS version, permission behavior, connection and UUID fallback,
battery pushes, every exposed noise mode, refused writes, and case-close disconnects.
For distribution to other Macs, sign with a Developer ID and notarize using your own
Apple developer credentials. The build's default ad-hoc signature is for local use.

The home screen prioritizes OPPO Enco Air5 in the paired-device list. Battery reports
merge per part during a connection, preserving the case's last reported percentage
when subsequent reports omit it. Case readings are labeled as last reported; opening
the lid and using Refresh battery requests a new reading with the existing battery
query. Switching devices clears battery readings; reconnecting the same device retains them as last known until refreshed. Case
availability on Air5 still depends on what its firmware reports; no percentage is
estimated when the device has not supplied one.


Automatic reconnect remembers the selected paired-device ID locally and checks every
five seconds while disconnected. It attempts a control link only when macOS reports
that device connected. Explicit Disconnect disables retries until Connect is selected
again or the app restarts. Battery timestamps track each part separately. Omitted
parts retain their value with a cached label, and disconnect marks all readings cached.
The overview shows elapsed time; tray battery tooltips show the last-received time.
Physical case-close/reopen and sleep/wake reconnect behavior still need verification.
