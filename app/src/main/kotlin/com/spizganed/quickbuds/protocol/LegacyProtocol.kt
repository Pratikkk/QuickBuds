package com.spizganed.quickbuds.protocol

/*
 * The legacy protocol: every builder and parser of the old wire code, in one file.
 * The typed layer (protocol2/NewProtocol.kt) is not in use yet; tests keep it byte-identical to this file.
 * Shared data is not here: Feature.kt, Capabilities.kt, ModelCatalog.kt, AncModes.kt.
 */

// ===== OpoProtocol =====

/**
 * OPPO / OnePlus / realme earbud RFCOMM protocol constants.
 *
 * PROVENANCE — almost nothing here was documented by the vendor. The command
 * numbers and payload shapes are reverse-engineered, and the main source is
 * Zhaoyi-ya/OppoPodsManager (https://github.com/Zhaoyi-ya/OppoPodsManager),
 * with additional framing details from
 * Star-ZER0/Pods-Protocol-Reverse-Engineering.
 *
 * READ docs/PROTOCOL-LEGACY.md BEFORE EDITING THIS FILE. It documents the frame layout,
 * every known command, the ANC set-vs-notify trap, and the assumptions that were
 * already proven wrong once. Its evidence tags record the source of each area; tag a new
 * constant there when you add it here.
 *
 * Do not copy code from those projects without checking their licenses first.
 */
object OpoProtocol {

    const val SPP_UUID_PRIMARY = "00001107-D102-11E1-9B23-00025B00A5A5"
    const val SPP_UUID_FALLBACK = "0000079A-D102-11E1-9B23-00025B00A5A5"

    const val CMD_HANDSHAKE = 0x0100
    const val CMD_QUERY_PRODUCT_ID = 0x0103
    const val CMD_QUERY_BROADCAST = 0x0200
    const val CMD_FIND_BUDS = 0x0400
    /** Earbud fit test, `01` start / `00` stop. `[VENDOR]` `switchCompactnessDetectionStatus` (PROTOCOL-LEGACY.md §9). */
    const val CMD_FIT_TEST = 0x0405
    const val CMD_SET_FEATURE = 0x0403
    const val CMD_SET_ANC = 0x0404
    const val CMD_SET_SPATIAL = 0x0422
    const val CMD_QUERY_BATTERY = 0x0106
    /** Firmware version, empty payload -> `0x8105`. `[CAPTURE]` 2026-09-27 (PROTOCOL-LEGACY.md §3). */
    const val CMD_QUERY_FIRMWARE = 0x0105
    const val CMD_QUERY_ANC = 0x010C
    const val CMD_QUERY_STATUS = 0x010D
    /** Several queries in one frame (bitmap bit 56), answered by one or more [CMD_BATCH_REPLY]. */
    const val CMD_BATCH = 0x012F
    const val CMD_BATCH_REPLY = 0x812F
    const val CMD_QUERY_EQ = 0x010F
    const val CMD_QUERY_EQ_ALL = 0x0122
    const val CMD_QUERY_BASSWAVE_LEVEL = 0x0124
    const val CMD_SET_EQ = 0x0406
    const val CMD_SAVE_CUSTOM_EQ = 0x0418
    const val CMD_SET_BASSWAVE_LEVEL = 0x041B
    const val CMD_EQ_CHANGED = 0x0504     // push: `<id>` after any EQ change
    /** Alert-sound (prompt) volume, `[level]` 1..10. `[CAPTURE]` 2026-09-25, PROTOCOL-LEGACY.md §9. */
    const val CMD_SET_ALERT_VOLUME = 0x0427
    /** -> `0x8130` `00 <level>`. Empty payload, as HeyMelody sends it. */
    const val CMD_QUERY_ALERT_VOLUME = 0x0130
    /** Paired-device list, empty payload -> `0x8112`. `[CAPTURE]` 2026-09-25, PROTOCOL-LEGACY.md §9. */
    const val CMD_QUERY_DEVICES = 0x0112
    /**
     * Another device on the buds' list, `[VENDOR]` (PROTOCOL-LEGACY.md §9): `0x0429 01 <MAC>` connect, `02 <MAC>`
     * disconnect, `04 00` preferred device automatic, `04 01 <MAC>`; the MAC in written order (the list
     * reply carries it reversed). Read the preferred one with `0x0132 02` -> `00 02 <00 auto | 01 <MAC>>`.
     */
    const val CMD_MULTI_CONNECT = 0x0429
    const val CMD_QUERY_PREFERRED = 0x0132
    /** Sent by HeyMelody after every dual-device toggle, meaning unknown. `[CAPTURE]` 2026-09-25. */
    const val CMD_DUAL_FOLLOWUP = 0x0413

    // --- gesture / key-function bindings ---
    const val CMD_QUERY_KEY_FUNCTION = 0x0108  // getKeyFunction — current bindings
    const val CMD_RESP_KEY_FUNCTION = 0x8108
    /**
     * setKeyFunction — write the gesture bindings back.
     *
     * **`0x0401` IS CONFIRMED ON THE DEVICE.** Every write is acked
     * immediately — `RX AA 08 00 00 01 84 .. 01 00 00`, payload `00` = success — and the
     * `0x8108` read-back then shows a real `KEYFN DIFF:` change. The feature works.
     *
     * How the wrong number got in, kept because it is the trap:
     *   - `0x0402` was sent repeatedly in a WELL-FORMED frame (`TotalLen 80 = 7 + 73`,
     *     payload `12` + 18x4) and the buds ignored it in total silence: no ack, and the
     *     0x8108 read-back still showed the old table. **A wrong command number fails
     *     silently** — "it worked" and "it did nothing" are indistinguishable without
     *     reading the table back, which is why every write re-reads and diffs.
     *   - `0x0401` is what the Melody-derived setting tables list for `setKeyFunction`
     *     (`BtOperate.m2699L`), and those tables run 0x0400, 0x0401, 0x0403, 0x0404 —
     *     0x0402 does not appear among them at all.
     *   - OppoPodsManager mentions 0x0402 only inside a COMMENT on its 0x0108 query line,
     *     and defines no constant for it. An earlier note here took that comment as a table
     *     entry, which is how the wrong number got in — and it cost a session.
     *
     * Payload SHAPE: `<count> [deviceType, button, buttonAction, function]...`, which is
     * what the `0x8108` read reply returns minus its leading status byte.
     */
    const val CMD_SET_KEY_FUNCTION = 0x0401

    // --- wearing / in-case status (reverse-engineered from OppoPodsManager) ---
    const val CMD_QUERY_WEARING = 0x0109      // getEarBudsStatus — THE in-case query
    const val CMD_RESP_WEARING = 0x8109
    const val CMD_ACTIVE_REPORT = 0x0204      // spontaneous notification, payload[0] = subType
    const val CMD_REGISTER_NOTIFY = 0x0205    // subscribe to spontaneous notifications
    const val EVT_WEARING = 0x02              // 0x0204 subType: wearing status changed
    const val EVT_GAME_MODE = 0x05            // 0x0204 subType: game mode changed
    const val EVT_FIT_TEST = 0x04             // 0x0204 subType: fit test result, [VENDOR]
    const val EVT_PUBLIC = 0x0F               // 0x0204 subType: `<flags u16 LE> <data>`; the first flag wins: bit 0 alert-sound volume, bit 4 tap level, [VENDOR]
    const val PUBLIC_FLAG_ALERT_VOLUME = 0x01
    const val PUBLIC_FLAG_TAP_LEVEL = 0x10
    const val EVT_GOLDEN_STATUS = 0x08        // 0x0204 subType: Golden Sound test status, [VENDOR]
    const val EVT_EAR_SCAN = 0x0E             // 0x0204 subType: ear scan result, [CAPTURE] (pushed unregistered)

    const val CMD_GAME_SOUND = 0x0423
    /** Game sound type: `0x0423 <type> 01`, read `0x012B` -> `00 <selected> <count> <types>`. `[VENDOR]` (PROTOCOL-LEGACY.md §9). */
    const val CMD_QUERY_GAME_SOUND = 0x012B
    /** Spatial type (`0` off, `1` fixed, `2` head tracking): `0x0422 <type>`, read `0x012A` -> `00 <type>`, pushed as `0x0510 <type>`. `[VENDOR]` */
    const val CMD_SET_SPATIAL_TYPE = 0x0422
    const val CMD_QUERY_SPATIAL_TYPE = 0x012A
    const val CMD_SPATIAL_TYPE_PUSH = 0x0510
    /** Who renders 3D audio: `0x041E 00` the buds, `01` the phone. realme Link sends `00` on connect. `[VENDOR]` (PROTOCOL-LEGACY.md §9). */
    const val CMD_SPATIAL_SOURCE = 0x041E
    /**
     * Codec picker on `hiRes` models (PROTOCOL-LEGACY.md §9), `[VENDOR]`, unverified on buds: read the
     * current codec with `0x0114` -> `00 <codec>`, the offered ones with `0x0123` -> `00 <u16 LE mask>`
     * (bit k = codec k + 1), write `0x041A <codec> <hiRes> 00`. The buds restart after the write.
     */
    const val CMD_QUERY_CODEC = 0x0114
    const val CMD_QUERY_CODEC_LIST = 0x0123
    const val CMD_SET_CODEC = 0x041A
    const val CMD_GAME_EQ_STATE = 0x0420
    /** Head gestures' mapping: `0x0431 <type>`, `0` nod answers / shake declines, `1` the reverse. `[VENDOR]` */
    const val CMD_SET_HEAD_MOTION_TYPE = 0x0431
    /** Read with `0x0134` (empty) → `00 <type>`; a change also comes as the `0x0204` push [EVT_HEAD_MOTION_TYPE]. `[VENDOR]` */
    const val CMD_QUERY_HEAD_MOTION_TYPE = 0x0134
    const val EVT_HEAD_MOTION_TYPE = 0xF5
    const val CMD_PERSONAL_NOISE = 0x0412
    const val CMD_QUERY_PERSONAL_NOISE = 0x011A
    const val EVT_PERSONAL_NOISE = 0x0B
    const val PERSONAL_NOISE_TEST = 1
    const val PERSONAL_NOISE_USE_STORED = 2
    const val PERSONAL_NOISE_CANCEL = 3
    /** Tap sensitivity 1..5 (lower triggers more easily): `0x042D <level>`, read `0x0133` -> `00 <level> <default>`. `[VENDOR]` */
    const val CMD_SET_TAP_LEVEL = 0x042D
    const val CMD_QUERY_TAP_LEVEL = 0x0133

    private var seqCounter = 0x01

    /** 0xC0..0xDF belongs to the feature-sync engine (see docs/FEATURE-SYNC.md): skip it. */
    @Synchronized
    private fun nextSeq(): Int {
        val seq = seqCounter
        seqCounter = if (seqCounter >= 0xBF) 0x01 else seqCounter + 1
        return seq
    }

