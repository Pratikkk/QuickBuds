# PROTOCOL-NEXT.md — the earbud wire format, rebuilt

Draft for the owned protocol layer (the "new protocol" setting). It replaces [PROTOCOL.md](./PROTOCOL.md) when
that layer becomes the default. Until then PROTOCOL.md describes the shipping code.
Each fact comes from a capture, from our own buds, or from the vendor apps' behaviour (studied for
interoperability only). The code is written from this file, not from the vendor apps.

**Evidence tags:** `[VENDOR]` the vendor app's behaviour · `[CAPTURE]` seen in a capture on our own buds ·
`[TESTED]` checked by hand on our own buds · `[GUESS]` unverified. Never promote a claim without evidence.

**Status of this draft:** sections 1 to 5 are complete. Section 6 lists what moves over from PROTOCOL.md.

---

## 1. Transport

Classic Bluetooth RFCOMM (SPP). One long-lived socket; the buds push events at any time.

| UUID | Use |
|---|---|
| `0000079A-D102-11E1-9B23-00025B00A5A5` | Tried first `[CAPTURE]` |
| `00001107-D102-11E1-9B23-00025B00A5A5` | Tried second; kept for other models |

A case lid close ends the socket.

## 2. Link layer

One frame carries one message, or one slice of a message.

```
AA  <Len>  <Slice> 00  [<Fsn>]  <Message or slice of it>
```

| Field | Size | Meaning |
|---|---|---|
| `AA` | 1 | Frame start |
| Len | 1-2 | LEB128: 7 bits per byte, low bits first, bit 7 set = more bytes. Counts every byte after itself `[CAPTURE]` |
| Slice | 1 | `00` whole message, `01` first slice, `02` middle slice, `03` last slice `[VENDOR]` |
| `00` | 1 | Always zero |
| Fsn | 1 | Slice counter, from `00`. Present in every slice, absent in a whole message `[VENDOR]` |

- Whole message: `Len = 2 + message length`. Slice: `Len = 3 + slice length`.
- The message is `Cmd (2, little endian)  Seq (1)  PayLen (2, little endian)  Payload`. Its header is 5 bytes.
- No checksum.
- The receiver reads a byte stream, not reads. It skips bytes up to `AA`, reads `Len`, and waits for `Len` more
  bytes. A `Len` below 2 or above 4096 drops the `AA` and searches on. (The vendor app splits on read boundaries
  instead; both give the same messages for well-formed input.)
- The receiver joins slices in order and parses the message when `Slice` is `03`. A declared `PayLen` larger
  than the bytes present cuts the payload to the bytes present `[VENDOR]`.
- The sender splits a message when it exceeds the frame budget. The budget is 660 bytes by default. The reply
  to `0x0107` (`00 <u16 LE>`) can lower it `[VENDOR]`. Some bulk transfers use a larger budget; QuickBuds sends none.
- One write at a time. Wait for the write to finish (5 s limit) and pause a model-specific interval between slices.

### Seq and replies

- `Seq` `01`-`FE`: the sender picks it; the reply repeats it.
- `Seq` `FF`: a push from the buds `[CAPTURE]`.
- Reply command = request command with bit 15 set (`0x0106` → `0x8106`) `[CAPTURE]`. A write reply carries a
  status byte, `00` = accepted. Accepted does not mean applied: read the state back.
- A reply is matched on `(Seq, Cmd & 0x7FFF)` `[VENDOR]`.

## 3. Command classes

| Range | Class |
|---|---|
| `0x01xx` | Query, reply `0x81xx` |
| `0x02xx` | Event registration and pushes (`0x0204` push) |
| `0x03xx` | Firmware upgrade (not used) |
| `0x04xx` | Write, reply `0x84xx` |
| `0x05xx` | Requests from the buds |
| `0x0Fxx` | Private channel (not used) |

## 4. Command bitmap

The reply to `0x0100` is `00` and then bytes read least significant bit first. Bit n, when set, enables the commands
in the table. A command with no bit is refused by the vendor app and never sent. Always enabled: `0x0100`-`0x0104`,
`0x0106`, `0x010B`, `0x010D`, `0x0F00`, `0x0F03`, `0x0F04`. The table has 69 bits; bits past the buds' bitmap length
are off `[VENDOR]`. Buds 4 `[CAPTURE]`: `00 FF 77 5A EA 67 0E 20 07`.

