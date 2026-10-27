# PROTOCOL-LEGACY.md — the earbud wire format (legacy layer)

This describes the shipping protocol layer (`LegacyProtocol.kt` with `Wire.kt`, `Replies.kt` and the topic decoders in `protocol/`, `desktop/src/protocol.rs`). The project merges verified facts from
[PROTOCOL.md](./PROTOCOL.md) into it. The layer in PROTOCOL.md replaces it only at full parity with HeyMelody and realme Link.

The OPPO / OnePlus / realme earbud RFCOMM protocol as QuickBuds uses it. Current knowledge only;
the history of how it was found is in git.

**Evidence tags.** Every claim has one tag:
- `[VENDOR]`: HeyMelody's behaviour, or realme Link's where a line says so. The project studied the vendor apps for interoperability only.
- `[CAPTURE]`: a frame in a capture on our own buds.
- `[TESTED]`: a manual test on our own buds.
- `[GUESS]`: unverified.
Never promote a claim without evidence. Never remove a `[GUESS]` tag to tidy up.

**Test device:** OnePlus Buds 4, firmware `B4.1-260810-1153` (HeyMelody shows `138.138.105`). Other models differ in feature ids, button
ids and ANC bits. "Wired, unverified" means the app sends it but no owner of that model has read a
write back.

**Update this file in the same commit as the code change.** Keep section numbers stable: the code
cites them.

---

## 1. Transport

Classic Bluetooth **RFCOMM / SPP**, not BLE. One long-lived socket. The buds push events at any time.

| UUID | Use |
|---|---|
| `0000079A-D102-11E1-9B23-00025B00A5A5` | Tried first. The one that works on Buds 4 `[CAPTURE]`, as HeyMelody does `[VENDOR]` |
| `00001107-D102-11E1-9B23-00025B00A5A5` | Tried second. Never connects on Buds 4 (~5 s timeout); kept for other models |

The connect order is both UUIDs secure first, then both insecure: `079A` secure, `1107` secure, `079A`
insecure, `1107` insecure. Then RFCOMM channel 15 by reflection. A 12 s watchdog closes a connect that
blocks. Five retries follow a failed connect, and eight a lost link `[CAPTURE]`.

A case lid close kills the socket (§8).

---

## 2. Frame format

```
AA  <TotalLen>  00 00  <Cmd LE>  <Seq>  <PayLen LE>  <Payload...>
1B  LEB128      2B     2B        1B     2B           PayLen bytes
```

| Field | Meaning |
|---|---|
| `AA` | Header |
| TotalLen | Length of everything after this field: `7 + PayLen` |
| `<Slice> 00` | Slice state, `00 00` for a whole message (see Slicing) |
| Cmd | Command id, little-endian |
| Seq | Picked by the sender; the reply echoes it. **`0xFF` = a push from the buds** `[CAPTURE]` |
| PayLen | Payload length, little-endian |

- **TotalLen is standard LEB128** `[CAPTURE]`: one byte below 128, else 7 bits per byte, low first,
  high bit = more. A length that subtracts 1 first does not fit. A 124-byte payload goes out as `AA 83 01`
  (131). A 250-byte reply arrives as `AA 81 02` (257). Golden Sound (§9) uses frames over 127 bytes.
- `OpoProtocol.buildPacket()` writes it. `OppoPacketFramer` reads it and normalises every frame to a
  one-byte length. After the framer the header is a fixed 9 bytes: **cmd at 4-5, payload from
  index 9** (`BudsConnectionManager.payloadOf()`). A wrong offset gives garbage, not an error.
- Replies are **`cmd | 0x8000`** `[CAPTURE]`. Write acks (`0x84xx`) carry `00` for success `[CAPTURE]`.
  **An ack of `00` does not prove the write took** (§5 hold mask); read back.
- **Seq ranges** `[CAPTURE]`: `00` the status poll `0x010D` and the connect reads, `01`-`BF` the generic
  counter, `C0`-`DF` the feature-sync engine, `F0` the battery query, `F2` the wearing query, `FF` a push.
  A status read with a non-zero seq gets an echo in the reply `0x810D` `[TESTED 2026-10-24]`.
- A refused `0x0403` write carries `01` (the buds lack the id) or `02` (they refuse it now). Any other
  non-zero status is a refusal now `[CAPTURE]`. §9 lists the statuses. The buds refuse a `0x0404` write with `0E` (§5)
  when no bud is in an ear.

```
AA 07 00 00 00 01 01 00 00                  handshake 0x0100, PayLen 0
AA 0A 00 00 04 04 06 03 00 01 01 01         set ANC Off
AA 0C 00 00 04 02 FF 05 00 03 01 01 08 00   ANC push (Seq FF)
```

### Slicing `[VENDOR]`

A message that does not fit the frame budget goes out in slices. QuickBuds sends none: no command it writes is large enough.
The framers (`OppoPacketFramer`, `Framer` in `protocol.rs`) join received slices into one whole message.

```
AA  <TotalLen>  <Slice> 00  <Fsn>  <part of Cmd/Seq/PayLen/Payload>
```

- `Slice`: `00` whole message, `01` first slice, `02` middle slice, `03` last slice.
- `Fsn` (slice counter, from `00`) is present in every slice, never in a whole message.
  `TotalLen` is then `3 + data`, not `2 + data`.
- The receiver joins the slice data in order. It parses the message when `Slice` is `03`. A middle or last slice with no first slice before it is dropped.
  A declared `PayLen` larger than the bytes present cuts the payload to the bytes present. A slice frame has `TotalLen` 4 or more, a whole message 7 or more.
- Frame budget: 660 bytes by default; the reply to `0x0107` can lower it. Buds that report a larger budget
  allow bigger slices for bulk transfers. Over 129 bytes `TotalLen` takes two LEB128 bytes.
- A read that starts with `AA` begins a new frame. Any other read continues the frame in progress.
  A new `AA` before the frame is complete drops the pending bytes.
- The sender waits for each write to finish (5 s limit) and pauses a model-specific interval between slices.

---

## 3. Command map

Details in the sections cited. `—` = empty payload.

### Queries (`0x01xx` → `0x81xx`)

| Cmd | Name | Request → reply | § |
|---|---|---|---|
| `0x0100` | Handshake | — → command bitmap | 4 |
| `0x0103` | Product id | — → `00 <3 bytes LE>` | 4 |
| `0x0105` | Firmware version | — → `00 <count> <triples>` | 4 |
| `0x0106` | Battery | — | 7 |
| `0x0108` | Gesture table | — → `<status> <count> <entries>` | 6 |
| `0x0109` | Wearing | — | 8 |
| `0x010C` | ANC state / hold cycle | `01 01`, `02 01`/`02 03`/`02 04`, `04 01` | 5 |
| `0x010D` | Feature switches | `<count> <ids>` | 9 |
| `0x010F` | Current EQ | — → `00 <id>` | 9 |
| `0x0112` | Dual connection device list | — | 9 |
| `0x0114` | Current codec | — → `00 <codec>` | 9 |
| `0x0115` / `0x011E` | Golden Sound active record | — | 9 |
| `0x0116` / `0x011F` | Golden Sound filters | see §9 | 9 |
| `0x011A` | Personalised ANC result stored? | — → `00 <exist>` | 9 |
| `0x0122` | Custom EQ list | — | 9 |
| `0x0123` | Codecs offered | — → `00 <u16 LE mask>` | 9 |
| `0x0124` | BassWave level | — → `00 FB 05 <level>` | 9 |
| `0x012A` | Spatial type | — → `00 <type>` | 9 |
| `0x012B` | Game sound type | — → `00 <selected> <count> <types>` | 9 |
| `0x0130` | Alert-sound volume | — → `00 <level>` | 9 |
| `0x0132` | Preferred device | `02` | 9 |
| `0x0133` | Tap sensitivity | — → `00 <level> <default>` | 9 |
| `0x0134` | Head gesture mapping | — → `00 <type>` | 9 |

### Subscriptions and pushes (`0x02xx`)

| Cmd | Name |
|---|---|
| `0x0200` | Query broadcast codes → `0x8200` (§4) |
| `0x0205` | Subscribe → `0x8205` ack (§4) |
| `0x0204` | **Push**, Seq `0xFF`, `payload[0]` = event code (table in §4) |

### Writes (`0x04xx` → `0x84xx`)