    /** `TotalLen` is LEB128: one byte up to 127, more after (PROTOCOL-LEGACY.md §2). */
    @Synchronized
    fun buildPacket(cmd: Int, seq: Int? = null, payload: ByteArray = ByteArray(0)): ByteArray {
        val s = seq ?: nextSeq()
        val payLen = payload.size
        var totalLen = 7 + payLen
        val len = mutableListOf<Byte>()
        do {
            val b = totalLen and 0x7F
            totalLen = totalLen shr 7
            len += (if (totalLen != 0) b or 0x80 else b).toByte()
        } while (totalLen != 0)
        val head = byteArrayOf(
            0xAA.toByte(), *len.toByteArray(), 0x00, 0x00,
            (cmd and 0xFF).toByte(), ((cmd shr 8) and 0xFF).toByte(), s.toByte(),
            (payLen and 0xFF).toByte(), ((payLen shr 8) and 0xFF).toByte()
        )
        return head + payload
    }

    /** Time request `0x0500` from the buds (PROTOCOL-LEGACY.md §9). */
    const val REQ_TIME = 0x0500
    /** Command groups HeyMelody knows; a request from the buds in any other gets `01`. */
    private val KNOWN_GROUPS = setOf(0x0100, 0x0200, 0x0300, 0x0400, 0x0500, 0x0F00)

    /**
     * HeyMelody's answer to a frame the buds send on their own, or null (PROTOCOL-LEGACY.md §9 "Requests
     * from the buds"). [cmd], [seq], [payload] are the incoming frame's. `[VENDOR]`
     */
    fun answerFor(cmd: Int, seq: Int, payload: ByteArray, unixSeconds: Long): ByteArray? {
        if (cmd and 0x8000 != 0) return null
        val answer = when (cmd) {
            REQ_TIME -> unixSeconds.toInt().let { t ->
                byteArrayOf(0, t.toByte(), (t shr 8).toByte(), (t shr 16).toByte(), (t shr 24).toByte())
            }
            0x050C -> byteArrayOf(0, 1, 0)
            0x051C -> byteArrayOf(0, 0, 0)
            // ponytail: always "invalid"; HeyMelody answers the phone's spatial mode, which we do not have.
            0x051D -> byteArrayOf(1)
            CMD_ACTIVE_REPORT -> when (payload.firstOrNull()?.toInt()?.and(0xFF)) {
                0xF2 -> byteArrayOf(0, 0xF2.toByte())
                0xF4 -> jsonAck(payload.copyOfRange(1, payload.size))
                else -> null
            }
            else -> if (cmd and 0x7F00 !in KNOWN_GROUPS) byteArrayOf(1) else null
        } ?: return null
        return buildPacket(cmd or 0x8000, seq, answer)
    }

    /** `F4` JSON push: `00 F4 {"cmd":<its cmd>}`, `01 F4` if it does not parse. */
    private fun jsonAck(json: ByteArray): ByteArray {
        val head = byteArrayOf(0, 0xF4.toByte())
        val text = String(json, Charsets.UTF_8)
        if (text.isEmpty()) return head
        return try {
            val cmd = org.json.JSONObject(text).opt("cmd")
            head + org.json.JSONObject().apply { if (cmd is String) put("cmd", cmd) }.toString().toByteArray()
        } catch (e: org.json.JSONException) {
            com.spizganed.quickbuds.bluetooth.PacketLogger.error("JSON push", e)
            byteArrayOf(1, 0xF4.toByte())
        }
    }

    fun buildHandshake(): ByteArray = buildPacket(CMD_HANDSHAKE)
    fun buildQueryProductId(): ByteArray = buildPacket(CMD_QUERY_PRODUCT_ID)
    fun buildQueryBroadcastCodes(): ByteArray = buildPacket(CMD_QUERY_BROADCAST)

    /** getEarBudsStatus (0x0109) — returns [count][comp,st] pairs, st=4 means in case. */
    fun queryWearingStatus(): ByteArray = buildPacket(CMD_QUERY_WEARING, seq = 0xF2)

    /**
     * Subscribe to spontaneous 0x0204 notifications (battery + wearing + ANC).
     *
     * Payload is canonical: a count byte followed by event ids.
     *
     * CONFIRMED on device: with count=2 and events battery (0x01) + wearing (0x02),
     * the ACK lists BOTH and the buds then emit 0x0204 subtype 02 on every wear
     * change. Wear updates are effectively instant — no polling needed.
     *
     * The old legacy literal 01 01 02 02 was a misread: under the count-first
     * shape it means count=1 (register battery only) with 02 02 left over. The
     * firmware ACKed that happily and silently never sent wear events, which is
     * why wear appeared to be poll-only. Do NOT revert to that.
     *
     * 0x03 (ANC): the buds advertise their event ids in
     * the 0x8200 broadcast-codes reply, which lists `01 02 03 04 08 0B F1 F2 F3` —
     * 03 is offered, and the ANC subType 0x03 push arrived only unreliably while we
     * had not subscribed to it. Subscribing is the documented way to make a 0x0204
     * subType arrive, and it is how wear was fixed, so the same reasoning applies.
     *
     * CONFIRMED ON DEVICE: the 0x8205 ACK lists `01 02 03`, and a bud-side
     * gesture now updates the app circles AND the widget, including a gesture REBOUND
     * to a different mode cycle. If ANC ever stops following gestures, check the ACK
     * still lists `01 02 03` first — a firmware that rejects the longer list would ACK
     * a shorter one.
     *
     * 0x04 (fit test result) is added when the buds have the fit test (`0x0405`):
     * HeyMelody registers every id the buds list in 0x8200 (`registerMultiNotification`).
     * 0x08 (Golden Sound test status) likewise, when the buds have the test (`0x040D`).
     */
    /**
     * The events to subscribe to: ours, kept to those the buds offer in `0x8200` when they sent one,
     * as HeyMelody subscribes to the offered list only (PROTOCOL-LEGACY.md §4). Some firmwares never ack a
     * list with others.
     */
    fun notifyIds(fitTest: Boolean, golden: Boolean, personalNoise: Boolean, offered: Set<Int>?): List<Int> =
        (listOf(0x01, 0x02, 0x03) + (if (fitTest) listOf(EVT_FIT_TEST) else emptyList()) +
            (if (golden) listOf(EVT_GOLDEN_STATUS) else emptyList()) +
            (if (personalNoise) listOf(EVT_PERSONAL_NOISE) else emptyList()))
            .filter { offered == null || it in offered }

    fun registerNotifications(ids: List<Int>): ByteArray =
        buildPacket(CMD_REGISTER_NOTIFY, payload = byteArrayOf(ids.size.toByte()) + ids.map { it.toByte() })

    /** Subscribe one event, for buds without `0x0205` in their bitmap. `[VENDOR]` */
    const val CMD_REGISTER_ONE = 0x0201
    fun registerOne(id: Int): ByteArray = buildPacket(CMD_REGISTER_ONE, payload = byteArrayOf(id.toByte()))

    /** `0x8200` = `00 <count> <codes>`, or null. */
    fun offeredEvents(payload: ByteArray): Set<Int>? {
        if (payload.size < 2 || payload[0].toInt() != 0) return null
        val n = payload[1].toInt() and 0xFF
        if (payload.size < 2 + n) return null
        return (0 until n).map { payload[2 + it].toInt() and 0xFF }.toSet()
    }

    // --- Golden Sound test (PROTOCOL-LEGACY.md §9), `[CAPTURE]` 2026-09-29 + `[VENDOR]` ---
    const val CMD_GOLDEN_DETECT = 0x040D      // q0: ear scan `04 01|00 <uid>`, hearing test `02 01|00`
    const val CMD_GOLDEN_RECORD = 0x040E      // w0 / m1: tone, stop tone, apply record
    const val CMD_GOLDEN_RESTORE = 0x0411     // F0: restore data (the description id)
    const val CMD_GOLDEN_SCAN_DATA = 0x0415   // v0: ear-scan data
    const val CMD_GOLDEN_FILTER = 0x0116      // W: hearing filter, the reply carries the enhance type
    const val CMD_GOLDEN_SCAN_FILTER = 0x011F // M: ear-scan filter (the graph only)
    const val CMD_GOLDEN_ACTIVE = 0x0115      // the record on the buds
    const val CMD_GOLDEN_ACTIVE_SCAN = 0x011E // the ear-scan data on the buds

    private fun int32(v: Int) = byteArrayOf((v ushr 24).toByte(), (v ushr 16).toByte(), (v ushr 8).toByte(), v.toByte())
    private fun le16(v: Int) = byteArrayOf(v.toByte(), (v ushr 8).toByte())
    /** 12 x `<side> <freq> <value>`: left 1..6 then right 1..6, as HeyMelody sends them. */
    private fun hearingInfo(values: IntArray) = ByteArray(36) { i ->
        when (i % 3) { 0 -> (i / 18 + 1).toByte(); 1 -> (i / 3 % 6 + 1).toByte(); else -> values[i / 3].toByte() }
    }

    /** Ear scan start / stop, `04 01|00 <uid>`; the result comes as event `0x0E`. */
    fun earScan(on: Boolean, uid: Int): ByteArray =
        buildPacket(CMD_GOLDEN_DETECT, payload = byteArrayOf(0x04, if (on) 0x01 else 0x00) + int32(uid))

    /** Hearing test start / stop, `02 01|00`. */
    fun hearingTest(on: Boolean): ByteArray =
        buildPacket(CMD_GOLDEN_DETECT, payload = byteArrayOf(0x02, if (on) 0x01 else 0x00))

    /** Plays a tone on one bud: `03 01 <side 1 L / 2 R> <freq 1..6> <value>`. */
    fun hearingTone(side: Int, freq: Int, value: Int): ByteArray =
        buildPacket(CMD_GOLDEN_RECORD, payload = byteArrayOf(0x03, 0x01, side.toByte(), freq.toByte(), value.toByte()))

    /** Stops the tone, `04`; HeyMelody sends it before every new level. */
    fun hearingToneStop(): ByteArray = buildPacket(CMD_GOLDEN_RECORD, payload = byteArrayOf(0x04))

    /** Asks for the filter of a result; the `0x8116` reply's byte 9 is the enhance type. */
    fun hearingFilter(uid: Int, values: IntArray): ByteArray =
        buildPacket(CMD_GOLDEN_FILTER, payload = byteArrayOf(0x0C) + hearingInfo(values) + int32(uid))

    /** Asks for the ear-scan filter of a record: `<length little-endian> <data> <uid>`. */
    fun earScanFilter(uid: Int, data: ByteArray): ByteArray =
        buildPacket(CMD_GOLDEN_SCAN_FILTER, payload = le16(data.size) + data + int32(uid))

    /** Applies a record: `03 0c <12 x info> <uid> <name>`. */
    fun hearingRecord(uid: Int, name: String, values: IntArray): ByteArray =
        buildPacket(CMD_GOLDEN_RECORD, payload = byteArrayOf(0x03, 0x0C) + hearingInfo(values) + int32(uid) + name.toByteArray())

    /** The record's description id, `01 01 01 00 <id>` (count 1, type 1, length 1 little-endian). */
    fun hearingRestore(descId: Int): ByteArray =
        buildPacket(CMD_GOLDEN_RESTORE, payload = byteArrayOf(0x01, 0x01, 0x01, 0x00, descId.toByte()))