| Bit | Commands |
|---|---|
| 0 | 0x0105 |
| 1 | 0x0106 |
| 2 | 0x0107 |
| 3 | 0x0108 0x0401 0x0416 |
| 4 | 0x0109 |
| 5 | 0x0400 |
| 6 | 0x0402 |
| 7 | 0x0403 |
| 8 | 0x010C 0x0404 |
| 9 | 0x0405 |
| 10 | 0x0406 0x010F |
| 11 | 0x0407 |
| 12 | - |
| 13 | 0x0408 |
| 14 | 0x0409 |
| 15 | - |
| 16 | - |
| 17 | 0x0114 |
| 18 | - |
| 19 | 0x040E 0x040D 0x0115 0x0116 |
| 20 | 0x040F |
| 21 | 0x0410 0x0119 |
| 22 | 0x0205 |
| 23 | 0x0F00 |
| 24 | - |
| 25 | 0x0118 0x0411 |
| 26 | 0x011A 0x0412 |
| 27 | 0x011C 0x0413 |
| 28 | - |
| 29 | 0x0112 0x040B |
| 30 | 0x011E 0x011F 0x0415 |
| 31 | 0x040D |
| 32 | - |
| 33 | 0x0121 0x0417 |
| 34 | 0x0122 0x0418 |
| 35 | 0x0419 |
| 36 | 0x011D 0x0414 |
| 37 | 0x0123 0x041A |
| 38 | 0x0124 0x041B |
| 39 | 0x0125 0x041C 0x0127 0x041D 0x041F |
| 40 | 0x0421 0x0023 0x0024 0x0022 0x0126 0x0129 |
| 41 | 0xEF01 |
| 42 | 0xEF02 |
| 43 | 0xEF03 0x041E |
| 44 | 0x0420 |
| 45 | 0x001C |
| 46 | - |
| 47 | 0x0422 0x012A |
| 48 | 0xEF04 |
| 49 | 0x0423 0x012B |
| 50 | - |
| 51 | 0x0424 |
| 52 | 0xEF06 |
| 53 | - |
| 54 | - |
| 55 | 0x0425 0x012E 0x0426 |
| 56 | 0x012F |
| 57 | 0x0427 0x0130 |
| 58 | 0x0131 0x0428 |
| 59 | 0x0429 0x0132 |
| 60 | 0x0014 |
| 61 | 0x042D 0x0133 |
| 62 | 0x042E |
| 63 | 0xEF07 |
| 64 | 0xEF08 0x0136 0x0433 |
| 65 | 0xEF09 |
| 66 | 0x0431 0x0134 |
| 67 | 0x0432 0x0135 0x0435 |
| 68 | 0x0117 |

## 5. Pushes — `0x0204`

`payload[0]` is the event code. `Seq` is `FF`. "After the code" below means the bytes that follow `payload[0]`.
A known code with too few bytes is malformed and dropped. An unknown code is kept as raw bytes.

