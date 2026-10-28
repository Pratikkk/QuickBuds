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

The menu bar item shows batteries, noise controls, and an EQ preset menu, plus refresh and connect/disconnect actions. Closing the main window keeps
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

Still to port: remaining feature switches, gestures, fit and
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


Automatic reconnect remembers the selected paired-device ID locally and checks while disconnected with retries paced at 2, 5, 10, 20, then 30 seconds. It attempts a control link only when macOS reports
that device connected. Explicit Disconnect disables retries until Connect is selected
again or the app restarts. Battery timestamps track each part separately. Omitted
parts retain their value with a cached label, and disconnect marks all readings cached.
The overview shows elapsed time; tray battery tooltips show the last-received time.
Physical case-close/reopen and sleep/wake reconnect behavior still need verification.


## Air5 Sound Master EQ and device management

For detected Air5 (`06D810`) the overview now reads the device list, dual-connection
status, selected EQ, and custom EQ table. Controls are gated by the reported command
bitmap. The three built-in presets use catalog slots 0 Ultimate sound, 1 Pure vocals,
and 2 Thundering bass. Custom presets support ten catalog bands, creation, editing,
apply, and deletion, up to the catalog limit (default three). Names are limited to
64 UTF-8 bytes; writes preserve the gain range read from each preset. New presets
start at -6..+6 dB. Draft slider edits are sent only on Save.

Dual connection has a switch where its status is reported. The device list exposes
Connect/Disconnect where `0x0429` is supported. The current host is protected from
Disconnect, and other device actions use a confirmation dialog. Air5 has no preferred
device function in its catalog, so that control is not offered.

Every operation waits for a matching write acknowledgment and then requests a read
with a separate sequence. Displayed selections come from device reports. Refusals,
missing replies, malformed confirmation replies, and unchanged values report an
error. Disconnection cancels pending operations. Custom-table reads after create or
delete account for firmware-assigned and renumbered IDs.

The test suite verifies Air5 slots/bands against the shared catalog, signed custom-EQ
payloads and UTF-8 name lengths, malformed tables, and device-address byte order.
Live verification on Air5 firmware `116.116.101`: both NARZO 70 and the current Mac
appear in the device list; Pure vocals selection is read back correctly, and Ultimate
sound is restored. A temporary flat ten-band custom preset was created, confirmed
in the returned table, then deleted; Ultimate sound was selected afterward. Editing
nonzero gains, dual toggling, and remote-device connect/disconnect still need physical
verification. Protocol tests and compilation alone do not establish those behaviors.


## Tray and reconnect reliability

The compact tray offers Connect for the remembered Air5, cancellation during a
connection attempt, Disconnect, battery refresh, dismissible errors, and the supported
built-in EQ presets. It shows a countdown while an automatic retry is scheduled.
Explicit Connect resets retry pacing. Successful command-bitmap initialization also
resets pacing; absent devices and failed attempts back off to a maximum 30 seconds.
No automatic attempt starts unless macOS reports the remembered device connected.

Before system sleep the control connection and pending tasks are closed. Wake restarts
retries if the user has not explicitly disconnected. An open channel that does not
provide a command bitmap within eight seconds is closed and retried. Protocol and
retry-policy tests cover capped pacing and reset; physical sleep/wake and silent-link
failure scenarios remain to be verified.

Live restart check on Air5 firmware `116.116.101`: after quitting and relaunching,
the remembered earbuds reconnected automatically without selecting Connect. Battery,
ANC, connected-device list, EQ selection, and firmware reads were restored.

## Runtime hardening

Queued writes carry a per-connection cancellation gate; disconnect invalidates queued
work before closing the channel. A 64-write cap prevents an unresponsive transport
from accumulating work indefinitely. Write failures and expired connection timers
cannot tear down a newer connection through an old gate.

Battery queries are paced to at most one per second, with one deferred refresh to
retain the latest lid/wearing event in a burst. Deferred work is canceled on
connection teardown. Battery arrays publish only when values change; timestamps
still record each received report. Manual audio-table refreshes cannot interrupt a
pending setting confirmation.

The framer rejects missing or out-of-order slice counters. Invalid custom-EQ gains
are rejected before reaching sliders, and fixed gain ranges disable editing.
F4 acknowledgments now match the Rust core for empty JSON payloads and valid JSON
values without a string command. Tests cover these guards and malformed input.

A matching acknowledgment followed by an unchanged, valid status read no longer
fails immediately. Read-back confirmation retries every 500 ms within the original
six-second operation deadline, allowing asynchronous device changes to settle.
Invalid replies and explicit refusals fail immediately. These retries only read
state; they never resend the original write. Closing the connection cancels them.

Battery age labels update on a 30-second timeline rather than every second. The
compact tray's retry countdown remains live when shown. A launch-time CPU sample
is not used as an idle benchmark; before the final age-label change, a settled live
snapshot on the development Mac showed 0.9% CPU and about 84 MiB RSS. This is a
single observation, not a long-running energy or memory guarantee.

Live lifecycle check during hardening: explicit Disconnect remained disconnected
beyond the first retry interval; selecting Connect restored Air5 battery reads.
Long-duration soak testing and physical sleep/wake testing remain outstanding.


## Remote-device reconnect confirmation

Device actions use the existing documented `0x0429 01|02 <MAC>` payloads. They now
observe device-list reads once per second for up to 15 seconds, independently of
whether a write acknowledgment arrives, and accept a matching real device-state
push. Only the original command is sent; polling never resends the action. Explicit
refusals still fail immediately. Other audio operations keep their six-second window.
A bounded on-screen diagnostic records request/acknowledgment/read/push events without
Bluetooth addresses; it is not uploaded or written to disk.

The phone must already be paired with the earbuds. QuickBuds cannot establish that
pairing on the phone. The user clarified that Disconnect works but Connect from the Mac does not reconnect
NARZO 70. Live diagnostics showed repeated device-list replies and state pushes, with
only one connected device during the 15-second observation window. The phone appeared
connected later. This does not establish that pairing was removed. The implementation
sends documented disconnect action `02`, never unused unpair action `03`. Remote
reconnect on this firmware remains unverified; use the phone's Bluetooth settings as
a fallback. Late device-state confirmation clears only the corresponding timeout
warning, preserving unrelated errors. The timeout diagnostic retains whether a write
acknowledgment was received.

Device-action failure handling is shared by the deadline timer and a status reply
arriving at the deadline. Both retain the acknowledgment diagnostic, produce a
device-specific warning, and register that warning for later state reconciliation.
This avoids a generic audio-change warning winning the race with the device timeout.
The unsuccessful remote reconnect itself is still unresolved on Air5 firmware.

## Compact overview layout

The main window keeps a compact device header with Disconnect beside its status.
The battery card uses smaller rings and horizontal part summaries, with a single
refresh action. Each received part shows its localized last-received time, with a
full timestamp in its tooltip; cached parts retain their Last known label.

A native segmented control separates Sound (ANC and Sound Master EQ) from Devices
(dual-device management). Firmware and battery diagnostics sit in a collapsed Details
section. Device help is also collapsed by default. Existing colors, materials, tray
controls, Bluetooth operations, and remote reconnect limitations are unchanged.