    /** Applies ear-scan data: `03 <length little-endian> <data> <uid>`. */
    fun earScanData(uid: Int, data: ByteArray): ByteArray =
        buildPacket(CMD_GOLDEN_SCAN_DATA, payload = byteArrayOf(0x03) + le16(data.size) + data + int32(uid))

    fun queryGoldenActive(): ByteArray = buildPacket(CMD_GOLDEN_ACTIVE)
    fun queryGoldenActiveScan(): ByteArray = buildPacket(CMD_GOLDEN_ACTIVE_SCAN)

    fun personalNoise(action: Int): ByteArray = buildPacket(CMD_PERSONAL_NOISE, payload = byteArrayOf(action.toByte()))
    fun queryPersonalNoise(): ByteArray = buildPacket(CMD_QUERY_PERSONAL_NOISE)
    fun gameEqState(inGame: Boolean): ByteArray = buildPacket(CMD_GAME_EQ_STATE, payload = byteArrayOf(1, if (inGame) 1 else 0))
    fun setTapLevel(level: Int): ByteArray = buildPacket(CMD_SET_TAP_LEVEL, payload = byteArrayOf(level.toByte()))
    fun queryTapLevel(): ByteArray = buildPacket(CMD_QUERY_TAP_LEVEL)

    /** Earbud fit test: `0x0405` `01` start / `00` stop (HeyMelody stops it when its sheet closes). */
    fun fitTest(on: Boolean): ByteArray =
        buildPacket(CMD_FIT_TEST, payload = byteArrayOf(if (on) 0x01 else 0x00))

    /**
     * SET_ANC (0x0404): `01 01` then a little-endian bit field with bit [bit] set, `index / 8 + 1`
     * bytes long. The bit is the mode's `protocolIndex` for the connected model ([AncModes],
     * PROTOCOL-LEGACY.md §5). Buds 4: Off 0 `01 01 01`, Transparency 2 `01 01 04`, Deep 4, Medium 5,
     * Light 6, Adaptive 11 `01 01 00 08`.
     *
     * Reports (`0x810C`, `0x0204` subType 3) name the SAME tree's bits, but a mode's child where it
     * has one (Buds 4 reports Off as bit 3, Transparency as bit 8). Setting a child bit was tried
     * once and sent the wrong mode: SET uses the parent. Adaptive's bit is 11, not 8: `8` gives
     * `01 01 00 01`, a different mode.
     */
    fun anc(bit: Int): ByteArray {
        val arr = ByteArray(2 + bit / 8 + 1)
        arr[0] = 0x01
        arr[1] = 0x01
        arr[2 + bit / 8] = (1 shl (bit % 8)).toByte()
        return buildPacket(CMD_SET_ANC, payload = arr)
    }

    private fun featurePayload(featureId: Int, on: Boolean): ByteArray =
        byteArrayOf(featureId.toByte(), if (on) 0x01 else 0x00)

    /** Find my earbuds — `0x0400` `01` start / `00` stop, both buds, no side byte. `[CAPTURE]` 2026-09-23. */
    fun findTone(on: Boolean): ByteArray =
        buildPacket(CMD_FIND_BUDS, payload = byteArrayOf(if (on) 0x01 else 0x00))

    /** Any `0x0403` feature switch. Spatial (`0x1B`) and Hi-Res (`0x18`) are `[CAPTURE]` — PROTOCOL-LEGACY.md §9. */
    fun setFeature(featureId: Int, on: Boolean, seq: Int? = null): ByteArray =
        buildPacket(CMD_SET_FEATURE, seq = seq, payload = featurePayload(featureId, on))

    fun queryFirmware(): ByteArray = buildPacket(CMD_QUERY_FIRMWARE)

    /**
     * The `0x8105` text as HeyMelody shows it: the `versionType` 2 versions in reply order, joined
     * with dots. Buds 4 `1,2,138,2,2,138,3,1,01,3,2,105` -> `138.138.105`, as on HeyMelody's own
     * screen `[CAPTURE]` 2026-09-27. Null when there is none.
     */
    fun firmwareVersion(text: String): String? = text.split(',').chunked(3)
        .filter { it.size == 3 && it[1].trim() == "2" }
        .joinToString(".") { it[2].trim() }
        .ifEmpty { null }
    fun queryBattery(): ByteArray = buildPacket(CMD_QUERY_BATTERY, seq = 0xF0)
    fun queryAncMode(): ByteArray = buildPacket(CMD_QUERY_ANC, payload = byteArrayOf(0x01, 0x01))

    /** The level Smart chose: `00 04 01 <SET bit mask LE>`, also pushed as `03 04 01 <mask>` `[VENDOR]`. */
    fun querySmartLevel(): ByteArray = buildPacket(CMD_QUERY_ANC, payload = byteArrayOf(0x04, 0x01))

    /**
     * getNoiseReductionSwitchMode — WHICH ANC modes the hold cycles through.
     *
     * `0x010C` is one command number carrying several different questions, selected by the
     * payload. `[OSS]`, from the Melody-derived tables:
     *
     *     01 01   getCurrentNoiseReductionMode   what is active right now  <- queryAncMode()
     *     02 01   getNoiseReductionSwitchMode    the HOLD's mode list
     *     02 03   "                              (list may be one of these)
     *     02 04   "                              (…the source lists all three)
     *     04 01   getIntelligentNoiseReductionMode
     *
     * WHY THIS EXISTS: the hold's ANC cycle has its own write (setHoldAncModes).
     * Its key-function byte (`0x08`) only *reports* that the hold cycles ANC — measured on
     * the device, clearing that byte did not stop the cycle, and which modes it stepped
     * through was decided entirely on the vendor side. The mode list lives HERE instead, and
     * this read is the cheap, read-only way to see it before anyone writes `0x0404`.
     *
     * READ-ONLY, so it cannot change a binding. `[VENDOR]`: `02 01` on
     * most models, `02 03` (left) and `02 04` (right) on models with a per-bud hold
     * (`longPressType`), see [HOLD_TYPE_SHARED].
     */
    fun queryNoiseSwitchModes(type: Int = HOLD_TYPE_SHARED): ByteArray =
        buildPacket(CMD_QUERY_ANC, payload = byteArrayOf(0x02, type.toByte()))

    /**
     * The hold cycle's noise type, the second byte of [setHoldAncModes] and [queryNoiseSwitchModes].
     * `[VENDOR]`: 1 for one cycle shared by both buds, 3 (left) / 4
     * (right) on models whose `longPressType` gives each bud its own hold.
     */
    const val HOLD_TYPE_SHARED = 1
    const val HOLD_TYPE_LEFT = 3
    const val HOLD_TYPE_RIGHT = 4

    /**
     * setSupportNoiseReduction — WRITE which ANC modes the hold cycles through.
     *
     * `[CAPTURE]` 2026-09-22, via an HCI capture of HeyMelody itself (Option C,
     * PACKET-CAPTURE.md): he added Adaptive to a 3-stop hold cycle in HeyMelody, and the
     * app sent `AA 0A 00 00 04 04 <seq> 04 00 02 01 07 08` — `0x0404` action `02`, noiseType
     * `01`, then the mask `07 08` (little-endian `0x0807`). Acked (`0x8404` status `00`), and
     * the immediate `0x010C` `02 01` read-back returned the same `07 08`, up from `07 00`
     * (`0x0007`) beforehand.
     *
     * THIS CONFIRMS THE MASK REUSES [anc]'S OWN BIT NUMBERING, not a separate scheme —
     * bit 11 (`0x0800`) is exactly Adaptive's bit in the plain SET_ANC table above, and it is
     * the only bit that moved. PROTOCOL-LEGACY.md §5 previously carried this as `[INFERRED]`; this
     * capture settles it. Off = bit 0, Transparency = bit 2 (matching [anc]); bit 1 is
     * some generic "On" that resolves to whichever level was last hand-set, seen set in every
     * capture so far and never independently isolated.
     *
     * A DIFFERENT COMMAND FROM [anc], same command NUMBER. `0x0404`'s first payload
     * byte selects the question: `01 01 <bits>` sets the CURRENT mode (see [ancOff] etc.),
     * `02 01 <mask LE>` sets the cycle's MEMBERSHIP. Do not merge these two payload shapes.
     */
    fun setHoldAncModes(mask: Int, type: Int = HOLD_TYPE_SHARED): ByteArray {
        // `[VENDOR]` `NoiseReductionInfo.getData()`: the mask in as few bytes as it needs (1-4), LE.
        var len = 4
        while (len > 1 && (mask ushr ((len - 1) * 8)) and 0xFF == 0) len--
        return buildPacket(
            CMD_SET_ANC,
            payload = byteArrayOf(0x02, type.toByte()) + ByteArray(len) { (mask ushr (it * 8)).toByte() }
        )
    }

    /**
     * setKeyFunction write for the on-call group (`btn 0x06`) — `[CAPTURE]`, same capture as
     * [setHoldAncModes]. HeyMelody writes ONE entry, `deviceType = 0x04`
     * ([KeyFunctionParser.DEVICE_TYPE_BOTH]), which the buds fan out to both sides: the
     * following `0x8108` read-back always showed the SAME `fn` on `dev=0x01` AND `dev=0x02`,
     * never `dev=0x04` itself. See [KeyFunctionParser.BUTTON_ON_CALL] for the full capture and
     * the confirmed act-to-row mapping.
     */
    /**
     * `act 0x02`, toggled `[CAPTURE]` between `fn 0x00` and `fn 0x1D` while he switched
     * HeyMelody's on-call **double tap** row between None and Answer/end call, twice. The
     * bytes are exactly what shipped from the vendor app; the ENGLISH LABEL ("double tap")
     * is `[TESTED]` on a real call — see [KeyFunctionParser.BUTTON_ON_CALL].
     *
     * Named (not inlined) so [com.spizganed.quickbuds.ui.OnCallConfigStore] can reverse-map a
     * device reading back to a row without duplicating these numbers.
     */
    const val ON_CALL_ACT_DOUBLE_TAP = 0x02
    const val ON_CALL_FN_ANSWER_END = 0x1D

    /**
     * `act 0x06`, toggled `[CAPTURE]` between `fn 0x00` and `fn 0x1C` while he switched
     * HeyMelody's on-call **long hold** row between None and Decline call, twice. Same
     * evidence as the double-tap pair above: the bytes are measured, and the "long hold"
     * label is `[TESTED]` on a real call.
     */
    const val ON_CALL_ACT_LONG_HOLD = 0x06
    const val ON_CALL_FN_DECLINE = 0x1C

    private fun onCallPayload(act: Int, fn: Int, button: Int) = byteArrayOf(
        0x01,
        KeyFunctionParser.DEVICE_TYPE_BOTH.toByte(),
        button.toByte(),
        act.toByte(),
        fn.toByte()
    )

    /**
     * `[VENDOR]`: callControl 28 / 32 are `act 0x01` (single tap),
     * 29 / 33 / 36 `act 0x02`, 30 / 34 `act 0x03`, 31 / 35 `act 0x06`; all `btn 0x06`, `dev 0x04`.
     */
    const val ON_CALL_ACT_SINGLE_TAP = 0x01