| Code | Event | Layout after the code |
|---|---|---|
| `01` | Battery | `<count>` then `<part> <raw>` pairs (section 8) |
| `02` | Wearing | `<count>` then `<part> <state>` pairs (section 8) |
| `03` | ANC | `01 01 <mode LE, 1-4 bytes>` current mode; `02 <kind> <mask LE>` hold-cycle mask; `04 01 <mask LE>` the level Smart chose |
| `04` | Fit test | `<dev> <result>` per bud; `dev` `01` left, `02` right. A bud missing from the list has no result |
| `05` | Game mode | `<state>`; on four product ids (`062410` `062810` `063010` `063410`) `<x> <x> <state>` |
| `06` | Dual connection devices | `<count>` then devices (section 14) |
| `08` | Hearing test status | `<kind> <status>`; kind `02` hearing test, `04` ear scan |
| `0A` | Zen-mode switch state | `<state>` `[VENDOR]`. QuickBuds has no zen-mode feature (needs the vendor's tone files) |
| `0B` | Personalised ANC result | `<result>` |
| `0D` | Related-device info | `<flags u16 LE>` then the fields whose flag is set, in this order: bit 0 capability `<u8>`, bit 1 version `<u8>`, bit 2 wear `<u8>`, bit 3 status `<u8>`, bit 4 name `<len> <bytes>` `[VENDOR]`. Status bits: 0 phone has no active command, 1 connected by the related device, 2 HFP busy, 3 A2DP busy, 4 the other device's auto switch is closed. A field whose bytes are missing is absent |
| `0E` | Ear scan result | `<kind> <len u16 LE> <data> <id u32 BE>`; `len` is not 0 (section 16) |
| `0F` | Public event | `<flags u16 LE> <data>`. The first match wins: bit 0 alert-sound volume `<v>`; bit 1 nothing; bit 2 handheld device `<mode> [<MAC 6, reversed>]`, mode `00` = automatic, no MAC; bit 4 tap level `<v>` `[VENDOR]`. Needs at least 1 data byte; a MAC needs 6 more |
| `10` | One-shot state | Voice-wakeup audio stream setup `[VENDOR]`: flags, sample-rate type, format, channels, codec, frame size u16 LE, session state, VAD state. Not built (voice wakeup is decided against) |
| `11` | Tone-change list | List of tone records `[VENDOR]`. Not built (tone files come from the vendor's servers) |
| `F1` | Gesture fired | `<side> <button> <act> <fn> <context> <int16 LE options...>`; needs 5 bytes (section 10) |
| `F2` | Connected-devices info (answered) | `<count>` then `<MAC 6, reversed> <times u16 LE> <nameLen> <name>` |
| `F4` | Diagnostic JSON (answered) | UTF-8 JSON |
| `F5` | Head gesture type | `<type>` |
| `F6` | Head-motion try result | `<v>` |
| `F7` | Voice-assist control type | `<bits>`: bit 0 type 0, bit 1 type 1, bit 2 type 2 (highest set bit wins), bit 7 a support flag `[VENDOR]` |
| `F8` | LE / BR link switch state | `<bits>`: bit 0 state, bit 1 owner, bit 2 reset flag `[VENDOR]` |
| `F9` | AI translation audio focus | `<v>`; online feature, not built |

Codes `07` and `0C` have no handler; `09` goes to a separate parser for a list of records (not read, unused). `F3` is
offered by the buds and ignored `[VENDOR]`.

Registration: `0x0200` reads which codes the buds offer (`00 <count> <codes>`). `0x0205 <count> <codes...>`
subscribes. Buds without `0x0205` in the bitmap take `0x0201 <code>` once per code `[VENDOR]`.

## 6. Connection

Init sequence `[CAPTURE]` (Buds 4, both apps): wait 300 ms, then one message every 200 ms.

```
0x0100 bitmap → 0x0103 product id → 0x0200 offered codes → 0x0205 subscribe (or 0x0201 per code) →
reads (section 7 when the bitmap has bit 56, else one by one)
```

Nothing is polled after that. Battery, wearing, ANC and game mode arrive as pushes `[TESTED]`.

- **Bitmap** `0x8100` (section 4). Every command the app sends is checked against it. A command with no bit is not sent.
- **Product id** `0x8103`: `00 <id, 3 bytes LE>`. Buds 4: `00 14 54 06` is `065414` `[CAPTURE]`. Four colour ranges
  fold into one id before the model lookup `[VENDOR]`: `100100`-`100102` → `060414`, `100200`-`100202` → `060814`,
  `108100`-`108102` → `068414`, `108200`-`108202` → `068814`.
- **Firmware** `0x0105` → `00 <count>` then UTF-8 text `device,type,version` repeated, joined by commas `[CAPTURE]`.
  Device `1` left, `2` right, `3` case. Buds 4: `1,2,138,2,2,138,3,1,01,3,2,105`. The app shows the type `2` versions in reply
  order, joined by dots: `138.138.105`.
- **Offered codes** `0x8200` = `00 <count> <codes>`. Buds 4 `[CAPTURE]`: `00 09 01 02 03 04 08 0B F1 F2 F3`. Read it
  before deciding that an event does not exist: silence usually means "not subscribed".
- **Subscribe** `0x0205 <count> <codes>`, count first `[CAPTURE]`. The app asks for `01 02 03`, plus `04` when the bitmap has
  `0x0405`, `08` when it has `0x040D`, `0B` when it has `0x0412`. It keeps only the codes the buds offered, as the vendor app
  registers exactly the offered list `[VENDOR]`.
  - Never send `01 01 02 02`: that is "count 1, battery only". The buds accept it and never push wearing.
  - Never drop `03` when offered. Without it Buds 4 sends no ANC change made on the buds.
  - Nord Buds 3 Pro offers only `02 01` `[CAPTURE]` and sent no `0x8205` when asked for more. OPPO Enco Buds2 also offers `02 01`.
  - The `0x8205` reply is `01 <count>` then `<code> 00` per code `[GUESS]`. The vendor app reads none of it; the reply ends the setup.
- Which model the buds are is the app's choice, not the protocol's. The model list matches the Bluetooth name first, then the
  product id.

## 7. Batch query — `0x012F`

Bitmap bit 56. One message carries many queries `[CAPTURE]`: HeyMelody sends one with 24 queries after `0x0100`, `0x0200`
and `0x0205`.

- Request: `<count>` then `<cmd LE> <len LE> <payload>` per query.
- Reply: several `0x812F` messages with the request's `Seq`, each `00 <count>` then `<cmd LE> <len LE> <reply payload>` per
  answer. `cmd` is the query's. Handle each answer as the reply `cmd | 0x8000` with the batch's `Seq` `[VENDOR]`. An answer
  that runs past the end ends the list.
- Buds 4 query list `[CAPTURE]`: `0102 FFFF`, `0106`, `010B`, `0103`, `0101 0002`, `0114`, `0105`, `0107`, `0108 020301`,
  `010C 0201`, `010C 0101`, `010C 0301`, `010C 0401`, `010F`, `010D 09 05 04 0B 11 18 06 1B 1D 1C`, `0121`, `0123`,
  `0118 0101`, `011C`, `0115`, `011E`, `0122`, `0105`, `0109`.
- Buds without bit 56 (Enco Buds2) take the queries one by one.
- QuickBuds sends its own list. A later status read goes alone.

## 8. To move over

Battery and wearing, ANC, gestures, feature switches, EQ, sound settings, dual connection, requests from the buds,
Golden Sound.
