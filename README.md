<div align="center">

# QuickBuds

**An alternative to HeyMelody and realme Link for your OnePlus, OPPO and realme earbuds.**

Android · Windows · Linux

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](./LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Android%208%2B%20%7C%20Windows%20%7C%20Linux-3DDC84.svg)](#install)
[![Release](https://img.shields.io/badge/Release-latest-blue.svg)](https://github.com/spizganed/QuickBuds/releases/latest)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=kofi&logoColor=white)](https://ko-fi.com/spizganed)

<img src="docs/images/phone.png" width="230" alt="QuickBuds on a phone"> &nbsp; <img src="docs/images/desktop.png" width="560" alt="QuickBuds on a PC">

</div>

QuickBuds is an open-source alternative to HeyMelody and realme Link. It talks to your earbuds over
Bluetooth and shows the controls that your model has: on the phone with real home screen widgets, and on
the PC with a tray icon. It is a hobby project; you can support it on [Ko-fi](https://ko-fi.com/spizganed).

## Install

Pair your earbuds in the system's Bluetooth settings first. QuickBuds connects to paired earbuds; it does not pair them.

| Platform | How |
| --- | --- |
| **Android 8+** | Install `QuickBuds<version>.apk` from the [latest release](https://github.com/spizganed/QuickBuds/releases/latest). Allow the Bluetooth permission when asked. |
| **Windows 10 / 11** | Unzip `QuickBuds<version>-windows-x64.zip` and run `quickbuds.exe`. It is portable: no installer, no drivers, no background service. |
| **Linux** | Run the command below. It needs only `curl` and BlueZ. |

```sh
curl -fsSL https://raw.githubusercontent.com/spizganed/QuickBuds/main/scripts/install-linux.sh | sh
```

The command puts the AppImage in `~/.local/share/quickbuds`, adds a `quickbuds` command and a menu entry, and needs no administrator rights.
The app updates itself. To remove it, run the same script with `uninstall`:
`curl -fsSL https://raw.githubusercontent.com/spizganed/QuickBuds/main/scripts/install-linux.sh | sh -s uninstall`.
If you prefer, download `QuickBuds-linux-x64.AppImage` or the tarball `QuickBuds<version>-linux-x64.tar.gz` from the release. The tarball needs BlueZ, GTK 3 and libayatana-appindicator.

> **Got the APK somewhere else?** The only official downloads are the GitHub releases. A copy from
> another site is genuine only if it is signed with this certificate (SHA-256), which
> `apksigner verify --print-certs <apk>` shows:
> `9ae0ea888079cbd86af5a24391cd351af4a81fc04ebe9bd8429074c736249f61`

## What it does

| | |
| --- | --- |
| **Battery and wear** | Each bud and the case, live. |
| **Noise control** | Off, Noise cancelling (Low, Medium, High, Smart), Adaptive, Transparency. |
| **Sound** | Equalizer with presets, custom curves and bass boost. High-quality audio, 3D audio, low latency mode. |
| **Hearing** | A hearing test that tunes the sound to your ears, and a fit test of the ear-tip seal. |
| **Controls** | Gestures per bud, wear detection, dual connection, find my earbuds, prompt volume, firmware version. |
| **Per model** | QuickBuds reads the model and shows only what that model supports. |
| **Android** | Home screen widgets (battery and controls, 2×2 or 4×2) and a first-run setup. |
| **PC** | A tray icon with quick controls (Windows) or a battery menu (Linux). |
| **Looks** | Material, QuickBuds and Pixel art styles. OLED Black, Classic Dark and White themes, your own accent and presets. The same on the phone, the widgets and the PC. |
| **Languages and updates** | 27 languages, switchable in the app. Updates from GitHub releases; the PC app updates itself. |

<div align="center">

| Widget: battery | Widget: noise control | Tray: Windows | Tray: Linux |
| :---: | :---: | :---: | :---: |
| <img src="docs/images/widget-battery.png" width="170"> | <img src="docs/images/widget-controls.png" width="170"> | <img src="docs/images/tray-windows.png" width="280"> | <img src="docs/images/tray-linux.png" width="220"> |

</div>

## Supported earbuds

QuickBuds works on **any Android 8+ phone** (Samsung, Google Pixel, Xiaomi, Nothing, Motorola, OnePlus…),
not only on OnePlus / OPPO phones. It knows every earbud in HeyMelody's and realme Link's lists, 127 in
all, and shows only the features each one has.

<details><summary><b>Confirmed</b> (12 models)</summary>

<details><summary><b>OnePlus</b> (4)</summary>

- **OnePlus Buds 4**: fully working. Every feature tested on a Nothing Phone (3a), Android 16, and on Windows and Linux.
- **OnePlus Buds Pro 3**: confirmed by its owner on Android: every feature tested through the app's model test.
- **OnePlus Nord Buds 3r**: confirmed by its owner on Android: battery, low latency, 3D audio, equalizer, dual connection, gestures, find my earbuds.
- **OnePlus Bullets Wireless Z3**: confirmed by its owner on Android: 3D audio, equalizer.

</details>

<details><summary><b>OPPO</b> (1)</summary>

- **OPPO Enco Buds2**: confirmed by its owner on Windows: connection, equalizer, battery.

</details>

<details><summary><b>realme</b> (7)</summary>

- **realme Buds Air7 Pro**: confirmed by its owner on Android and Windows: noise control, gestures, 3D audio.
- **realme Buds Air8**: confirmed by its owner on Android: battery, noise control, low latency, high-quality audio, 3D audio, hearing profile, equalizer, gestures, wear detection, find my earbuds.
- **realme Buds Air8 Pro**: confirmed by its owner on Android: battery, noise control, low latency, high-quality audio, hearing profile, dual connection, gestures, wear detection.
- **realme Buds Air 5 Pro**: confirmed by its owner on Android: noise control and the feature switches, read back from the buds.
- **realme Buds Wireless 3**: confirmed by its owner on Android: battery, noise control, low latency, high-quality audio, 3D audio, hearing profile, equalizer, dual connection, wear detection.
- **realme Buds T310**: confirmed by its owner on Android: battery, noise control, low latency.
- **realme Buds T200**: confirmed by its owner on Android: battery, high-quality audio, 3D audio, equalizer, dual connection, find my earbuds.

</details>

</details>

<details><summary><b>Not confirmed yet</b> (115 models)</summary>

Detected, with features built from HeyMelody's and realme Link's data. Some may not work.

<details><summary><b>OnePlus</b> (25 models)</summary>

OnePlus Bullets Wireless Z2, OnePlus Bullets Wireless Z2 ANC, OnePlus Buds, OnePlus Buds Z, OnePlus Buds Pro, OnePlus Buds Z2, OnePlus Nord Buds, OnePlus Buds N, OnePlus Nord Buds CE, OnePlus Buds Pro 2, OnePlus Nord Buds 2, OnePlus Buds Ace, OnePlus Nord Buds 2r, OnePlus Buds Pro 2R, OnePlus Buds 3, OnePlus Nord Buds 3 Pro, OnePlus Buds V, OnePlus Buds Ace 2, OnePlus Nord Buds 3, OnePlus Open Buds, OnePlus Buds 3V, OnePlus Buds Ace 3, OnePlus Nord Buds 4 Pro, OnePlus Nord Buds 4, OnePlus Flow Buds.

</details>

<details><summary><b>OPPO</b> (51 models)</summary>

OPPO Enco Quiet, OPPO Enco M31, OPPO Enco M32, OPPO Enco M33, OPPO Enco Free, OPPO O-Free, OPPO Enco W31, OPPO Enco W51, OPPO Enco W11, OPPO Enco X, OPPO Enco Air, OPPO Enco Play, OPPO Enco Free2, OPPO Enco Buds, OPPO Enco Air Lite, OPPO Enco W31 Lite, OPPO Enco R, OPPO Enco Air2, OPPO Enco Air2 Pro, OPPO Enco X2, OPPO Enco Free2i, OPPO Enco Air2i, OPPO Enco Air3, OPPO Enco R Pro, OPPO Enco R2, OPPO Enco Air3 Pro, OPPO Enco Free3, OPPO Enco X3i, OPPO Enco Air3i, OPPO Enco X3, OPPO Enco Air3s, OPPO Enco Air4 Pro, OPPO Enco Buds2 Pro, OPPO Enco R3, OPPO Enco Air 3i, OPPO Enco Free4, OPPO Enco Air4, OPPO Enco Air4i, OPPO Enco R3 Pro, OPPO Enco Buds3 Pro+, OPPO Enco Buds3, OPPO Enco Buds3 Pro, OPPO Enco R4, OPPO Enco R5, OPPO Enco Clip, OPPO Enco X3s, OPPO Enco Air5 Pro, OPPO Enco Air5s, OPPO Enco Air5, OPPO Enco Clip2, OPPO Enco Air4s.

</details>

<details><summary><b>realme</b> (36 models)</summary>

realme Buds Wireless 2S, realme Buds Wireless 3 Neo, realme Buds Wireless 5 ANC, realme Buds Wireless 6 Neo, realme Buds Wireless 6, realme Buds Wireless 6 ANC, realme Buds Air 3, realme Buds Q2s, realme Buds Air 3S, realme Buds T100, realme Buds Air 3 Neo, realme Buds Air 5, realme Buds T300, realme Buds T110, realme Buds Air6 Pro, realme Buds Air6, realme Buds N1 Pro, realme Buds N1, realme Buds T01, realme Buds Air7, realme Buds T200 Lite, realme Buds T200x, realme Buds Clip, realme TechLife Buds, realme Buds T500 Pro, realme Buds T500, realme Buds T500 Pro Harry Potter Edition, realme Buds Air 2 Neo, realme Buds Air, realme Buds Q2, realme Buds Air Pro, realme Buds Air 2, realme Buds Wireless 2, realme Buds Wireless 2 Neo, realme Buds Wireless Pro, realme Buds Air Neo.

</details>

<details><summary><b>DIZO</b> (3 models)</summary>

DIZO Wireless, DIZO GoPods D, DIZO GoPods.

</details>

</details>

**Own other buds? Please tell us what works.** In the app, Settings › Test my earbuds (desktop: App
settings) reconnects the buds, lets you mark each feature, and sends the result with the log. The app
offers this test once, after the first connect to a model that is not confirmed. Settings › Report a
problem sends a description and the log, without a GitHub account. Or open an
[issue](https://github.com/spizganed/QuickBuds/issues); [CONTRIBUTING.md](./docs/CONTRIBUTING.md#reporting-a-model)
says what to include. Each report lets the next release fix that model.

## For developers

- [CONTRIBUTING.md](./docs/CONTRIBUTING.md): build, tests, translations, where things are, and the rules.
- [PACKET-CAPTURE.md](./docs/PACKET-CAPTURE.md): capturing the earbuds' traffic, HeyMelody's included.
- [PROTOCOL.md](./docs/PROTOCOL.md): the wire format, every claim tagged with its source.
- [ROADMAP.md](./docs/ROADMAP.md): the plan, and the features decided against.

Everything to build and release QuickBuds is in this repository, except the signing key. If the
project stops, anyone can fork it and publish their own signed build.

## Credits

The first hints about the earbud protocol came from
[OppoPodsManager](https://github.com/Zhaoyi-ya/OppoPodsManager),
[Pods-Protocol-Reverse-Engineering](https://github.com/Star-ZER0/Pods-Protocol-Reverse-Engineering) and
[OppoPods](https://github.com/Leaf-lsgtky/OppoPods). Today every fact in [PROTOCOL.md](./docs/PROTOCOL.md) rests on a
capture from real hardware, on a test on a device, or on the HeyMelody and realme Link apps (studied for interoperability).

Built with [Slint](https://slint.dev) (GPLv3) and [tray-icon](https://github.com/tauri-apps/tray-icon) on the PC.
Fonts: [Doto](https://github.com/oliverlalan/Doto) and [Google Sans Flex](https://github.com/googlefonts/googlesans-flex),
both SIL Open Font License 1.1, licenses bundled in `app/src/main/assets/`.

## Privacy

Once a day QuickBuds sends its platform, its version number (for example "Android 5.0.0") and a random install ID to a Cloudflare Worker.
The Worker counts active installs with it. The ID ties to no device, account or address, and the app sends no earbud data.
A problem report carries the same ID and is sent only when you send it. On Android, About > Privacy > Reset install ID
forgets the ID. On the PC, delete the `app_id` key in `settings.json`.
The Worker code is in [report-worker/](./report-worker).

## License

Copyright (C) 2026 spizganed

QuickBuds is free software under the GNU General Public License v3.0 or later, distributed without
any warranty; see [LICENSE](./LICENSE).

QuickBuds is an independent project, not affiliated with, endorsed by or sponsored by OnePlus, OPPO,
realme, HeyTap or any of their affiliates. OnePlus, OPPO, realme, HeyMelody and the earbud model names
are trademarks of their respective owners and are used here only to say which devices the app works
with. The protocol was studied only to make these earbuds work with this app.