    /** [button] is [KeyFunctionParser.BUTTON_ON_CALL], or `btn 01` for the realme both-buds hold. */
    fun setOnCall(act: Int, fn: Int, button: Int = KeyFunctionParser.BUTTON_ON_CALL): ByteArray =
        buildPacket(CMD_SET_KEY_FUNCTION, payload = onCallPayload(act, fn, button))

    // --- Equalizer, all [CAPTURE] 2026-09-23 (PROTOCOL-LEGACY.md §9) ---

    /** Current EQ -> `0x810F` `00 <id>` (00-02 built-in, 04+ custom). */
    fun queryEq(): ByteArray = buildPacket(CMD_QUERY_EQ)
    /** Custom preset list -> `0x8122`, see [EqCodec.parseList]. Empty payload, as HeyMelody sends it. */
    fun queryEqAll(): ByteArray = buildPacket(CMD_QUERY_EQ_ALL)
    fun setBuiltInEq(id: Int): ByteArray = buildPacket(CMD_SET_EQ, payload = byteArrayOf(id.toByte()))
    fun customEq(action: Int, p: EqCodec.Preset): ByteArray =
        buildPacket(CMD_SAVE_CUSTOM_EQ, payload = EqCodec.encode(action, p))
    /** BassWave level, signed -5..+5; `FB 05` is the range (min, max) HeyMelody sends. */
    fun setBassWaveLevel(level: Int): ByteArray =
        buildPacket(CMD_SET_BASSWAVE_LEVEL, payload = byteArrayOf(0xFB.toByte(), 0x05, level.toByte()))
    /** -> `0x8124` `00 FB 05 <level>`. */
    fun queryBassWaveLevel(): ByteArray = buildPacket(CMD_QUERY_BASSWAVE_LEVEL)

    fun setAlertVolume(level: Int): ByteArray =
        buildPacket(CMD_SET_ALERT_VOLUME, payload = byteArrayOf(level.coerceIn(1, 10).toByte()))
    fun queryAlertVolume(): ByteArray = buildPacket(CMD_QUERY_ALERT_VOLUME)

    fun setSpatialType(type: Int): ByteArray = buildPacket(CMD_SET_SPATIAL_TYPE, payload = byteArrayOf(type.toByte()))
    fun querySpatialType(): ByteArray = buildPacket(CMD_QUERY_SPATIAL_TYPE)
    fun spatialOnBuds(): ByteArray = buildPacket(CMD_SPATIAL_SOURCE, payload = byteArrayOf(0))
    fun queryCodec(): ByteArray = buildPacket(CMD_QUERY_CODEC)
    fun queryCodecList(): ByteArray = buildPacket(CMD_QUERY_CODEC_LIST)
    /** HeyMelody sends Hi-Res only with LDAC (`3`) or LHDC V5 (`8`), else `0`; the last byte is always `0`. */
    fun setCodec(codec: Int, hiRes: Boolean): ByteArray = buildPacket(CMD_SET_CODEC,
        payload = byteArrayOf(codec.toByte(), if (hiRes && (codec == 3 || codec == 8)) 1 else 0, 0))
    /** HeyMelody sends every pick, Off (`0`) included, with enable `01`. */
    fun setGameSoundType(type: Int): ByteArray = buildPacket(CMD_GAME_SOUND, payload = byteArrayOf(type.toByte(), 0x01))
    fun queryGameSound(): ByteArray = buildPacket(CMD_QUERY_GAME_SOUND)
    fun setHeadMotionType(type: Int): ByteArray = buildPacket(CMD_SET_HEAD_MOTION_TYPE, payload = byteArrayOf(type.toByte()))
    fun queryHeadMotionType(): ByteArray = buildPacket(CMD_QUERY_HEAD_MOTION_TYPE)

    fun queryDevices(): ByteArray = buildPacket(CMD_QUERY_DEVICES)
    private fun macBytes(mac: String) = mac.split(":").map { it.toInt(16).toByte() }.toByteArray()
    fun connectDevice(mac: String, on: Boolean): ByteArray =
        buildPacket(CMD_MULTI_CONNECT, payload = byteArrayOf(if (on) 0x01 else 0x02) + macBytes(mac))
    /** [mac] null = automatic. */
    fun setPreferred(mac: String?): ByteArray = buildPacket(CMD_MULTI_CONNECT,
        payload = if (mac == null) byteArrayOf(0x04, 0x00) else byteArrayOf(0x04, 0x01) + macBytes(mac))
    fun queryPreferred(): ByteArray = buildPacket(CMD_QUERY_PREFERRED, payload = byteArrayOf(0x02))
    /** `08 00 01` after dual OFF, `08 00 00` after ON — copied from HeyMelody, not understood. */
    fun dualFollowup(dualOn: Boolean): ByteArray =
        buildPacket(CMD_DUAL_FOLLOWUP, payload = byteArrayOf(0x08, 0x00, if (dualOn) 0x00 else 0x01))

    /**
     * getKeyFunction (0x0108) — read the CURRENT gesture bindings.
     *
     * READ-ONLY, and deliberately so: it reports what the buds are already doing,
     * so it cannot change state or break a binding. The 0x8108 reply carries the function
     * enum; PROTOCOL-LEGACY.md §6 lists the measured values.
     *
     * Payload is a bare query (no payload), matching the other 0x01xx reads. The buds
     * DO answer it: the reply is `<status> <count> <4-byte entries>...` and its layout
     * is `[CAPTURE]`-confirmed — see PROTOCOL-LEGACY.md §6 and `KeyFunctionParser`, which
     * logs it RAW and grouped so two readings can be diffed.
     *
     * The `function` VALUES are no longer a mystery either: they were MEASURED by
     * diffing two of these replies around a change made in the vendor app. See
     * `GestureAction.functionByte`.
     */
    fun queryKeyFunction(): ByteArray = buildPacket(CMD_QUERY_KEY_FUNCTION)

    /**
     * setKeyFunction (0x0401) — write gesture bindings back.
     *
     * Payload is `<count>` then 4 bytes per entry, and the entries are the SAME
     * [KeyFunctionParser.Entry] type the read reply decodes into, deliberately: using
     * one type for both directions is what stops the two from drifting, and it means a
     * table read back from the buds can be written out again unmodified.
     *
     * NO LEADING STATUS BYTE, unlike the reply. The reply is `<status> <count> ...`
     * (`[CAPTURE]`), but a status byte is something the device REPORTS; a write supplies
     * only the count and its entries.
     *
     * WHY THE CALLER STILL RE-READS: the payload layout above is `[OSS]`, and the ONE
     * thing the device has taught us about this command is that it ignores a wrong
     * command number in total silence (see CMD_SET_KEY_FUNCTION). A silent ignore looks
     * exactly like success unless the table is read back, so the read-back is not
     * belt-and-braces — it is the only evidence there is.
     *
     * LEB128 is safe here: the largest realistic payload is an 18-entry table,
     * `1 + 18*4 = 73` bytes, so `TotalLen = 80` fits in one LEB128 byte. `buildPacket()`
     * would only need real LEB128 above ~30 entries, which this device does not have.
     */
    fun setKeyFunction(entries: List<KeyFunctionParser.Entry>): ByteArray {
        val payload = ByteArray(1 + entries.size * KeyFunctionParser.ENTRY_SIZE)
        payload[0] = entries.size.toByte()
        for ((i, e) in entries.withIndex()) {
            val o = 1 + i * KeyFunctionParser.ENTRY_SIZE
            payload[o] = e.deviceType.toByte()
            payload[o + 1] = e.button.toByte()
            payload[o + 2] = e.action.toByte()
            payload[o + 3] = e.function.toByte()
        }
        return buildPacket(CMD_SET_KEY_FUNCTION, payload = payload)
    }

    /**
     * [statusQuery]: the model's `statusQuery` (hex ids, HeyMelody's per-model list, PROTOCOL-LEGACY.md §9); a model
     * without one gets the full list below. [windNoise]: the model's `windMode` flag adds [Feature.WIND_NOISE].
     */
    fun queryStatus(statusQuery: String? = null, windNoise: Boolean = false, seq: Int = 0x00): ByteArray {
        val ids = (statusQuery?.split(' ')?.map { it.toInt(16).toByte() }?.toByteArray() ?: byteArrayOf(
            // Feature ids. 0x1D (BassWave): HeyMelody asks for it too.
            // Second line: the Earbud settings Features switches.
            // Third line: the rest of HeyMelody's list; 0x0C personalized ANC.
            0x05, 0x04, 0x0B, 0x11, 0x13, 0x18, 0x06, 0x1B, 0x1C, 0x27, 0x28, 0x1D,
            0x09, 0x17, 0x30, 0x31, 0x3A,
            0x32, 0x35, 0x37, 0x38, 0x3B, 0x0C
        )) + if (windNoise) byteArrayOf(Feature.WIND_NOISE.toByte()) else byteArrayOf()
        return buildPacket(CMD_QUERY_STATUS, seq = seq, payload = byteArrayOf(ids.size.toByte()) + ids)
    }

    /**
     * `0x012F` (PROTOCOL-LEGACY.md "Batch query"): built query [packets] in one frame, `<count>` then
     * `<cmd LE> <len LE> <payload>` each. Each packet's length fits in one byte (short queries).
     */
    fun batch(packets: List<ByteArray>): ByteArray {
        val body = packets.flatMap { p -> (p.copyOfRange(4, 6) + p.copyOfRange(7, 9 + u16(p, 7))).toList() }
        return buildPacket(CMD_BATCH, payload = byteArrayOf(packets.size.toByte()) + body)
    }

    /**
     * `0x812F` = `00 <count>`, then `<cmd LE> <len LE> <reply payload>` per answer, `cmd` being the query's:
     * (reply cmd, payload) pairs, stopping at the first that runs past the end.
     */
    fun batchReplies(payload: ByteArray): List<Pair<Int, ByteArray>> {
        if (payload.size < 2 || payload[0].toInt() != 0) return emptyList()
        val out = mutableListOf<Pair<Int, ByteArray>>()
        var o = 2
        repeat(payload[1].toInt() and 0xFF) {
            if (o + 4 > payload.size) return out
            val len = u16(payload, o + 2)
            if (o + 4 + len > payload.size) return out
            out += (u16(payload, o) or 0x8000) to payload.copyOfRange(o + 4, o + 4 + len)
            o += 4 + len
        }
        return out
    }

    /** Little-endian unsigned 16-bit value at [i]. */
    fun u16(b: ByteArray, i: Int) = (b[i].toInt() and 0xFF) or ((b[i + 1].toInt() and 0xFF) shl 8)

    fun bytesToHex(bytes: ByteArray): String =
        bytes.joinToString(" ") { "%02X".format(it) }
}

// ===== OppoPacketFramer =====