| Cmd | Name | Payload | § |
|---|---|---|---|
| `0x0400` | Find my earbuds | `01` / `00` | 9 |
| `0x0401` | **Gesture write** | `<count> [dev, btn, act, fn]...` | 6 |
| `0x0403` | Feature switch | `<id> <01/00>` | 9 |
| `0x0404` | ANC mode / hold cycle | `01 01 <mask>` / `02 <type> <mask>` | 5 |
| `0x0405` | Fit test | `01` / `00` | 9 |
| `0x0406` | Select built-in EQ | `<id>` | 9 |
| `0x040D` / `0x040E` / `0x0411` / `0x0415` | Golden Sound | see §9 | 9 |
| `0x0412` | Personalised ANC test | `01` / `02` / `03` | 9 |
| `0x0413` | Phone state for device switching | `<flags> 00 <values>` | 9 |
| `0x0418` | Custom EQ create / save / delete | see §9 | 9 |
| `0x041A` | Set codec (`hiRes` models) | `<codec> <hiRes> 00` | 9 |
| `0x041B` | BassWave level | `FB 05 <level>` | 9 |
| `0x041E` | 3D audio source: `00` buds, `01` phone | `<source>` | 9 |
| `0x0422` | Spatial type (bitmap `0x012A` models) | `<type>` | 9 |
| `0x0423` | Game sound type | `<type> 01` | 9 |
| `0x0427` | Alert-sound volume | `<level>` 1..10 | 9 |
| `0x0429` | Device manager / preferred device | see §9 | 9 |
| `0x042D` | Tap sensitivity | `<level>` 1..5 | 9 |
| `0x0431` | Head gesture mapping | `<type>` | 9 |

### Writes QuickBuds does not send `[VENDOR]`

Payloads are as HeyMelody builds them. The names come from where the vendor app calls them. `[GUESS]` = the
meaning, not the bytes. All are gated by the handshake bitmap (§4) except `0x042B`, `0x042C`, `0x0430`, `0x0434`.

| Cmd | Payload | Meaning |
|---|---|---|
| `0x0408` | `<hostType> <host MAC 6> <count> {<type> <MAC 6> <state>}`, count at most 6 | Related-device list. HeyMelody has the sender but nothing calls it `[VENDOR]` |
| `0x040F` | `<state>` | Camera state `[VENDOR]` |
| `0x0414` | `<v>` | Unknown |
| `0x0419` | `<v>` | Unknown. Bitmap bit 35 |
| `0x041D` | `<v>` | Unknown |
| `0x0420` | `<type> <state>` | Game equalizer: `01 01` when a game app comes to the foreground, `01 00` when it leaves. Applies only while feature switch `21` (`0x0403 21 <01/00>`) is on `[VENDOR]`. No read command; bitmap bit 44 gates it |
| `0x0421` | `<state> <step>` | Spine-health range, part of a posture-tracking feature with local history charts `[VENDOR]` |
| `0x0424` | `<type> <value>` | LE Audio action |
| `0x0426` | `<v>` | Unknown |
| `0x042B` | `<type> <value>` | Unknown, no bitmap gate |
| `0x042C` | `<on> 00` | Unknown, no bitmap gate |
| `0x042E` | `<v>` | Multi-conversation switch `[GUESS]` |
| `0x0430` | `<v>` | Unknown, no bitmap gate |
| `0x0432` | `<t>`; the app sends `01`, `02` or `04` for its modes 0, 1, 2 | Unknown. Bitmap bit 67 |
| `0x0433` | `<on>` | Unknown. Bitmap bit 64 |
| `0x0434` | `<off>`: the app sends `01` when its switch is off | Unknown, no bitmap gate |
| `0x0435` | `00` | Trigger, unknown. Bitmap bit 67 |

### Other pushes

| Cmd | Name |
|---|---|
| `0x0500` | Time request (§9, answered) |
| `0x0501` / `0x0502` | Camera state request / shutter request (tap to take a photo) `[VENDOR]`. Not answered |
| `0x0503` | Feature push: `<count> <id> <value>...`. Reports a changed switch after a `0x0403` write or a buds gesture (§9) |
| `0x0504` | EQ mode changed: `<id>` (§9) |
| `0x0510` | Spatial type changed: `<type>` (§9) |

**Not commands:** `0x01F0` / `0x01F2` are a command byte welded to its Seq. Battery is `0x0106`,
wearing `0x0109`.

---

## 4. Connection

### Init sequence `[CAPTURE]`

`BudsConnectionManager.runInitSequence()`: 300 ms before the first frame, then 200 ms apart.

```
0x0100 handshake → 0x0103 product id → 0x0200 broadcast codes → 0x0205 subscribe →
0x010D features → 0x010C 01 01 ANC → 0x010C 04 01 Smart level (Smart models) →
0x0130 alert volume → 0x0106 battery → 0x0109 wear → 0x0108 gesture table →
0x010C 02 01 hold cycle (02 03 / 02 04 on per-bud holds) → 0x0105 firmware →
0x012A spatial type → 0x0123 codec list → 0x0114 codec → 0x012B game sound →
0x0134 head motion → 0x041E 00 3D audio on the buds (spatialToggle models)
```

With bit 56, every read after the subscription goes out in one `0x012F` frame.

The app does not poll after that. The buds push battery, wear, ANC and game mode. Was: a
keep-alive poll. Is: none. No stale link seen `[TESTED]`.

### What the buds are and accept

- **`0x8100` (handshake reply) is a command bitmap** `[VENDOR]`: `00`, then bytes read LSB first. Bit n
  enables a fixed list of commands. Bit 3 = `0x0108` `0x0401` `0x0416`, bit 8 = `0x010C` `0x0404`,
  bit 57 = `0x0427` `0x0130`. The table has 69 bits; the full table is in `Capabilities.kt`, `types.rs`
  and PROTOCOL.md section 4. Bits 35, 64, 67 and 68 enable `0x0419`, `0x0136`, `0x0433`, `0x0432`,
  `0x0135`, `0x0435` and `0x0117`. QuickBuds never sends these (payloads unknown). Always allowed
  without a bit: `0x0100`-`0x0104`, `0x0106`,
  `0x010B`, `0x010D`, `0x0F00`, `0x0F03`, `0x0F04`. HeyMelody sends nothing else, and neither do we
  (`protocol/Capabilities.kt`). Buds 4 `[CAPTURE]`: `00 FF 77 5A EA 67 0E 20 07` — every command the
  app uses, not `0x0422` / `0x012A`.
- **`0x8103` product id:** `00` + 3 bytes LE. Buds 4: `00 14 54 06` = `065414` `[CAPTURE]`. Four colour
  ranges fold into one id `[VENDOR]`: `100100`-`100102` → `060414`, `100200`-`100202` →
  `060814`, `108100`-`108102` → `068414`, `108200`-`108202` → `068814` (`ModelCatalog.normalise`).