/**
 * Reassembles 0xAA-framed OPPO protocol packets from a byte stream.
 * Handles partial frames, multiple frames in one read, and garbage before headers.
 *
 * `TotalLen` (byte 1) is LEB128, so a frame over 127 bytes has a longer header (Golden Sound's
 * ear-scan data, PROTOCOL-LEGACY.md §9). Every frame comes out [normalise]d, so the rest of the app
 * keeps its fixed layout: cmd at 4-5, seq at 6, payload length at 7-8, payload from 9.
 * Slices of one message (PROTOCOL-LEGACY.md §2) are joined into one such frame.
 */
class OppoPacketFramer {
    private var pending = ByteArray(0)

    /** Bytes thrown away because they are not part of a frame (noise before a header, a bad length). */
    var onDiscard: ((ByteArray) -> Unit)? = null

    fun append(buffer: ByteArray, length: Int): List<ByteArray> {
        if (length <= 0) return emptyList()
        pending += buffer.copyOfRange(0, length)
        val frames = mutableListOf<ByteArray>()

        while (pending.isNotEmpty()) {
            val start = pending.indexOf(0xAA.toByte())
            if (start < 0) {
                onDiscard?.invoke(pending)
                pending = ByteArray(0)
                break
            }
            if (start > 0) {
                onDiscard?.invoke(pending.copyOfRange(0, start))
                pending = pending.copyOfRange(start, pending.size)
            }
            val (totalLen, lenBytes) = readLength(pending) ?: break

            if (pending.size < 2 + lenBytes) break
            // Slice byte (PROTOCOL.md §2): 0 a whole message, 1 first, 2 middle, 3 last slice. A slice has 3 header bytes, a message 7.
            val slice = pending[1 + lenBytes].toInt()
            val sliced = slice in 1..3
            val frameLen = 1 + lenBytes + totalLen
            if (totalLen < (if (sliced) 4 else 7) || frameLen > 2048) {
                onDiscard?.invoke(pending.copyOfRange(0, 1))
                pending = pending.copyOfRange(1, pending.size)
                continue
            }
            if (pending.size < frameLen) break

            val frame = pending.copyOfRange(0, frameLen)
            pending = pending.copyOfRange(frameLen, pending.size)
            if (sliced) join(slice, frame.copyOfRange(1 + lenBytes + 3, frameLen))?.let { frames += it }
            else frames += normalise(frame)
        }
        return frames
    }

    private var slices: ByteArray? = null

    /**
     * Collects the data of slices 1, 2, 3 and returns the whole message as one normalised frame after slice 3.
     * A middle or last slice with no first slice before it is dropped. A `PayLen` past the bytes present keeps the bytes present.
     */
    private fun join(slice: Int, data: ByteArray): ByteArray? {
        if (slice == 1) slices = data
        else slices = (slices ?: return null) + data
        if (slice != 3) return null
        val msg = slices ?: return null
        slices = null
        if (msg.size < 5) return null
        val payload = minOf((msg[3].toInt() and 0xFF) or ((msg[4].toInt() and 0xFF) shl 8), msg.size - 5)
        val fixed = msg.copyOfRange(0, 3) + byteArrayOf(payload.toByte(), (payload shr 8).toByte()) + msg.copyOfRange(5, 5 + payload)
        return byteArrayOf(0xAA.toByte(), ((2 + fixed.size) and 0x7F).toByte(), 0, 0) + fixed
    }

    companion object {
        /** `TotalLen` and how many bytes it takes, or null while the frame is still too short. */
        fun readLength(frame: ByteArray): Pair<Int, Int>? {
            var value = 0
            for (i in 1 until minOf(frame.size, 4)) {
                val b = frame[i].toInt() and 0xFF
                value = value or ((b and 0x7F) shl (7 * (i - 1)))
                if (b and 0x80 == 0) return value to i
            }
            return null
        }

        /** The frame with a one-byte length field (its low 7 bits), so the header is always 9 bytes. */
        fun normalise(frame: ByteArray): ByteArray {
            if (frame.isEmpty() || frame[0] != 0xAA.toByte()) return frame
            val lenBytes = readLength(frame)?.second ?: return frame
            if (lenBytes == 1) return frame
            return byteArrayOf(frame[0], (frame[1].toInt() and 0x7F).toByte()) + frame.copyOfRange(1 + lenBytes, frame.size)
        }
    }
}

// ===== AncEventParser =====

/**
 * Parser for the 0x0204 subType 0x03 "ANC changed on the buds" active report.
 *
 * Frame shape (14 bytes):
 *
 *     AA 0C 00 00 04 02 FF 05 00 03 01 01 20 00
 *     |  |  |  |  |  |  |  |  |  |  |  |  +--+-- value (2 bytes, little endian)
 *     |  |  |  |  |  |  |  |  |  |  +--+-------- 0x01 0x01 (constant)
 *     |  |  |  |  |  |  |  |  |  +--------------- payload[0] = subType 0x03
 *     |  |  |  |  |  |  |  |  +------------------ payLen = 5
 *     |  |  |  |  |  |  +---------------------- 0xFF (event marker)
 *     |  |  |  |  |  +------------------------- sub-command 0x02
 *     |  |  |  |  +---------------------------- 0x04 (active report)
 *     |  |  |  +------------------------------- 0x00 0x00
 *     +--+--+---------------------------------- sync 0xAA + length (total - 2)
 *
 * PAYLOAD is exactly 5 bytes: `03 01 01 LO HI`.
 *
 * THE MAPPING IS CONFIRMED (capture with a known starting mode).
 * He cycled from Off on BOTH buds and reported the order, and both buds produced
 * the identical three values, so this is not side-dependent:
 *
 *     start        Off
 *     1st gesture  03 01 01 20 00   -> 0x0020   ANC on   (Medium)
 *     2nd gesture  03 01 01 00 01   -> 0x0100   Transparency
 *     3rd gesture  03 01 01 08 00   -> 0x0008   Off
 *
 * An earlier capture began from a different level and gave 0x0010 for its
 * "ANC on" stop, which is exactly the pattern to expect: the value is the buds'
 * own ANC bitmask (bit `index` set), so the ANC-on stop restores whichever LEVEL
 * was last used — 0x0010 = Deep (bit 4), 0x0020 = Medium (bit 5). The bit numbers
 * are the model's own tree, see [AncModes]. So this must NOT be treated as a
 * fixed enum of three values.
 *
 * WHY THE FRAME WAS INVISIBLE FOR THREE CAPTURES: `noteUnattributed` in
 * BudsConnectionManager blanket-excludes cmd 0x0204, so an undecoded 0x0204
 * subType prints NOTHING AT ALL. The F1 button frame fires for every gesture with
 * identical content, so it says WHEN but never WHAT. Both are documented below
 * because that combination is what made ANC look silent.
 */
object AncEventParser {

    /** 0x0204 subType: ANC mode changed on the buds. */
    const val EVT_ANC = 0x03

    /**
     * True if this 0x0204 payload is a GENUINE ANC-changed event.
     *
     * `setHoldAncModes()`'s write also raises
     * a `subType 0x03` frame (PROTOCOL-LEGACY.md §5's `[CAPTURE]` note on this), but its payload is
     * `03 02 01 <mask LE>` — the `0x010C` switch-list QUERY's own echo shape, not this event's.
     * The real event is always `03 01 01 <value LE>` (see the class doc). Before this fix,
     * `isAncEvent` only checked `payload[0] == 0x03`, so the mask-write's echo passed as a real
     * event, its MASK got misread as an ANC raw value, and the bogus name it produced
     * (`ANC-Light` was observed) got written into the persisted display state via
     * `onAncModeState` — a real, user-visible bug, not the "cosmetic, log-reading trap" it was
     * first filed as. Now checks the constant `01 01` bytes 1/2 as well, which the mask-write's
     * `02 01` echo fails.
     */
    fun isAncEvent(payload: ByteArray): Boolean {
        if (payload.size < 4) return false
        return payload[0].toInt() and 0xFF == EVT_ANC &&
            payload[1].toInt() and 0xFF == 0x01 &&
            payload[2].toInt() and 0xFF == 0x01
    }

    /**
     * Raw value from the event, little endian, 1-4 bytes (PROTOCOL.md section 9), or -1 if too short.
     * Kept separate from the name lookup so a capture can always be read back as
     * numbers, even when the name table is incomplete.
     */
    fun rawValue(payload: ByteArray): Int {
        if (payload.size < 4) return -1
        var value = 0
        for (i in minOf(payload.size, 7) - 1 downTo 3) value = (value shl 8) or (payload[i].toInt() and 0xFF)
        return value
    }

    /**
     * One-liner for the log, e.g. "raw=0x0020 (bit 5)". The mode name depends on the model
     * ([AncModes.modeForRaw]); the manager logs it next to this.
     */
    fun describe(payload: ByteArray): String {
        val raw = rawValue(payload)
        if (raw <= 0) return "ANC event (no value)"
        return "raw=0x%04X (bit %d)".format(java.util.Locale.ROOT, raw, Integer.numberOfTrailingZeros(raw))
    }
}

// ===== BatteryParser =====

object BatteryParser {

    /** SubType for 0x0204 active reports that carry a battery update. */
    const val EVT_BATTERY = 0x01

    data class Info(val level: Int, val isCharging: Boolean)
    data class Result(val left: Info?, val right: Info?, val case: Info?)

    /**
     * Parse battery query response (Cmd=0x8106).
     * Payload: [status 00][count][Index, RawValue] pairs. A count of 0, or a list that runs past the end, is no list (null).
     * Index: 1=Left, 2=Right, 3=Case.
     * RawValue: level = val & 0x7F, charging = (val & 0x80) != 0
     * Level 0 is "not known" (the Air7 Pro's closed case, issue #9): left out, as HeyMelody hides it.
     */
    fun parse(data: ByteArray): Result? {
        if (data.size < 9 || data[0] != 0xAA.toByte()) return null
        val cmd = OpoProtocol.u16(data, 4)
        if (cmd != 0x8106) return null
        val payLen = OpoProtocol.u16(data, 7)
        if (data.size < 9 + payLen || payLen < 2 || data[9].toInt() != 0) return null
        val count = data[10].toInt() and 0xFF
        if (count == 0 || payLen < 2 + count * 2) return null

        var l: Info? = null
        var r: Info? = null
        var c: Info? = null
        for (j in 0 until count) {
            val idx = data[11 + j * 2].toInt() and 0xFF
            val raw = data[12 + j * 2].toInt() and 0xFF
            val info = if ((raw and 0x7F) == 0) null else Info(raw and 0x7F, (raw and 0x80) != 0)
            when (idx) {
                1 -> l = info
                2 -> r = info
                3 -> c = info
            }
        }
        return Result(l, r, c)
    }

    /**
     * Parse active/unsolicited battery report (Cmd=0x0204, reportType=0x01).
     * Payload: [reportType=0x01][count][Index, StatusValue]...
     */
    fun parseActive(data: ByteArray): Result? {
        if (data.size < 9 || data[0] != 0xAA.toByte()) return null
        val cmd = OpoProtocol.u16(data, 4)
        if (cmd != 0x0204) return null
        val payLen = OpoProtocol.u16(data, 7)
        if (data.size < 9 + payLen || payLen < 2) return null
        if ((data[9].toInt() and 0xFF) != 0x01) return null
        val count = data[10].toInt() and 0xFF
        if (count == 0 || payLen < 2 + count * 2) return null

        var l: Info? = null
        var r: Info? = null
        var c: Info? = null
        for (j in 0 until count) {
            val idx = 11 + j * 2
            if (idx + 1 >= data.size) break
            val index = data[idx].toInt() and 0xFF
            val raw = data[idx + 1].toInt() and 0xFF
            val info = if ((raw and 0x7F) == 0) null else Info(raw and 0x7F, (raw and 0x80) != 0)
            when (index) {
                1 -> l = info
                2 -> r = info
                3 -> c = info
            }
        }
        return Result(l, r, c)
    }
}

// ===== EqCodec =====

/**
 * Custom EQ presets — the `0x8122` list and the `0x0418` select/save frame.
 * `[CAPTURE]` 2026-09-23, confirmed against HeyMelody screenshots; see PROTOCOL-LEGACY.md §9.
 *
 * List entry: `<flag> <tag 2B> <id> <nameLen> <name> <bandCount> [freq u16 LE, gain s8]...`
 * `flag` 01 = selected. `tag` is `FA 06` in every frame seen: `[VENDOR]` the gain range, min -6 then
 * max +6, which HeyMelody sets on a new preset. It is kept verbatim and echoed back.
 */
object EqCodec {

    const val GAIN_MIN = -6
    const val GAIN_MAX = 6

    class Preset(
        val id: Int,
        val name: String,
        val freqs: List<Int>,
        val gains: List<Int>,
        val selected: Boolean,
        val tag: ByteArray
    ) {
        fun withGain(band: Int, gain: Int) =
            Preset(id, name, freqs, gains.toMutableList().also { it[band] = gain }, selected, tag)

        fun withName(newName: String) = Preset(id, newName, freqs, gains, selected, tag)
    }

    /** `0x8122` payload: `<status> <count> <entries...>`. Returns null on any inconsistency. */
    fun parseList(payload: ByteArray): List<Preset>? {
        if (payload.size < 2 || payload[0].toInt() != 0) return null
        val count = payload[1].toInt() and 0xFF
        var o = 2
        val out = ArrayList<Preset>(count)
        repeat(count) {
            if (o + 5 > payload.size) return null
            val flag = payload[o].toInt() and 0xFF
            val tag = payload.copyOfRange(o + 1, o + 3)
            val id = payload[o + 3].toInt() and 0xFF
            val nameLen = payload[o + 4].toInt() and 0xFF
            o += 5
            if (o + nameLen + 1 > payload.size) return null
            val name = String(payload, o, nameLen, Charsets.UTF_8)
            o += nameLen
            val bands = payload[o].toInt() and 0xFF
            o += 1
            if (o + bands * 3 > payload.size) return null
            val freqs = ArrayList<Int>(bands)
            val gains = ArrayList<Int>(bands)
            repeat(bands) {
                freqs += OpoProtocol.u16(payload, o)
                gains += payload[o + 2].toInt() // signed byte
                o += 3
            }
            out += Preset(id, name, freqs, gains, flag == 1, tag)
        }
        return out
    }

    /** `0x0418` first byte. Create sends id `00` and the buds assign one; delete renumbers the rest. */
    const val ACTION_CREATE = 0x01
    const val ACTION_SAVE = 0x02   // select AND save in one
    const val ACTION_DELETE = 0x03

    /** The tag every captured preset carries; used for a new preset, which has none of its own yet. */
    val DEFAULT_TAG = byteArrayOf(GAIN_MIN.toByte(), GAIN_MAX.toByte())
    /** HeyMelody's bands where the model list gives none (`customEqBands`: 10 bands on 10 models). */
    val DEFAULT_FREQS = listOf(62, 250, 1000, 4000, 8000, 16000)

    fun newPreset(name: String, freqs: List<Int>) = Preset(0, name, freqs, List(freqs.size) { 0 }, false, DEFAULT_TAG)

    /** `0x0418` payload: `<action> <tag> <id> <nameLen> <name> <bandCount> <bands...>`. */
    fun encode(action: Int, p: Preset): ByteArray {
        val name = p.name.toByteArray(Charsets.UTF_8)
        val out = ArrayList<Byte>()
        out += action.toByte()
        out += p.tag.toList()
        out += p.id.toByte()
        out += name.size.toByte()
        out += name.toList()
        out += p.freqs.size.toByte()
        for (i in p.freqs.indices) {
            out += (p.freqs[i] and 0xFF).toByte()
            out += ((p.freqs[i] shr 8) and 0xFF).toByte()
            out += p.gains[i].toByte()
        }
        return out.toByteArray()
    }
}

// ===== GameModeParser =====

/**
 * Parser for the 0x0204 subType 0x05 "game mode changed" active report.
 *
 * Frame shape (2-byte payload after the 9-byte header):
 *
 *     AA 09 00 00 04 02 FF 02 00 05 01
 *     |  |  |  |  |  |  |  |  |  |  +-- payload[1] = state (0x01 ON, 0x00 OFF)
 *     |  |  |  |  |  |  |  |  |  +----- payload[0] = subType 0x05
 *     |  |  |  |  |  |  |  |  +-------- 0x00  (constant)
 *     |  |  |  |  |  |  |  +---------- 0x02  (constant)
 *     |  |  |  |  |  |  +------------- 0xFF  (event marker, same as wear/battery)
 *     |  |  |  |  +-------------------- sub-command 0x02
 *     |  |  |  +----------------------- 0x04  (active report)
 *     +--+--+-------------------------- length / flags
 *
 * The length field is 0x09 (9) because the payload is only 2 bytes, versus 8 for
 * wear and 6 for battery. Do not assume a fixed frame length.
 *
 * EVIDENCE (five samples, all from one continuous capture, `bytes[1]` = state):
 *
 *   time          frame                        state   cause
 *   14:00:44.480  ... 04 02 FF 02 00 05 01       ON     TX[GameMode] ON  (... 06 01)
 *   14:34:04.881  ... 04 02 FF 02 00 05 00       OFF    TX[GameMode] OFF (... 06 00)
 *   14:45:54.627  ... 04 02 FF 02 00 05 01       ON     TX[GameMode] ON  (... 06 01)
 *   14:50:32.433  ... 04 02 FF 02 00 05 00       OFF    BUDS GESTURE (no TX)
 *   14:50:35.743  ... 04 02 FF 02 00 05 01       ON     BUDS GESTURE (no TX)
 *
 * The last two are the important ones: they arrive with no preceding TX frame, so
 * this is a genuine push from the buds and not an echo of our own command. State
 * correlates 5/5 with the game-mode value, so the encoding is safe to rely on.
 *
 * IMPORTANT SCOPE LIMIT: this event is GAME MODE ONLY. ANC changes arrive in a
 * DIFFERENT subType, 0x03 — see AncEventParser. Do not read ANC out of this packet.
 */
object GameModeParser {

    /** True if this 0x0204 payload is a game-mode change event. */
    fun isGameModeEvent(payload: ByteArray): Boolean {
        if (payload.size < 2) return false
        return payload[0].toInt() and 0xFF == OpoProtocol.EVT_GAME_MODE
    }

    /** These models put the state in payload[3], not payload[1] `[VENDOR]`, unverified. */
    val STATE_AT_3 = setOf("062410", "062810", "063010", "063410")

    /**
     * Reads the game-mode state from a 0x0204 subType 0x05 payload.
     * Returns null when the payload is too short to contain a state byte.
     */
    fun parseActive(payload: ByteArray, productId: String? = null): Boolean? {
        if (!isGameModeEvent(payload)) return null
        val at = if (productId?.uppercase() in STATE_AT_3) 3 else 1
        // 0x01 = ON, 0x00 = OFF.
        // Read as "non-zero means ON" rather than "== 1" so an unexpected value
        // still produces a sane state instead of decoding as OFF.
        return payload.getOrNull(at)?.let { (it.toInt() and 0xFF) != 0 }
    }
}

// ===== KeyFunctionParser =====

/**
 * Parser for the `0x8108` reply to `getKeyFunction` (`0x0108`) — the CURRENT
 * gesture bindings, as the buds themselves report them.
 *
 * WHY THIS EXISTS: it answers "what are the buds currently bound to?" without an
 * HCI capture, and it is the read half of the gesture write's verification — a
 * wrong command number is ignored in complete silence, so a write is only ever
 * trusted after a read-back and diff. It is a READ, so it cannot change a binding.
 *
 * THE LAYOUT IS NOW [CAPTURE], AND THE FIRST READING OF IT WAS WRONG.
 * The `[OSS]` shape `<count> <4-byte entries>` omitted a LEADING STATUS BYTE, so
 * the first real reply printed `count=0 parsed=18 !LAYOUT`. The payload is:
 *
 *     <status> <count> <deviceType, button, buttonAction, function>...
 *      0x00     0x12     4 bytes each
 *
 * Evidence: a 74-byte payload, `00 12 ...`, where 2 + 0x12*4 = 74 EXACTLY. The
 * count is the second byte. `KeyFunctionItem.cs` does not model this: it describes
 * ONE entry's bytes, not the payload around it — which is exactly how the off-by-one
 * got in. See PROTOCOL-LEGACY.md §6.
 *
 * THE ENTRY FIELD ORDER IS AS THAT FILE SAYS, and the reply supports it: the third
 * byte is a small value (1..6, one per action this button supports) and the fourth
 * is the function (`0x00`, `0x05`, `0x06`, `0x08`, `0x11` seen). Swapping them would
 * make the "action" set `{0x00, 0x05, 0x06, 0x08, 0x11}` and the "function" set
 * `{1..6}`, which is the less plausible reading.
 *
 * The `function` VALUES were unknown for a long time and are now MEASURED — see
 * `GestureAction.functionByte` and PROTOCOL-LEGACY.md §6. Do not re-derive them.
 *
 * THE `buttonAction` NUMBERS ARE MEASURED, not guessed: 1 single, 2 double, 3 triple,
 * 4 hold, 5 slide, 6 long hold (PROTOCOL-LEGACY.md §6). They are NOT the `0x0204`
 * subType `0xF1` action numbers: `F1` reports single tap as `0x00` and long press as
 * `0x04`, and this reply binds `0x01` as well as `0x04`.
 *
 * The field is still printed as a NUMBER ONLY, so the line needs no name table.
 * Guessing ANC bits already cost this project a regression (PROTOCOL-LEGACY.md §5).
 *
 * Because the layout is exactly the thing that was once wrong here, the log line
 * still ends with `RAW=[...]`, so a capture stays readable by eye even if a future
 * firmware reshapes the reply.
 */
object KeyFunctionParser {

    /** Bytes per entry: deviceType, button, buttonAction, function. [OSS]+[CAPTURE] */
    const val ENTRY_SIZE = 4

    /**
     * Bytes before the first entry: a status/result byte, then the count.
     * `[CAPTURE]` — see the class comment. The count is NOT `payload[0]`.
     */
    const val HEADER_SIZE = 2