- **Model lookup** `[VENDOR]` (`ModelCatalog.find`): entries in `assets/models.json` whose `name`
  equals the Bluetooth name exactly. Also those whose `id` equals the product id. An entry matching
  both wins, then the first name match, then the first id match. A model picked by hand overrides it
  until other buds connect. Name before id matters: OPPO Enco Buds2 report `060C12`, the realme Buds Q2s
  id (issue #5) `[CAPTURE]`. A renamed device matches no name and falls back to its id.
- **`0x8105` firmware** `[CAPTURE]`: `00 <count>` then UTF-8 `deviceType,versionType,version` triples.
  Buds 4: `00 04` + `1,2,138,2,2,138,3,1,01,3,2,105`. HeyMelody shows `138.138.105`: the `versionType` 2
  versions in reply order, joined by dots (`OpoProtocol.firmwareVersion()`). Device types: 1 left, 2 right,
  3 case `[VENDOR]`.
- **`0x810D` lists only the switches the firmware has** (§9). A missing id = no such switch
  `[CAPTURE]`.

### Batch query — `0x012F` `[CAPTURE]`

HeyMelody on the Buds 4: after `0x0100`, `0x0200` and `0x0205` it sends **one `0x012F` frame with
24 queries**. Bitmap bit 56. Buds without it (Enco Buds2: 4-byte bitmap) get the queries one by one.
Both apps do the same: with bit 56, the connect-time reads after the event registration go out as
one `0x012F`. A later status read goes alone.

- Request payload: `<count>`, then per query `<cmd LE> <len LE> <payload>`. Buds 4 (`0D 01 0A 00 …` =
  `0x010D` with 10 bytes): `0102 FFFF`, `0106`, `010B`, `0103`, `0101 0002`, `0114`, `0105`, `0107`,
  `0108 020301`, `010C 0201`, `010C 0101`, `010C 0301`, `010C 0401`, `010F`,
  `010D 09 05 04 0B 11 18 06 1B 1D 1C`, `0121`, `0123`, `0118 0101`, `011C`, `0115`, `011E`, `0122`,
  `0105`, `0109`.
- Replies: several `0x812F` frames with the request's seq, each `00 <count>` then per answer
  `<cmd LE> <len LE> <normal reply payload>` (here 1 + 21 + 2 = 24 answers). `cmd` is the query's;
  each answer is handled as reply `cmd | 0x8000` `[VENDOR]`. The app drops an answer whose status byte is not `00`. Logged as `RX[batch]` (Android) / after a `batch reply, N answers` line (desktop).
- **HeyMelody's `0x010D` id list for the Buds 4 is `05 04 0B 11 18 06 1B 1D 1C`**; the buds answered
  `00 08 05 00 04 00 0B 01 11 01 18 01 06 00 1B 00 1D 01` (no `1C`). It does not ask for `17`, `0C`,
  `09`, `30`-`3B`. OppoPodsManager's model list predicts `0D` and no `06` for this model. Its
  `function` maps are not HeyMelody's current server list.

### Broadcast codes (`0x8200`) and pushes

`0x8200` = `00 <count> <codes>`. Buds 4 `[CAPTURE]`: `00 09 01 02 03 04 08 0B F1 F2 F3`.
**Read it before deciding an event is unsupported**: silence usually means "not subscribed".

| Code | `0x0204` push | Subscribed |
|---|---|---|
| `01` | Battery (§7) | always |
| `02` | Wearing (§8) | always |
| `03` | ANC: `01 01` mode, `02 01` hold cycle, `04 01` Smart's level (§5) | always |
| `04` | Fit test result (§9) | when the bitmap has `0x0405` |
| `05` | Game mode (`GAME EVT`) `[CAPTURE]`: `05 <state>`; `05 xx xx <state>` on `062410` `062810` `063010` `063410` `[VENDOR]`. The push confirms a game-mode write about 1 s after the ack; a read sooner returns the old value `[CAPTURE 2026-10-24]` | — |
| `06` | Dual connection device list (§9) `[CAPTURE]` | — |
| `08` | Golden Sound test status (§9) | when the bitmap has `0x040D` |
| `0B` | Personalised ANC result (§9) | when the bitmap has `0x0412` |
| `0E` | Golden Sound ear scan result (§9) | pushed, not in the list |
| `F1` | Gesture fired (§6) `[CAPTURE]` | in the `0x8200` list |
| `F2` | Connected-devices info `[VENDOR]`, acked (§9): `<count>`, then per device the MAC (6 bytes, reversed), connect count (u16 LE), name length, name | no |
| `F3` | Content unknown. HeyMelody ignores it `[VENDOR]` | in the `0x8200` list |
| `F4` | Diagnostic JSON `[VENDOR]`, acked (§9) | pushed, not in the list |
| `F5` | Head gesture type `[VENDOR]` | — |
| `0A` | Zen-mode switch state `<state>` `[VENDOR]`. Not built (tone files come from the vendor's servers) | — |
| `0D` | Related-device info `[VENDOR]`: `<flags u16 LE>` then the fields whose flag is set, in this order: bit 0 capability `<u8>`, bit 1 version `<u8>`, bit 2 wear `<u8>`, bit 3 status `<u8>`, bit 4 name `<len> <bytes>`. Status bits: 0 phone has no active command, 1 connected by the related device, 2 HFP busy, 3 A2DP busy, 4 the other device's auto switch is closed. A field whose bytes are missing is absent. Not used | — |
| `0F` | Public event `[VENDOR]`: `<flags u16 LE> <data>`. The first match wins: bit 0 alert-sound volume `<v>` (the app updates the slider), bit 1 nothing, bit 2 handheld device `<mode> [<MAC 6, reversed>]` (mode `00` = automatic, no MAC; not used), bit 4 tap level `<v>` (the app updates the slider). Needs at least 1 data byte | — |
| `10` | One-shot state `[VENDOR]`: the voice-wakeup audio stream setup (flags, sample-rate type, format, channels, codec, frame size u16 LE, session state, VAD state). Decided against (voice wakeup) | — |
| `11` | Tone-change list `[VENDOR]`: tone records. Not built (tone files come from the vendor's servers) | — |
| `F6` | Head-motion try result: one byte `[VENDOR]` | — |
| `F7` | Voice-assist control type `[VENDOR]`: `<bits>`, bit 0 type 0, bit 1 type 1, bit 2 type 2 (highest set bit wins), bit 7 a support flag. Not built | — |
| `F8` | LE / BR link switch state `[VENDOR]`: `<bits>`, bit 0 state, bit 1 owner, bit 2 reset flag. Not built | — |
| `F9` | AI translation audio focus: one byte `[VENDOR]`; online feature, not built | — |

Codes `07` and `0C` have no handler in the vendor app. Code `09` goes to a separate parser for a list of records (not read, unused).

### Subscribing — `0x0205`

Payload **`<count> <ids...>`, count first** `[CAPTURE]`. The app asks for `01 02 03` (battery, wear,
ANC). It adds `04` / `08` / `0B` as above, **kept to the ids the buds offer in `0x8200`**
(`OpoProtocol.notifyIds`, `protocol::notify_ids`). HeyMelody registers exactly the offered list
`[VENDOR]`. On buds without `0x0205` in their bitmap it sends `0x0201 <id>` once per id instead.

- **Never `01 01 02 02`**: that is "count 1, battery only". The buds ACK it and never push wear.
- **Never drop `03` when the buds offer it**. On Buds 4, without it, ANC changes made on the buds send
  nothing at all.
- Nord Buds 3 Pro (issue #2) `[CAPTURE]` offers only `00 02 02 01` (wear, battery). Asked for
  `01 02 03 04 08 0B`, it sent no `0x8205`. It still pushed an ANC change once without `03`.
- OPPO Enco Buds2 (issue #5, id `060C12`, bitmap `00 9B 2C 50 80`) `[CAPTURE]` also offers `02 01`.
  It sends no `0x8205`. The cause of its link drop was the long `0x010D` list, not the subscription (§9).
- The `0x8205` ack: `01 02 01 00 02 00` and, for five ids, `01 05 01 00 02 00 03 00 04 00 08 00`
  (Buds 4): `01 <count>`, then `<id> 00` per id `[GUESS]`. Fewer ids than asked = part rejected.
  HeyMelody reads none of these bytes; the ack only ends its connect setup `[VENDOR]`.

---

## 5. ANC (noise control)

### Per-model bits `[VENDOR]`

Each model's `ancModes` in `assets/models.json` (`protocol/AncModes.kt`) is a tree of
`{mode, slot, variants}`. mode: `1` Off, `2` Transparency, `3` Light (weak),
`8` Medium, `4` Deep (strong), `5` noise reduction (a parent of levels, or alone), `7` Smart,
`10` Adaptive, `6` transparency with voice (not Adaptive).

- **SET** sends the mode's `slot` as the bit. A level sends its child's index.
- **A report**: the app looks it up in the same tree, parents first, then children.
- Tree shapes: Buds 4 style (with or without Adaptive `11`, with or without Medium). Legacy `NC 0,
  Off 1, Transparency 2` (9 models. **Buds 4's Off bytes turn ANC on there**). `Off 0, Transparency 1,
  Light 2, Deep 3` (Enco X, Buds Z2), plus `Smart 4` on Buds Pro. The app does not offer voice-transparency
  modes that need a per-bud support read (4 OPPO models).
- A model with no `ancModes` (76 of 140) gets no noise control. Before detection, the app uses the Buds 4
  tree.
- **realme models** `[VENDOR]` (realme Link 5.5.514): HeyMelody lists some realme ids with no modes.
  realme Link has their data. Its write and report tables are **the same** (unlike Buds 4). Off `01`,
  Transparency `02`, Light `04`, Deep (and plain ANC) `08`, Medium `10`, Smart `20`. As a tree: Off 0,
  Transparency 1, ANC 3 with children Light 2, Deep 3, Medium 4, Smart 5 (only the levels a model
  has. T200 / T300 / T500 / T200x / Wireless 3 have plain ANC, N1 / T310 no Smart). In `models.json`
  for 21 realme models, wired. The Air7 Pro's owner confirms it works (issue #8). The others are
  unverified. The Air7 Pro log's
  `0x810C 01 01` value `02` = Transparency.

### SET — `0x0404 01 01 <mask>` `[CAPTURE]`

`OpoProtocol.anc(bit)`: `bit / 8 + 1` mask bytes after `01 01`, little-endian. Buds 4:

| Mode | Bit | Payload |
|---|---|---|
| Off | 0 | `01 01 01` |
| ANC (last level) | 1 | `01 01 02` |
| Transparency | 2 | `01 01 04` |
| Deep | 4 | `01 01 10` |
| Medium | 5 | `01 01 20` |
| Light | 6 | `01 01 40` |
| Smart | 7 | `01 01 80` |
| Adaptive | **11** | `01 01 00 08` |

**Adaptive is bit 11 (mask `0x0800`), not 8.** Bit 8 gives `01 01 00 01`, a different mode. Take the
bit from the model's `slot`, never from a mode's position in a list.

### NOTIFY — push `03 01 01 <mode LE>` and the `0x810C 01 01` reply `[CAPTURE]`

The mode is 1-4 bytes, little endian, in the push and in the reply. Buds 4 sends 2 bytes.

**A different table from SET. They are not supposed to agree.** It is the same model tree, one level
down. Buds 4's Off has a child at 3. Transparency has one at 8.

| Raw (LE) | Mode |
|---|---|
| `0x0008` | Off |
| `0x0002` | ANC, generic |
| `0x0010` / `0x0020` / `0x0040` / `0x0080` | Deep / Medium / Light / Smart |
| `0x0100` | Transparency |
| `0x0200` | Transparency, voice enhance |
| `0x0800` | Adaptive |

- With no bud in an ear, a SET is refused: ack `0E` (status 14) `[CAPTURE]`. One bud in an ear is enough.
  Both apps then re-read `0x010C 01 01`, so the display shows the real mode, and say to put a bud in.
- The ANC stop reports **whichever level is active**. It is a bitmask, not an enum.
- Query: `0x010C 01 01` → `00 01 01 <mode LE>` (status, echo, value). Read on every connect, so a
  reconnect corrects the display.
- `AncEventParser` accepts a push only when bytes 1-2 are `01 01`. A hold-cycle write also raises a
  push `03 02 01 <mask>`: the hold cycle's new mask `[VENDOR]`. It is not a mode. Was: the app decoded
  it as one, and the display stuck on a false mode. Is: the app treats it as the hold-cycle mask.
- **Smart** `[CAPTURE]`: SET bit 7 is acked and pushed as `0x0080`. Right after, the buds pushed
  `03 04 01 20 00` and `03 04 01 40 00`.
- **The level Smart chose** `[VENDOR]`+`[CAPTURE]`: push `03 04 01 <mask LE>`, read `0x010C 04 01` →
  `00 04 01 <mask LE>`. Buds 4 answers the read with ANC Off too, and pushes right after a Smart SET. The lowest set bit is a SET bit: `0x20` Medium, `0x40` Light on Buds 4. The
  ANC label shows it ("ANC S·M"). Read on connect for models with Smart.

- Other reads and writes `[VENDOR]`, not seen on our buds: a level report `01 02 <level>`. The vendor app reads it. Our buds never send it. The app
  does not decode a mode mask `01 05 <mask>` `[GUESS]`. It does not send a level write `0x0404 01 02 <level>` `[GUESS]`.

### The hold's ANC cycle — `setSupportNoiseReduction`

The gesture table's `fn 0x08` only says "the hold cycles ANC". **Which modes it cycles is a separate
setting**; clearing `fn` to `0x00` does not stop the cycle `[CAPTURE]`.

| | Frame |
|---|---|
| Read | `0x010C 02 01` → `00 02 01 <mask LE>` `[CAPTURE]` (`02 03` / `02 04` on per-bud holds, §6) |
| Write | `0x0404 02 <type> <mask LE>`, type `01` shared, `03` left / `04` right on per-bud holds |

- The mask uses **the SET bits** `[CAPTURE]`: adding Adaptive in HeyMelody moved it `0x0007` →
  `0x0807`. Buds 4 bits: Off 0, ANC 1, Transparency 2, Adaptive 11 (same numbering as `OpoProtocol.anc`).
  Other models use their top-level modes' `slot` `[VENDOR]`.
- The mask is sent in as few bytes as needed, 1-4 `[VENDOR]`: `02 01 03` acked and read back as
  `0x0003` `[CAPTURE]`.
- A single-bit mask is valid (HeyMelody's minimum is one mode) `[CAPTURE]`. The app never sends 0.
- **Level bits are dropped silently**: `0x0021` was ACKed `00` and read back `0x0001` `[CAPTURE]`. The
  hold's ANC stop is always bit 1, the level last set by hand.
- One shared setting for both buds (type `01`) `[TESTED]`+`[CAPTURE]`.
- Bit 1 alone has never been cleared in a test (§12).

### Mistakes not to repeat

1. "ANC raises no event": it does (push `03`); we were not subscribed, and `UNATTR RX` hides `0x0204`
   (§10).
2. "SET and NOTIFY share bits": they do not. "Fixing" SET to the notify values sent wrong bits.
3. Adaptive as bit 8 (see SET).

---

## 6. Gestures

### Gesture table — read `0x0108`, write `0x0401` `[CAPTURE]`

Entry: 4 bytes `[dev, btn, act, fn]` `[CAPTURE]`. The read reply has a **2-byte header**
`<status 00> <count>` (`KeyFunctionParser.HEADER_SIZE = 2`): 74 bytes = 2 + 18 × 4.

- **`dev`**: `01` left, `02` right. `04` in a write = both buds (on-call rows).
- **`btn`**: `01` main gestures, `06` on-call group (`KeyFunctionParser.BUTTON_ON_CALL`).
- **`act`**: `01` single, `02` double, `03` triple, `04` hold, `05` slide, `06` long hold / "super long
  press" (the app confirms it by matching `F1` frames).

Buds 4 table (`act:fn`):
```
dev=01/btn=01[01:00 02:11 03:00 04:08 06:00 05:07]
dev=01/btn=06[02:00 03:00 06:00]
dev=02/btn=01[01:00 02:00 03:00 04:08 06:00 05:07]
dev=02/btn=06[02:00 03:00 06:00]
```

**Write is `0x0401`** → ack `0x8401 00`. **`0x0402` gets no reply at all.** A wrong command
number fails silently. Every write therefore re-reads the table and diffs it. A write that matches no slot
is refused loudly. The diff baseline is persisted (`QuickBudsKeyFnDiff`). An empty reply never
overwrites a good one.

**The table's shape changes per bud and between writes** `[CAPTURE]`. Slide once lived in
`btn 01` on the left and was split into unbound `btn 02` + `btn 03` on the right. Writing both halves
with the same `fn` folded them back into one `btn 01` slot (19 → 18 entries). So
`writeGestureBinding(side, ...)` writes **every slot that the table has for that (side, act)**, excluding
only `btn 06`. Never hardcode a button group or a count.

### `fn` values — measured `[CAPTURE]`, names `[VENDOR]`

| `fn` | Function | | `fn` | Function |
|---|---|---|---|---|
| `00` | none | | `0B` / `0C` | volume up / down |
| `01` | play/pause | | `0D` | switch devices |
| `02` | listening music (never shown) | | `11` | game mode |
| `03` | voice assistant | | `12` | zen mode |
| `05` / `06` | previous / next track | | `16` | collect music |
| `07` | volume (slide; resolves to `0B`/`0C`) | | `19` | AI summary |
| `08` | ANC cycle (hold) | | `1A` / `1B` | AI translation |
| `09` | favourite music | | `1C` / `1D` | decline / answer + end call |
| `0A` | switch track (slide; resolves to `05`/`06`) | | `20` | spy tap |

`00 01 03 05 06 07 08 0A 0B 0C 11 1C 1D` are measured on Buds 4. **Do not re-derive them.** On
"OnePlus Buds" and "OnePlus Buds Z" only (by Bluetooth name), previous / next are `04` / `05` `[VENDOR]`.

### Per-model gesture menus `[VENDOR]` (`GestureModel`)

Each model's `gestures` and `callGestures` in `assets/models.json`: one entry per row,
`{gesture, options, minSelected}`. `options` is a mask of the allowed actions:

| bit | option | | bit | option |
|---|---|---|---|---|
| 1 | voice assistant `03` | | 512 | none `00` |
| 4 | play/pause `01` | | 1024 | volume `07` |
| 8 / 16 | volume up / down `0B` / `0C` | | 2048 | switch track `0A` |
| 32 / 64 | previous / next `05` / `06` | | 4096 | switch devices `0D` |
| 128 | noise cycle `08` | | 8192 | game mode `11` |
| 256 | favourite music `09` | | 32768 / 65536 / 131072 | collect music / zen / AI summary |

Buds 4: `1:516 2:8807 3:3154531 5:3584`, hold `27` — matches HeyMelody's menus `[TESTED]`.

- `gesture` → `act`: 1-6 are themselves. 16/17/18 = acts 1/2/3 (stem-press models). 11, 20, 27 = the
  hold's ANC cycle on act 4 (options = the model's top-level noise modes). 7, 8, 12-15 are text-only
  rows.
- **callControl** (all `btn 06`, `dev 04`): 28/32 = act 1, 29/33/36 = act 2, 30/34 = act 3, 31/35 =
  act 6. `options` 524288 = answer/end `1D`, 262144 = decline `1C`, 131072 = AI summary. A row without
  none (512) has one fixed option and writes nothing. Buds 4 `[CAPTURE]`: double tap
  `0401 01 04 06 02 1D` / `…02 00`, long hold `0401 01 04 06 06 1C` / `…06 00`. One write lands on both
  buds, labels confirmed on a real call `[TESTED]`.
- **realme models** `[VENDOR]` (realme Link, issue #8), wired. The Air7 Pro's owner confirms them. The
  others are unverified.
  - Per bud: double tap, triple tap and hold, the same `fn` bytes as above, `dev` 1 / 2, `btn 01`.
  - A hold with the noise cycle is `holdMask` 8833. Its cycle is **one mask for both buds**: the app writes `0x0404 02 01 <mask>` and reads
    `0x010C 02 01`. The Air7 Pro log's `0A` = ANC + Transparency. So our own key `"sharedHold":1`
    keeps type 1 on a per-bud hold.
  - **Holding both buds** (Air 3S, T01, T110, T200 Lite, T200x): one
    entry `04 01 04 <fn>`, game mode `11` or none. The buds read it back as its own `dev 04` entry (realme Link reads
    it so. Buds 4 fans `dev 04` on-call writes out to both sides instead). Our own key `"bothHold":1`,
    shown as a gesture row. The app writes it like an on-call row. Not offered: volume on the noise-cycle hold
    (T500, Air8, T200x), AI options.
- **realme neckbands** `[VENDOR]` (Wireless 3 Neo, 5 ANC, 6, 6 ANC, 6 Neo), wired, unverified: one button
  written as the left bud, `01 01 <act> <fn>`. Act 1-4 (single, double, triple, hold). Our own key
  `"oneButton":1` hides Left / Right. The app does not offer the noise button of the two ANC ones (`01 04 01 08`, single
  press = the noise cycle).
- **Per-bud hold** (`holdMask`, 28 models): options 512 none, 128 noise cycle, 1 voice assistant,
  8192 game mode. Each bud written alone (`dev` 1/2, `act 04`). The cycle mask per bud with type 3 / 4,
  read with `0x010C 02 03` / `02 04`.
- **Minimum modes in the cycle:** `minSelected`, else 2 on OnePlus models and per-bud holds, else 1
  (Buds 4: 1).
- **Top-level ANC levels:** Buds Z2 and Enco X list strong (4) and weak (3) as their own cycle options.
  OnePlus Buds Pro (`060C14`) shows 3 / 4 / 7 as one "ANC" option. Its bit is the current level's, or
  Smart's (bit 4) outside ANC.

### `F1` push — a gesture fired `[CAPTURE]`

`0x0204` payload `F1 <side> <btn> <act> <fn> <context> <int16 options...>`.
Example, right hold: `AA 0D 00 00 04 02 FF 06 00 F1 02 01 04 08 03`.

- `act` and `fn` are the gesture table's pair. `fn` is the **effective** function (a slide reports
  `0B`/`0C` or `05`/`06`. A hold with `fn` cleared still reports `08`). A "modifier bits"
  reading does not fit.
- `act 00` frames (seen on a single tap) are some other event, not a binding `[GUESS]`. HeyMelody
  stores the frame and reads no field of it `[VENDOR]`.
- **Diagnostic only, never a control signal.** Read the effect from the ANC / game / wear pushes. One
  double tap can report twice. The app follows faithfully. Do not "fix" it.

---

## 7. Battery — `0x8106` and push `01`

`[CAPTURE]` Payload `<count>` then `<index> <raw>` pairs: `01` left, `02` right, `03` case.
Level = `raw & 0x7F`, charging = `raw & 0x80`. Example: `03 01 64 02 64 03 50`.
The query reply has a status byte `00` before the count. A count of 0, or a list that runs past the end, is no list. The parsers
ignore the message and clear nothing.

- The case and a bud inside it report only with the lid **open**. With it closed, `0x0106` returns
  only the bud outside (`01 01 5A`) and the buds push nothing `[CAPTURE]`. So the app does not show case charging,
  by decision. A part left out keeps its last level unless the "Hide old battery levels" setting is on.
- Pushes come every 1-6 min on the buds' own clock, not tied to `0x0500` `[CAPTURE]`.
- Level `0` means "not known": the realme Buds Air7 Pro sends case `03 00` with the lid closed (issue
  #9). The apps leave it out; HeyMelody hides a part at 0 `[VENDOR]`.

## 8. Wearing — `0x8109` and push `02`

`[CAPTURE]` Payload `<count>` then `<component> <status>` pairs; component `1` left, `2` right, `3` case.

The status is a set of bits `[VENDOR]`. Bit 0 means out of the case, bit 1 means in ear, bit 2 means lid open. Bit 3 is a flag that
HeyMelody calls "safe remind". Both apps clear bit 3, so only the codes below remain. Query replies and pushes both need at least one part. Every part must be `1`-`3`.

| Status | Meaning |
|---|---|
| `0` | in case, lid closed |
| `1`, `5` | out of ear (lid closed / open) |
| `3`, `7` | **in ear** (lid closed / open) |
| `4` | **in case**, lid open |

- **No in-ear sensor** `[CAPTURE]` (report #46, and the logs of the T200, T200x and Nord Buds 3r): these models answer `1` (out) for a bud in every state. They answer `0` or `4` when it sits in the case. The vendor's `wearDetection` flag does not tell. The Bullets Wireless Z3 and the Buds Ace lack it and report `3`. The app therefore shows `1` and `5` as in the ear, with no wear label, until the buds report `3` or `7` once. It remembers that per model.
- Query replies sometimes prepend a status byte. `WearingStatusParser` tries offsets 0 and 1
  `[CAPTURE]`.
- **Lid close** `[CAPTURE]`: ~1 s before the socket drops, pushes fall to 0, ending all-zero
  `01 00 02 00 03 00`. No lasting "closed" state exists. The app logs `Case closed` and skips the
  reconnect retries. Opening brings `04 04 04` and the case battery.

---

## 9. Features

### Feature switches — `0x0403 <id> <01/00>`, read with `0x010D`

`0x010D <count> <ids>` → `00 <count> [<id> <value>]...`. Only ids the firmware has come back
`[CAPTURE]`. The reply `0x8403` to a write repeats the seq. `00` accepted, `01` the buds lack the id,
`02` the buds have the id and refuse it now. Any other non-zero status is a refusal now `[CAPTURE]`.

- **The ids are per model** `[VENDOR]`. `models.json` `statusQuery` holds the list (85 HeyMelody
  models, from HeyMelody 116.9.0's built-in data). Buds 4: `05 04 0B 11 18 06 1B 1D 1C` (matches the
  capture). Enco Buds2: `05 06` `[TESTED]`.
  Checked against the HeyMelody 117.6.6 id rule (`PROTOCOL.md` section 11), all 85 listed models match it. Our lists leave out the
  ids of features we decided against (`22`-`24`, `3C` neck health, `3D` Gemini assistant). The app asks for wear detection `04` for every model
  with `wearDetection` 1 or 2. The Enco X4 and Buds Pro 4 have 2.
- **A longer list drops some links.** Was: one fixed 23-id list, which dropped the Enco Buds2 (issue
  #5). Is: the per-model list. `02 05 0D` kept that link up `[CAPTURE]`.
- A model without `statusQuery` (hand-added) gets the 23 ids, plus `1A` where it has `windMode`.
- **realme models** `[VENDOR]` (realme Link 5.8.811): its per-model screen lists name the rows a model shows.
  Our `statusQuery` for 30 realme models asks only for the ids of those rows: `04` smart play/pause, `06` game
  mode, `0B` hearing profile, `11` dual connection, `18` Hi-Res, `1B` 3D sound. Only the Air8 Pro, Air7 Pro,
  Air 5 Pro and Air6 Pro list a hearing profile. The T500 Pro does not (report #21), and its buds do not answer
  the tones. The Air7 Pro keeps its list: its owner confirmed it.
- A row whose id the model is not asked for does not show (HeyMelody parity: no Power saving or
  Personalised ANC on Buds 4).
- Parsed into `featureStates`, logged as `FEATURES:`. The buds' list wins over the model list while connected.

How HeyMelody builds the list `[VENDOR]`: `05`, then one id per `function` key, in this order. The value is
exactly `1` unless noted:

`04` wearDetection, `0B` hearingEnhancement(New), `0C` personalNoise (or
personalNoiseCompat.personalNoise), `0D` clickTakePic(New), `0F` zenMode > 0, `11` multiDevicesConnect
1/2, `09` vocalEnhance, `13` headSetSoundRecord, `18` highToneQuality 1/2, `17` longPowerMode, `15`
smartCall, `16` deviceLostRemind, `14` voiceWake 1/3, `19` voiceCommand 1/2, `06` gameMode or
gameModeList, `1B` spatialTypes, `1D` bassEngineSupport, `1C` controlAutoVolumeSupport, `1E`
collectLogs, `21` gameEqPkgList, (`1F` OPPO / OnePlus phones only, skipped), `22 23 24` spineHealth,
`27 28` gameSoundList or bitmap `0x0423`, `30` adaptiveVolume, `31` adaptiveEar, `32`
speechPerception, `34` meetingAssistant, `35` longPressVolume, `37` swiftPair, `38` hearingOptimize,
`39` incomingCallControl, `3B` headMotion, `3A` sleepDetection.

| Id | Switch | Notes |
|---|---|---|
| `04` | Auto play/pause (wear detection) | `[CAPTURE]` |
| `05` | ? | HeyMelody asks it first for every model, ignores the value and never writes it `[VENDOR]` |
| `06` | Game mode / low latency | the apps call it Low latency mode; all models, **except** `28` where the bitmap has `0x0423` (`gameModeId()`) `[VENDOR]` |
| `09` | Vocal enhancement | flag `voiceBoost`; realme Link's "enhance voice" row writes it `[VENDOR]` |
| `0B` | Golden Sound; the apps call it Hearing profile | see below `[CAPTURE]` |
| `0C` | Personalised ANC applied | the apps call it Personalised noise cancelling; see below |
| `0F` | Zen mode | asked on 6 models that set HeyMelody's `zenMode` field; QuickBuds does not build it `[VENDOR]` |
| `11` | Dual connection | see below `[CAPTURE]` |
| `13` | Sound record | HeyMelody's `headSetSoundRecord` field; in both default id lists `[VENDOR]` |
| `16` | Lost reminder | HeyMelody's `deviceLostRemind` field; written as `0x0403 16 <01/00>` `[VENDOR]` |
| `17` | Power saving | restarts the buds, see below `[CAPTURE]` |
| `18` | Hi-Res | the apps call it High-quality audio; quality switch, see below `[CAPTURE]` |
| `1A` | Wind noise reduction | realme Link's "anti wind noise" row writes `0x0403 1A <01/00>` `[VENDOR]`; asked in `0x010D` only for models with flag `windMode` (Air7 Pro, T310: its realme Link item list has the wind noise row, and report #46 says the vendor app shows it); no capture `[GUESS]` that every `windMode` model answers |
| `1B` | Spatial sound | the apps call it 3D audio; on models without `0x012A` `[CAPTURE]`. The Enco Buds3 Pro lists `1B` in its status reply, but the bitmap has no `0x012A` and the sound does not change (reports #60 #61) `[CAPTURE]` |
| `1C` | Smart volume | flag `autoVolume` |
| `1D` | BassWave | the apps call it Bass boost; `[CAPTURE]`; realme Link's "dynamic sound mode" row (a switch and three bars) writes the same id `[VENDOR]` |
| `1E` | Log collection | HeyMelody's `collectLogs` field; asked on 16 models `[VENDOR]` |
| `27` | Game sound effects | HeyMelody's `gameSoundList` field |
| `28` | Game mode, where the model has game sound | HeyMelody's `gameSoundList` field or the `0x0423` bitmap; both default id lists ask it next to `06` `[VENDOR]` |
| `30` | Adaptive volume | |
| `31` | Adaptive left / right ear | |
| `32` | Conversation awareness | HeyMelody turns it off against voice commands (`19`) |
| `35` | Touch-and-hold volume | flag `holdVolume` |
| `37` | Windows Swift Pair | flag `fastPair` |
| `38` | Adaptive sound (ear canal and fit) | HeyMelody asks first (battery) |
| `3A` | Pause when asleep | |
| `3B` | Head gestures | mapping below |

Ids `09`-`3B` without a `[CAPTURE]` are `[VENDOR]`, wired, unverified on buds. A row shows where
`0x810D` lists the id, or a hand-picked model has the flag.

The project decided against these `[TESTED]`: voice wakeup `14`, voice commands `19` and incoming-call voice control
`39`. It also decided against neck health `22`-`24` (it needs OPPO's Health app) and meeting assistant `34`.

- **Power saving `17`** `[CAPTURE]`: either write restarts the buds. The link returns about 12 s
  later and `0x810D` shows the new value. ANC, gestures, wear, codec (LHDC V5 48 kHz / 24-bit) and link
  parameters do not change. The buds do not bring phone audio back; the app asks for it. HeyMelody
  does not show this row (China-market feature `[VENDOR]`).
- **Head gesture mapping** `[VENDOR]`: `0x0431 00` = nod answers / shake declines, `01` = the reverse.
  Read `0x0134` → `00 <type>`, pushed as `F5 <type>`. Bitmap bit 63. Was: "HeyMelody parses only the
  push". Is: it parses both.
- **Game sound type** `[VENDOR]`: `0x0423 <type> 01`, Off (`0`) included. Read `0x012B` →
  `00 <selected> <count> <types>`. Types: `0` Off, `1` a Chinese game, `3` shooting games. Offered:
  the model's `gameSoundList` types the buds also list.

### Personalised ANC — `0x0412` `[VENDOR]`, wired, unverified

Model flag `personalAnc` (11 models), bitmap `0x0412` (bit 26 with `0x011A`). Buds 4 lacks it.

| Step | Frame |
|---|---|
| Stored result? | `0x011A` → `811A 00 <exist>`, non-zero = a result exists |
| Test / use stored / cancel | `0x0412 01` / `02` / `03`; ack status `15` = another device busy |
| Result | push `0B <r>`: `0` applied, `1` too quiet, `2` poor fit, `3` wind, `4` movement, `5` audio or call |
| Off | `0x0403 0C 00`; the buds set `0C` on themselves when a result applies |

Both buds in an ear; 5 s timeout for `0x811A`, 15 s for a result; cancel when the dialog closes.

### Tap sensitivity — `0x042D` `[VENDOR]`, wired, unverified

Flag `tapLevel`. `0x042D <level>` 1..5 (lower = taps trigger more easily). Read `0x0133` →
`00 <level> <default>`. Warn below the default; offer a reset.
A change on the buds also arrives as push `0F 10 00 <level>` (§4, event `0F`, flag bit 4). It has no default byte;
the app keeps the default from the last read `[VENDOR]`, wired, unverified.

### Golden Sound (Hearing profile) `[VENDOR]`+`[CAPTURE]`

On / off: `0x0403 0B 01/00` `[CAPTURE]`; it applies the record stored on the buds. The buds hold only
the active record; the phone keeps the list. A record: 4-byte id (big-endian, chosen by the phone),
12 hearing values, 168 bytes of ear-scan data. A hearing value is 3 bytes
`<side 01 L / 02 R> <freq 01..06> <value, signed>`. Every frame below matches a HeyMelody capture.

| Step | Frames |
|---|---|
| Read active record | `0x0115` → `00 03 0C <12 values> <id> <name ASCII>`; `0x011E` → `00 03 A8 00 <168 scan> <id>` |
| Ear scan | `0x040D 04 01 <id>` start, `04 00 <id>` stop; result pushed ~8 s later: `0E 03 A8 00 <168> <id>` |
| Hearing test | `0x040D 02 01` start, `02 00` stop |
| Tone | `0x040E 03 01 <side> <freq> <value>`; `0x040E 04` stops it, sent before each new level |
| Apply | `0x0411 01 01 01 00 <desc>` (when the record has a description id; `0B` in the capture), `0x040E 03 0C <12 values> <id> <name>` (name = date, "2026/09/29 01:53"), `0x0415 03 A8 00 <scan> <id>` (when there is a scan), then `0x0403 0B 01` |
| Clear | `0x040E 02` + `0x0415 02`; after the test `0x040E 01 00 00000000` + `0x0415 01 00 00 00000000` |
| Status push | `08 <2 test / 4 scan> <s>`: 1/3 audio playing, 2/4 resumed, 5 a bud out, 6 back in, 7 timed out |
| Filters | `0x0116 0C <12 values> <id>` → `00 <id> 04 <count LE> <packet> <type> <floats>`; `0x011F A8 00 <scan> <id>` → `00 <id> 04 <rate LE 44100> <count LE> <packet> <floats>`. Floats LE, left ear then right, biquads `a0 a1 a2 b0 b1 b2` (60 = 5 per ear, 72 = 6) |

`0x040F 01` seen in the capture is an unrelated camera feature.

- **Slider:** 25 stops `-120 -88 -55 -52 -49 -45 -41 -38 -35 -30 -25 -22 -19 -15 -11 -8 -5 -1 3 5 7 10
  13 15 17`. Each move plays a tone (at most every 300 ms), starting near `-30`. The saved value is the stop
  snapped to the nearest of `-55 -49 -41 -35 -25 -19 -11 -5 3 7 13 17`. A loudness warning shows once at ≥ 10.
- **Radar graph:** per ear, sum the hearing and ear-scan biquads in dB (`20 log10 |B/A|`, 44100 Hz
  default). Read at 80, 10000, 4800, 2400, 1200, 250 Hz (80 at the top, clockwise). Radius =
  `max(2, -|dB| × 10 / scale + 10)`, scales 7.5, 15, 15, 12.5, 12.5, 7.5. 10 = no change.
- Ear scan only where `models.json` has `"hearingScan":1`.
- The OPPO Enco Air5 Pro refuses an ear-scan start (`0x040D 04 01 <id>`) with status `01` on Windows
  (6 times, report #44). The cause is unknown; the wear state is a guess `[CAPTURE]`.

### Fit test — `0x0405` `[VENDOR]`+`[CAPTURE]`

`0x0405 01` start / `00` stop, ack `00`. Result push `04 <dev> <s> <dev> <s>` (dev `01` L / `02` R):
`1` good, `0` average, `6` poor, else error. Buds 4: ~8.6 s → `04 01 01 02 01`. Both buds in an ear,
15 s timeout, stop when the sheet closes. Needs event `04` subscribed (§4).

### Spatial sound and Hi-Res `[CAPTURE]`+`[VENDOR]`

- **Two ways** `[VENDOR]`. If the bitmap has `0x012A` (bit 47), the app writes `0x0422 <type>`. The types are `0` off, `1` fixed
  and `2` head tracking. It reads `0x012A` → `00 <type>`. The buds push `0x0510 <type>`. Else the app uses feature `1B`.
  Head tracking only where `spatialModes` has `2` (Buds Pro 2, Buds Pro 3, Enco X3). `0x0422`
  is wired, unverified.
- **Spatial and Hi-Res are mutually exclusive.** HeyMelody warns, then writes this. Spatial on: `1B 01`, then
  `18 00`. Hi-Res on: `1B 00`, then `18 01`.
- **An accepted `18` write restarts the link**. The buds reconnect ~4 s later. A refused write drops nothing.
- **`18` and `0x8114` are per link** `[CAPTURE]`: with dual connection, each host reads its own value.
  Buds 4: phone `18=1`, LHDC V5; PC `18=0`, AAC, at the same time.
- **realme models** `[VENDOR]` (realme Link, issue #8): 3D sound is feature `1B` alone. Hi-Res is `18`
  alone, whatever the bitmap says. realme Link never writes one with the other (on some models it only
  warns). The Air7 Pro lists `0x012A` / `0x0422` but leaves `0x012A` unanswered. Our own key
  `"spatialToggle":1` (realme models with 3D sound). Wired. It works on the Air7 Pro (its owner, issue #9). It is
  unverified on the others.
- **3D audio source `0x041E`** `[VENDOR]` (bitmap bit 43, with `0xEF03`): `00` the buds render 3D audio,
  `01` the phone does. HeyMelody sends only `01`, when the phone's own spatializer takes over. realme
  Link sends `00` on every connect (no realme model prefers the phone). The apps do the same for
  `spatialToggle` models. Issue #9's "`1B` changes nothing" was a false alarm: the buds play no sound
  on the switch.
- **realme Link's `0x0403` ids** `[VENDOR]`: dynamic sound mode `1D`, enhance voice `09`, wind noise `1A`, game mode `06`,
  wear `04`, dual `11`, Hi-Res `18`, 3D sound `1B`. A second "enhance voice" setter writes `0C` (ours: personalised ANC
  applied. Do not assume they match). It reports power saving as `05` (ours `17` from HeyMelody: unresolved). The
  realme items STRENGTHEN_VOICE, WIND_NOISE and DYNAMIC_SOUND_EFFECT map to `09`, `1A` and `1D`. No device test exists.
- **`18` is a quality switch, not a codec switch** `[CAPTURE]`: LHDC V5 either way. On: 44.1-192 kHz,
  400 kbps cap. Off: 44.1 / 48 kHz, 256 kbps. The phone chose 48 kHz / 24-bit both times. No LDAC on
  Buds 4.
- **The codec decides whether the buds take the write** `[TESTED 2026-10-24]`. On a model with a codec list
  (`0x0123`), the buds accept `18 01` (`00`) while the phone plays LDAC or LHDC. They refuse it (`02`) while it
  plays AAC or SBC. The phone's Bluetooth settings choose the codec. The switch raises the bitrate and
  the sampling rate only. Read `0x0114` first. Older models accept `18` on AAC (realme Buds Air 5 Pro #4,
  T500 Pro #21, Air7 Pro #27, OnePlus Nord Buds 3 Pro #50). The Nord Buds 3 Pro offers only SBC and AAC.
  The switch turns on, but the sound does not change (report #40). Report #33's phone played AAC, so the
  newer rule refuses those writes. The link stays up.

### Codec picker (`hiRes` models) `[VENDOR]`, wired

Models with `hiRes` (Enco X2, Enco Air3 Pro, Enco Free3) and `0x0123` in the bitmap; replaces
the Hi-Res switch.

- Codecs: `1` SBC, `2` AAC, `3` LDAC, `4` aptX, `5` aptX HD, `6` aptX Adaptive, `7` LHDC, `8` LHDC V5.
  Shown in the order 8, 7, 3, 6, 5, 4, 2, 1; drop 7 when 8 is also offered.
- `0x0123` → `00 <u16 LE mask>`, bit k = codec k+1. `0x0114` → `00 <codec>`.
- Write `0x041A <codec> <hiRes> 00`, `hiRes` only with codec 3 or 8 (else `0`). The buds restart; the
  app warns and asks for phone audio on the reconnect.
- Buds 4 `[CAPTURE]`: `0x8114` = `00 08` (LHDC V5, correct). `0x8123` = `00 46 01`, which decodes to
  AAC, LDAC and LHDC; the maker lists LHDC 5.0, AAC and SBC only. The mask is trusted only on `hiRes`
  models.

### Equalizer `[CAPTURE]`+`[VENDOR]`

```
0x0406 <id>                          select built-in                 ack 8406 00
push 0x0504 <id>                     EQ changed
0x010F → 810F 00 <id>                current EQ (Buds 4: 00-02 built-in, 04-06 custom)
0x0122 → 8122 00 <count> <presets>   custom list
0x0418 <action> FA 06 <id> <nameLen> <name> <bands> [freq u16 LE, gain s8]...   ack 8418 00 <id>
0x0403 1D 01/00                      BassWave on / off
0x041B FB 05 <level>                 BassWave level; read 0x0124 → 00 FB 05 <level>
```

- Custom preset: `<selected flag> FA 06 <id> <nameLen> <name> <band count> <bands>`. `FA 06` = gain
  range -6..+6. `FB 05` = -5..+5.
- `0x0418` actions:
  - `01` create. Send id `00`. The buds assign it and return it in the ack.
  - `02` save and select. Send the whole preset, every time, also on every drag step and rename.
  - `03` delete. The selection falls back to `00`.
  **Ids are renumbered after create / delete. Always re-read `0x0122`.**
- **Built-ins differ per model** `[VENDOR]`: `eqPresets` lists `{mode, slot}`;
  `slot` is the `0x0406` id. Buds 4: 0 Balanced, 1 Clear Vocals, 2 Bass. Nord Buds 2r: 0, 1
  Bold, 2. Types 1-4 are named differently where `eqKind` is `2` (or Enco R / Air2). 41 of 143
  models have none.
- **realme** `[VENDOR]` realme Link: 28 realme models have four built-ins, not in HeyMelody's list.
  `0x0406` ids: 0 Pure Raw, 1 Deep Bass, 2 Pure Voice, 3 Pure Bass. `models.json` gives them our own
  types 101-104 (Pure Raw, Deep Bass, Pure Voice, Pure Bass), `eqKind` 1. T110 and T01 have three
  other presets with unknown ids: no EQ until a capture. Firmware-gated: use `eqPresetsCompat`, else `eqPresetsByFirmware`, else
  `eqPresets`. Drop entries whose `minFirmware` is above the buds' firmware (the lower non-zero
  of left and right, `138` on Buds 4, unreadable = 0). Stable sort by `order` (0 if absent).
- **What a model shows** `[VENDOR]`: EQ row where `eqKind` is 1-4 or custom presets exist. Custom
  presets need `customEq` and `0x0418` in the bitmap. BassWave needs `bassEngine` and
  `0x041B`. New presets use `customEqBands` bands (10 bands `31 62 125 250 500 1k 2k 4k 8k 16k`, 10
  models), else 6 bands `62 250 1k 4k 8k 16k`. `customEqLimit` caps presets (default 3). EQ reads go out
  only where the bitmap lists them.

### Find my earbuds — `0x0400` `[CAPTURE]`

`0x0400 01` / `00`, ack `8400 00`. Rings both buds; no side byte. Warn when the buds are in the ears.

### Alert-sound volume — `0x0427` `[CAPTURE]`

`0x0427 <level>` 1..10, ack `8427 00 <level>`; read `0x0130` → `00 <level>`. The app sends on slider
release only (each write plays a prompt).

Smart auto-pause (pause only when both buds leave) is the app's own, not firmware: `BudsService` sends
a media pause key. It and switch `04` are mutually exclusive in the UI.

### Dual connection — `11`, device list `0x0112` `[CAPTURE]`+`[VENDOR]`

```
0x0112 → 8112 00 <count> <entries>        push 0x0204 06 <count> <entries>
entry: <MAC, 6 bytes reversed> <len> <state> <flags> <nameLen> <name UTF-8>
```

- `len` counts state, flags, nameLen and name. State `02` connected, `00` not. A dropped device stays
  listed. Flags: bit 0 this phone, bit 1 main audio device, bit 2 playing, bits 3-5 device type.
- Toggle: `0x0403 11 00/01`, then `0x0413 08 00 01` after off / `08 00 00` after on (also sent when the
  screen opens). Replayed verbatim.
- `0x0413` `[VENDOR]`: the phone's state for switching between devices. `<flags> 00`, then one byte for each set
  flag, in bit order. `01` protocol version, `02` screen on, `04` held by hand, `08` auto switch on, `10`
  music playing. So `08 00 01` = auto switch on.
- **Device manager** `[VENDOR]`, wired, unverified: models whose `dualDevice` lists it (11;
  bitmap bit 59 `0x0429` / `0x0132`, Buds 4 lacks it). `0x0429 01 <MAC>` connect, `02 <MAC>`
  disconnect (MAC in written order, the reverse of the list). `03 <MAC>` unpair (unused). Preferred
  device: `0x0429 04 00` automatic, `04 01 <MAC>`; read `0x0132 02` → `00 02 <00 | 01 <MAC>>`.

### Requests from the buds — answered as HeyMelody does `[VENDOR]`

Some firmwares wait for these answers and reset the link when none comes. Nord Buds 3 Pro
(`064414`, issue #2) `[CAPTURE]` sends `0x0500` with a real Seq and `00 00 00 00`. It sends the request again, then stops
answering queries. It resets the link 3-5 s after connect, every time, if no answer comes. Buds 4
sends `0x0500` empty and works either way. Both apps answer from one function (`Replies.answerFor`,
`protocol::answer_for`). Every answer echoes the request's Seq and sets `0x8000` on its cmd.

| From the buds | Answer | Meaning |
|---|---|---|
| `0x0500` | `00 <Unix seconds u32 LE>` | Time request, sent on connect with `0x0501` |
| `0x050C` | `00 01 00` | Last byte: is the foreground app on a list the buds hold (no) |
| `0x051C` | `00 00 00` | The app's capabilities; middle byte = fast discovery (off) |
| `0x051D` | `01` | The phone's spatial mode. HeyMelody answers it later with the mode, `01` only for bad data; we always send `01` |
| `0x0204` push `F2` | `00 F2` | Connected-devices info (HeyMelody's status 00 = parsed) |
| `0x0204` push `F4` | `00 F4 {"cmd":<its cmd>}` | Diagnostic JSON; `01 F4` if it does not parse. Buds 4 sends these with Seq `FF` |
| a cmd outside groups `01`-`05`, `0F` | `01` | Unknown request |

- HeyMelody does not answer any other push. This includes battery, wear, ANC and `06`.
  `0x0501` and `0x0504` get no answer.
- HeyMelody adds `<zone offset seconds s32 LE>` to the time answer for models whose model list flags
  `utcTimeZone`. The model list flags none, so the 4-byte form goes out for every model. A newer list may flag some.
- Buds 4 `[CAPTURE]`: `0x0500` answered within 5 ms, link unchanged.

### Online features — not built `[VENDOR]`

QuickBuds talks to the network only to check GitHub for updates and to send reports to the Worker. These
commands belong to features that need the vendor's servers or account. The app has no UI for them:
`0x0425` AI summary type, `0x0426` AI prompt sound, `0x0428` AI translation state, `0x041C` account key
(`01 01 <len> <key>` binds, `02` clears), `0x0410` and `0x0417` tone and zen-mode file checks, and the
`0x03xx` firmware upgrade. Push `F9` belongs to the same set.

Also not built, by decision: the camera state and shutter (`0x040F`, `0x0501`, `0x0502`, feature `0D`) and the spine-health range `0x0421`. Not built: the multi-conversation
switch `0x042E` `[GUESS]`, the LE Audio action `0x0424`, the voice-wakeup stream (push `10`) and the zen-mode state (push `0A`).

Some commands share a bitmap bit with these, and no QuickBuds feature reads them. They are unread. The queries are `0x0117`, `0x0125`-`0x0127`, `0x0129`, `0x012E`, `0x0135`, `0x0136`. The writes are `0x0408`, `0x0414`, `0x0419`, `0x041D`, `0x041F`, `0x042B`, `0x042C`, `0x0430`, `0x0432`-`0x0435`. Their layouts and meanings are `[GUESS]`.
QuickBuds never sends them.

---

## 10. Debugging — log lines

| Line | Meaning |
|---|---|
| `RX:` / `TX[...]:` | Raw hex |
| `BTN EVT:` | `F1` push (§6) |
| `ANC EVT:` / `GAME EVT:` | ANC / game mode push |
| `WEAR EVT:` / `WEAR QRY:` | Wear push / reply |
| `KEYFN:` | Gesture table, grouped per `dev/btn` |
| `FEATURES:` | `0x810D` reply |
| `UNATTR RX:` | A frame we do not decode |
| `ancFlush=` | Whether an ANC write just went out |

- The gesture table prints in three places, all ending `RAW=[...]`: the `RX:` line, `KEYFN:`, and
  `LogDecoder`.
- **`UNATTR RX` never shows `0x0204`** (`noteUnattributed()` excludes it). An undecoded push prints
  nothing there. Dev tools' decoder does show it (`Unknown active report`, in amber). Change the exclusion
  first if a capture needs these.

---

## 11. Method — learning a new command

1. Check HeyMelody and realme Link first.
2. Check the `0x8100` bitmap and the `0x8200` codes: does the firmware claim it?
3. Subscribe to the event, or silence looks like absence.
4. Capture from a known state; the developer says what he did, in order
   ([PACKET-CAPTURE.md](./PACKET-CAPTURE.md)).
5. **Never guess a payload or an enum.** Log the raw value; let the device name it. Confirming a read
   is cheap; a guessed write fails silently.
6. **Read every write back.** An ack of `00` is not proof.
7. **Do not refute a theory with a sample that may be a different kind of event.**

---

## 12. Open questions

- Hold cycle mask: bit 1 alone has never been cleared (§5).
- Push `F1` `act 00`; `F3` (HeyMelody ignores both).
- The recurring `F1` family `AA 0D 00 00 04 02 FF 06 00 F1 01 01 XX YY 02`, and `02 01 08 0C 02` /
  `02 01 07 0B 02` (non-multiples of ten; maybe a fine battery field). **Do not guess these from a
  few samples.**
- `0x0404` with a level form `01 02 <level>` `[GUESS]`. Unverified.
- Switch `05`: HeyMelody never uses its value.
- Other models' ANC bits, gesture writes and `[VENDOR]` features: unverified until an owner reads a
  write back.