    /**
     * The physical-button group: what every `F1` frame this project has captured reports
     * (`btn=0x01` on both buds), so it is where the user's taps and holds live.
     *
     * It is NOT the only group a gesture writes to, and treating it as such was a real bug
     * — see [BUTTON_ON_CALL] and `writeGestureBinding()`.
     */
    const val BUTTON_PRIMARY = 0x01

    /**
     * `btn 0x06` — the on-call bindings. `[CAPTURE]` 2026-09-22 (see
     * PACKET-CAPTURE.md Option C), no longer a guess: an HCI capture of HeyMelody itself
     * caught it writing this exact group while he toggled its on-call section on/off/on/off,
     * twice over, with the `0x8108` read-back diffing exactly the slot he touched each time.
     *
     * Two rows exist in HeyMelody, confirmed: **double tap** (`None` / `Answer + end
     * call`, one combined option) and **long hold** (`None` / `Decline call`). Measured from
     * the capture:
     *
     *     act 0x02  fn 0x00 <-> 0x1D   (toggled twice — double tap)
     *     act 0x06  fn 0x00 <-> 0x1C   (toggled twice — long hold)
     *
     * `act 0x03` is untouched in every capture — a third slot this group has but HeyMelody's
     * UI never exercised, so its meaning is still unknown.
     *
     * **THE BYTES ARE MEASURED; THE ENGLISH LABELS ARE `[TESTED]`.** The two rows are
     * double tap (`None` / `Answer + end call`) and long hold (`None` / `Decline call`),
     * confirmed on a real call (PROTOCOL-LEGACY.md §6). Writing these bytes back is safe:
     * they are HeyMelody's own, byte-for-byte.
     *
     * `act 0x02` and `act 0x03` also exist in `btn 0x01` (the primary group) with UNRELATED
     * meanings (primary double tap / triple tap). A write that matched on (side, action)
     * alone, ignoring the button group, would silently re-bind the wrong gesture — which is
     * exactly why [BUTTON_PRIMARY]-scoped writes ([writeGestureBinding]) and on-call writes
     * ([OpoProtocol.setOnCall]) are separate code
     * paths that never share a table diff.
     */
    const val BUTTON_ON_CALL = 0x06

    /**
     * `deviceType = 0x04` — "both buds", WRITE-ONLY. `[CAPTURE]` 2026-09-22: every on-call
     * write HeyMelody sent used this single value instead of two separate `0x01`/`0x02`
     * entries, and the very next `0x8108` read-back always expanded it into identical `fn`
     * values on BOTH `dev=0x01` and `dev=0x02` — never `dev=0x04` itself on a READ.
     *
     * The hold's ANC-cycle membership (`setSupportNoiseReduction`, PROTOCOL-LEGACY.md §5) is a
     * DIFFERENT command with no `deviceType` byte in its payload at all — its shared-both-buds
     * behaviour is a property of that command itself, not this alias. Do not assume the two
     * are the same mechanism.
     */
    const val DEVICE_TYPE_BOTH = 0x04

    data class Entry(
        val deviceType: Int,
        val button: Int,
        val action: Int,
        val function: Int
    )

    /**
     * Named `Table`, not `Result` — a nested class called `Result` collided with
     * `kotlin.Result` and failed to compile ("inferred type is Result but Result
     * was expected"). Do not rename it back.
     */
    data class Table(
        /** `payload[0]` — a status/result byte, `0x00` in every reply so far. */
        val status: Int,
        val count: Int,
        val entries: List<Entry>,
        /** Bytes left over after `entries` — non-empty means the layout is wrong. */
        val trailing: ByteArray,
        /** True when `count` disagrees with the number of whole entries found. */
        val layoutMismatch: Boolean
    )

    /**
     * Structured read. Tolerant by design: it reads as many whole 4-byte entries as
     * the payload holds and reports any shortfall instead of throwing, so a reshaped
     * reply still prints something a human can read.
     */
    fun parse(payload: ByteArray): Table {
        if (payload.size < HEADER_SIZE) {
            return Table(-1, 0, emptyList(), payload.copyOf(), true)
        }
        val status = payload[0].toInt() and 0xFF
        val count = payload[1].toInt() and 0xFF
        val body = payload.copyOfRange(HEADER_SIZE, payload.size)

        val whole = body.size / ENTRY_SIZE
        val entries = ArrayList<Entry>(whole)
        for (i in 0 until whole) {
            val o = i * ENTRY_SIZE
            entries.add(
                Entry(
                    deviceType = body[o].toInt() and 0xFF,
                    button = body[o + 1].toInt() and 0xFF,
                    action = body[o + 2].toInt() and 0xFF,
                    function = body[o + 3].toInt() and 0xFF
                )
            )
        }
        val trailing = body.copyOfRange(whole * ENTRY_SIZE, body.size)
        // The count byte should account for every entry AND leave nothing over.
        val mismatch = count != whole || trailing.isNotEmpty()
        return Table(status, count, entries, trailing, mismatch)
    }

    /**
     * One log line for the reply, GROUPED PER DEVICE/BUTTON and ending with the raw
     * payload:
     *
     *     getKeyFunction reply: status=0x00 count=18 parsed=18
     *       dev=0x01/btn=0x01[01:00 02:11 ...] dev=0x01/btn=0x06[...] ...
     *
     * `act:fn` pairs, both as raw numbers. The grouping is the point: the reply is
     * read in order to be COMPARED with one taken after a HeyMelody change, and a
     * per-button list makes the single entry that moved obvious. Groups are sorted by
     * (device, button) so each bud's rows sit together; the entries inside a group
     * keep the buds' own order, which is itself a fact about their action enum.
     */
    fun describe(payload: ByteArray): String {
        val raw = OpoProtocol.bytesToHex(payload)
        if (payload.isEmpty()) return "getKeyFunction reply: EMPTY payload RAW=[]"

        val r = parse(payload)
        val sb = StringBuilder("getKeyFunction reply: status=0x")
        sb.append("%02X".format(r.status)).append(" count=").append(r.count)
            .append(" parsed=").append(r.entries.size)
        if (r.entries.isEmpty()) {
            sb.append(" (no whole ").append(ENTRY_SIZE).append("-byte entries)")
        }
        val groups = sortedMapOf<Int, StringBuilder>()
        for (e in r.entries) {
            val key = (e.deviceType shl 8) or e.button
            val g = groups.getOrPut(key) { StringBuilder() }
            if (g.isNotEmpty()) g.append(' ')
            g.append("%02X:%02X".format(e.action, e.function))
        }
        for ((key, g) in groups) {
            sb.append(" dev=0x").append("%02X".format((key shr 8) and 0xFF))
                .append("/btn=0x").append("%02X".format(key and 0xFF))
                .append('[').append(g).append(']')
        }
        if (r.trailing.isNotEmpty()) {
            sb.append(" trailing=[").append(OpoProtocol.bytesToHex(r.trailing))
            sb.append(']')
        }
        if (r.layoutMismatch) {
            sb.append(" !LAYOUT expected ").append(HEADER_SIZE).append('+').append(r.count)
                .append('*').append(ENTRY_SIZE)
                .append(" bytes, got ").append(payload.size)
        }
        sb.append(" RAW=[").append(raw).append(']')
        return sb.toString()
    }

    /**
     * Canonical, order-independent signature of a reading: one
     * `dev=0x01/btn=0x01 act=0x02 -> 0x11` token per entry, sorted.
     *
     * A PLAIN STRING on purpose, so it can be STORED between connections. The
     * experiment this exists for requires disconnecting our app — the vendor app needs
     * the RFCOMM socket — and reconnecting it, which may restart the process. An
     * in-memory baseline would be lost exactly in the gap it exists to span, and a lost
     * baseline is indistinguishable from "nothing changed".
     */
    fun signature(table: Table): String =
        table.entries
            .map { slotOf(it) + " -> " + "0x%02X".format(it.function) }
            .sorted()
            .joinToString("; ")

    /** `dev=0x01/btn=0x01 act=0x02` — the part of an entry a function is bound TO. */
    private fun slotOf(e: Entry): String =
        "dev=0x%02X/btn=0x%02X act=0x%02X".format(e.deviceType, e.button, e.action)

    /**
     * Describes ONLY what moved between two readings, which is the whole reason the
     * reply is read: change ONE gesture in the vendor app and the single `fn` that
     * moved names that function's value. The line is built for exactly that reading:
     *
     *     dev=0x01/btn=0x01 act=0x02: 0x00 -> 0x12
     *
     * A slot present in only ONE of the two is reported as added/removed, because a
     * missing entry is a different fact from a changed one — and on this firmware a
     * slot does appear and disappear as a binding is cleared.
     *
     * NO CHANGE is stated explicitly rather than returning an empty string, because it
     * is a REAL RESULT here: rebinding the ANC-cycle HOLD cannot move `fn` at all (the
     * ANC-cycle hold is offered only the ANC modes, so that hold cannot be bound to voice
     * assistant), so "nothing moved" is the expected outcome of that particular experiment
     * and must not be readable as a failure to run.
     */
    fun diff(previous: String, current: String): String {
        val before = parseSignature(previous)
        val after = parseSignature(current)
        if (before.isEmpty() && after.isEmpty()) return "unreadable baseline and reading"

        val changed = ArrayList<String>()
        for (slot in (before.keys + after.keys).sorted()) {
            val a = before[slot]
            val b = after[slot]
            when {
                a == null -> changed.add("$slot added: 0x%02X".format(b!!))
                b == null -> changed.add("$slot removed (was 0x%02X)".format(a))
                a != b -> changed.add("$slot: 0x%02X -> 0x%02X".format(a, b))
            }
        }
        return if (changed.isEmpty()) {
            "NO CHANGE (${after.size} slots identical)"
        } else {
            changed.joinToString(" | ")
        }
    }

    /** `dev=0x01/btn=0x01 act=0x02 -> 0x11` tokens -> slot to function. */
    private fun parseSignature(sig: String): Map<String, Int> {
        val out = HashMap<String, Int>()
        for (token in sig.split(';')) {
            if (token.isBlank()) continue
            val at = token.lastIndexOf(" -> ")
            if (at <= 0) continue
            val fn = parseHex(token.substring(at + 4)) ?: continue
            out[token.substring(0, at).trim()] = fn
        }
        return out
    }

    /**
     * Reads a `0x`-prefixed hex byte out of a signature, or null if it is not one.
     *
     * WHY NOT `toIntOrNull(16)`: that delegates to `Integer.parseInt`, which REJECTS a
     * `0x` prefix. Since [signature] writes exactly that prefix, the naive call returns
     * null for EVERY field, so [parseSignature] yields an empty map for both readings and
     * every diff silently reports "unreadable baseline and reading" — a failure that
     * looks like an inconclusive experiment rather than a bug. Strip the prefix first.
     */
    private fun parseHex(s: String): Int? {
        val t = s.trim()
        val body = if (t.startsWith("0x", ignoreCase = true)) t.substring(2) else t
        if (body.isEmpty()) return null
        return body.toIntOrNull(16)
    }
}

// ===== UserInteractionParser =====

/**
 * Decoder for the 0x0204 "user interaction" active report — the F1 family.
 *
 * Frame shape (captured on Buds 4, 5 samples across several sessions):
 *
 *     AA 0D 00 00 04 02 FF 06 00 F1 01 01 XX YY 02
 *     |  |  |  |  |  |  |  |  |  |  |  |  |  |  +-- payload[5] = context/field
 *     |  |  |  |  |  |  |  |  |  |  |  |  +--+----- payload[4] = function
 *     |  |  |  |  |  |  |  |  |  |  |  +---------- payload[3] = action
 *     |  |  |  |  |  |  |  |  |  |  +------------- payload[2] = button id
 *     |  |  |  |  |  |  |  |  +------------------- payload[1] = side (0x01 L, 0x02 R)
 *     |  |  |  |  |  |  |  +---------------------- payload[0] = subType 0xF1
 *     |  |  |  |  |  |  +------------------------- 0x06  (payload length)
 *     |  |  |  |  |  +---------------------------- 0xFF  (event marker)
 *     |  |  |  |  +------------------------------- 0x02  (sub-command)
 *     |  |  |  +---------------------------------- 0x04  (active report)
 *     +--+--+------------------------------------- length / flags
 *
 * SOURCE (not guessed): Zhaoyi-ya/OppoPodsManager, `Models/UserInteractionEventInfo.cs`.
 * That class is explicitly documented as the data body of "0x0204 subType=0xF1", derived
 * from HeyMelody `[VENDOR]`. Its field order matches this
 * frame byte for byte, including the trailing int16 options array.
 *
 *     byte0 = side        0x01 = left, 0x02 = right (confirmed by that project)
 *     byte1 = button id   model dependent (0x06 on Enco Air5 Pro)
 *     byte2 = action      0x00 single, 0x02 double, 0x03 triple,
 *                         0x04 long press, 0x07 slide up, 0x08 slide down
 *                         for the taps this is ALSO the key-function `act` — see below
 *     byte3 = the bound FUNCTION, as the buds report it — matching the 0x8108 reply's
 *                         `function` byte. See the correspondence below.
 *     byte4 = context
 *     then  = int16 options from payload[6], little endian
 *
 * THE OLD CLAIM HERE WAS WRONG — do not restore it. This comment used to say "ANC
 * changes made on the BUDS raise no 0x0204 subtype event — verified three times".
 * They DO: subType 0x03, mapped in AncEventParser, and the belief survived three
 * captures only because `noteUnattributed` blanket-excludes cmd 0x0204, so an
 * undecoded subType printed nothing at all (PROTOCOL-LEGACY.md §5).
 *
 * WHAT THE F1 FRAME IS FOR: it says WHEN, and which side and action. On a real ANC
 * hold, F1 lands FIRST and the 0x0204 subType 0x03 frame follows 1.3-2.0 s later.
 * So a capture window that stops at the F1 line misses the answer, and the F1 log
 * line stays diagnostic evidence, never a control signal.
 *
 * THE CONCLUSION THAT USED TO SIT HERE WAS TOO STRONG, and it is kept only so it is not
 * restored. It read "byte3 is identical for every gesture of the same action+side,
 * therefore it cannot say what the gesture did", with two long presses — one cycling ANC,
 * one firing voice assistant — offered as the proof. That example cannot exist: the
 * ANC-cycle hold is offered only the ANC modes, so that hold cannot be bound to voice
 * assistant, and a binding is one function per (device, button, action) anyway. The
 * observation never supported the conclusion.
 *
 * BYTE2 AND BYTE3 ARE THE KEY-FUNCTION MAPPING. PROTOCOL-LEGACY.md §6 asked whether byte3
 * carries the bound function. It does — and byte2 carries the key-function *action*
 * number, so the F1 frame is the key-function entry for the gesture that fired:
 *
 *     F1 payload      byte2  byte3   key-function entry      its fn
 *     01 01 01 01 0x    01     01     dev=01/btn=01 act 01     0x01
 *     01 01 02 06 0x    02     06     dev=01/btn=01 act 02     0x06
 *     01 01 03 05 0x    03     05     dev=01/btn=01 act 03     0x05
 *     01 01 04 08 0x    04     08     dev=01/btn=01 act 04     0x08
 *
 * Four independent slots, four DIFFERENT function values, all matching — that is not a
 * coincidence, and byte2 == the key-function `act` in every one of them. The F1 numbering
 * is therefore NOT the key-function numbering (as §6 already warned); byte2 here is
 * already a key-function `act`, not an F1 action id.
 *
 * AN EARLIER REVISION OF THIS COMMENT CLAIMED THE OPPOSITE, using this frame as its
 * counter-example:
 *
 *     payload F1 01 01 00 00 03   byte2 = 0x00, byte3 = 0x00 (parsed as "single tap")
 *     key-function table:         act 01 -> fn 0x01
 *
 * byte3 (0x00) is not fn (0x01), so the theory was declared dead. That was too hasty:
 * byte2 = 0x00 is not a key-function `act` at all (the table has none — its acts are
 * 1..6), so this frame is not the single-tap BINDING entry and is not comparable to one.
 * It is very likely a different event (a touch/tap report rather than a binding firing),
 * which also means the [OSS] reading of 0x00 as "single tap" is suspect.
 *
 * The frame carries the function: byte2 is the key-function act and byte3 its fn.
 * A slide reports 0B/0C or 05/06, so byte3 is the effective function, not the slot's own
 * `fn`. The frame is diagnostic only; do not use it as a control signal.
 */
object UserInteractionParser {

    /** SubType of the user-interaction active report. */
    const val EVT_USER_INTERACTION = 0xF1

    /** True if this 0x0204 payload is a user-interaction (button/gesture) event. */
    fun isUserInteractionEvent(payload: ByteArray): Boolean {
        if (payload.size < 3) return false
        return payload[0].toInt() and 0xFF == EVT_USER_INTERACTION
    }

    private fun sideName(v: Int): String = when (v) {
        0x01 -> "L"
        0x02 -> "R"
        else -> "?0x%02X".format(v)
    }

    private fun actionName(v: Int): String = when (v) {
        0x00 -> "single tap"
        0x02 -> "double tap"
        0x03 -> "triple tap"
        0x04 -> "long press"
        0x07 -> "slide up"
        0x08 -> "slide down"
        else -> "action=0x%02X".format(v)
    }

    /**
     * Human-readable one-liner for the log, e.g.
     * "L btn0x01 long press (ctx=0x02 opts=[2])".
     */
    fun describe(payload: ByteArray): String {
        if (!isUserInteractionEvent(payload)) return "not a user-interaction event"
        // payload[0] = 0xF1, fields start at payload[1].
        val side = payload.getOrElse(1) { 0 }.toInt() and 0xFF
        val button = payload.getOrElse(2) { 0 }.toInt() and 0xFF
        val action = payload.getOrElse(3) { 0 }.toInt() and 0xFF
        val modifier = payload.getOrElse(4) { 0 }.toInt() and 0xFF
        val context = payload.getOrElse(5) { 0 }.toInt() and 0xFF

        // Options: little-endian int16 pairs from payload[5] onward, matching
        // UserInteractionEventInfo's own walk (pos += 2 while pos+1 < end).
        val options = mutableListOf<Int>()
        var pos = 5
        while (pos + 1 < payload.size) {
            options.add(OpoProtocol.u16(payload, pos))
            pos += 2
        }

        val sb = StringBuilder()
        sb.append(sideName(side))
        sb.append(" btn=0x").append("%02X".format(button))
        sb.append(' ').append(actionName(action))
        if (modifier != 0) sb.append(" +0x").append("%02X".format(modifier))
        sb.append(" ctx=0x").append("%02X".format(context))
        if (options.isNotEmpty()) sb.append(" opts=[").append(options.joinToString(",")).append(']')
        return sb.toString()
    }
}

// ===== WearingStatusParser =====

/**
 * Wearing / in-case status parser for OnePlus Buds 4.
 *
 * Source: Zhaoyi-ya/OppoPodsManager (Services/PodManager.Parsing.cs, ParseWearingData),
 * cross-validated against HeyMelody `[VENDOR]` and live captures.
 *
 * Two packet sources carry the same payload:
 *   - 0x8109  response to query 0x0109 (getEarBudsStatus)
 *   - 0x0204  spontaneous notification, subType 0x02 (after 0x0205 registration)
 *
 * Payload: [count][comp, st] x N
 *   comp: 1 = Left bud, 2 = Right bud, 3 = Case
 *   st:   bits `[VENDOR]`: 0 out of the case, 1 in ear, 2 lid open, 3 a flag the app ignores.
 *         So 4 = in case, 1/5 = out, 3/7 = in ear, 0 = in case with the lid closed.
 *         The flag bit is cleared here, so the codes above stay the only ones.
 *
 * Buds 4 query responses sometimes prepend a status byte, so parsing tolerantly
 * tries both offsets and keeps the first layout that yields valid pairs.
 */
object WearingStatusParser {

    const val ST_IN_CASE = 4

    data class Result(
        val leftValid: Boolean,
        val rightValid: Boolean,
        val leftStatus: Int = -1,    // raw st for left bud, -1 = not reported
        val rightStatus: Int = -1,   // raw st for right bud, -1 = not reported
        val caseStatus: Int = -1     // raw st for case (comp 3), -1 = not reported
    )

    private fun isPlausibleSt(st: Int) = st in 0..15
    private fun isPlausibleComp(c: Int) = c == 1 || c == 2 || c == 3

    private fun tryParse(p: ByteArray, off: Int): Result? {
        if (p.size < off + 3) return null
        val count = p[off].toInt() and 0xFF
        if (count == 0 || count > 8) return null

        var leftValid = false
        var rightValid = false
        var leftStatus = -1
        var rightStatus = -1
        var caseStatus = -1
        var used = 0
        var pos = off + 1

        var i = 0
        while (i < count && pos + 1 < p.size) {
            val comp = p[pos].toInt() and 0xFF
            val raw = p[pos + 1].toInt() and 0xFF
            if (!isPlausibleComp(comp) || !isPlausibleSt(raw)) return null
            val st = raw and 0x07
            when (comp) {
                1 -> { leftValid = true; leftStatus = st }
                2 -> { rightValid = true; rightStatus = st }
                3 -> { caseStatus = st }
            }
            used++
            pos += 2
            i++
        }
        if (used == 0) return null
        return Result(leftValid, rightValid, leftStatus, rightStatus, caseStatus)
    }

    /** Parse a 0x8109 wearing query response payload. */
    fun parseQueryResponse(payload: ByteArray): Result? {
        tryParse(payload, 0)?.let { return it }   // strict: [count][pairs]
        return tryParse(payload, 1)               // tolerant: [status][count][pairs]
    }

    /** Parse a 0x0204 active-report payload (payload[0] must be subType 0x02). */
    fun parseActiveReport(payload: ByteArray): Result? {
        if (payload.isEmpty() || payload[0].toInt() and 0xFF != OpoProtocol.EVT_WEARING) return null
        return tryParse(payload, 1)
    }
}
