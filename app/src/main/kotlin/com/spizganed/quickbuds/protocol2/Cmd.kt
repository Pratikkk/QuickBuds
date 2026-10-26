package com.spizganed.quickbuds.protocol2

/**
 * Typed messages of docs/PROTOCOL.md: builds what the app sends. Each function returns a [Frame]
 * (command, payload); the link layer adds the sequence number and the frame. [seq] holds the fixed
 * number of the battery and wearing queries. Written from the spec.
 */
data class Frame(val cmd: Int, val payload: ByteArray, val seq: Int? = null) {
    override fun equals(other: Any?) =
        other is Frame && cmd == other.cmd && seq == other.seq && payload.contentEquals(other.payload)
    override fun hashCode() = (cmd * 31 + (seq ?: 0)) * 31 + payload.contentHashCode()
    override fun toString() = "Frame(%04X %s)".format(cmd, payload.joinToString(" ") { "%02X".format(it) })
}

object Cmd {
    private fun f(cmd: Int, vararg v: Int) = Frame(cmd, ByteArray(v.size) { v[it].toByte() })

    // --- Connection (section 6) ---

    fun handshake() = f(0x0100)
    fun productId() = f(0x0103)
    fun broadcastCodes() = f(0x0200)

    /** `0x0205 <count> <codes>`. Count first: `01 01 02 02` would mean "count 1, battery only". */
    fun register(codes: List<Int>) = f(0x0205, codes.size, *codes.toIntArray())
    /** `0x0201 <code>`, for buds without `0x0205` in the bitmap. */
    fun registerOne(code: Int) = f(0x0201, code)

    /**
     * The events to subscribe to: battery, wearing, ANC, plus fit test, hearing test and personalised ANC where the
     * bitmap has their commands. Kept to the codes the buds offer when they answered `0x0200`.
     */
    fun notifyIds(bitmap: Bitmap, offered: List<Int>?): List<Int> {
        val ids = mutableListOf(0x01, 0x02, 0x03)
        if (bitmap.allows(0x0405)) ids += 0x04
        if (bitmap.allows(0x040D)) ids += 0x08
        if (bitmap.allows(0x0412)) ids += 0x0B
        return if (offered == null) ids else ids.filter { it in offered }
    }

    /** The id the model list uses: four colour ranges fold into one id. */
    fun foldProduct(id: Int): Int = when (id) {
        in 0x100100..0x100102 -> 0x060414
        in 0x100200..0x100202 -> 0x060814
        in 0x108100..0x108102 -> 0x068414
        in 0x108200..0x108202 -> 0x068814
        else -> id
    }

    /** Six hex digits, as the model list writes ids: `065414`. */
    fun productText(id: Int) = "%06X".format(foldProduct(id))

    /** The `versionType` 2 versions in reply order, joined by dots. */
    fun firmwareText(parts: List<Triple<Int, Int, String>>): String? =
        parts.filter { it.second == 2 }.map { it.third }.takeIf { it.isNotEmpty() }?.joinToString(".")

    /** `0x012F`: `<count>` then `<cmd LE> <len LE> <payload>` per query. */
    fun batch(queries: List<Frame>): Frame {
        val out = java.io.ByteArrayOutputStream()
        out.write(queries.size)
        for (q in queries) {
            out.write(q.cmd and 0xFF); out.write(q.cmd shr 8)
            out.write(q.payload.size and 0xFF); out.write(q.payload.size shr 8)
            out.write(q.payload)
        }
        return Frame(0x012F, out.toByteArray())
    }

    // --- Reads ---

    /** `0x0106` with the fixed sequence `0xF0`, as legacy `queryBattery` (`LegacyProtocol.kt` 391). */
    fun battery() = Frame(0x0106, ByteArray(0), seq = 0xF0)
    /** `0x0109` with the fixed sequence `0xF2`, as legacy `queryWearingStatus` (`LegacyProtocol.kt` 227). */
    fun wearing() = Frame(0x0109, ByteArray(0), seq = 0xF2)
    fun firmware() = f(0x0105)
    /** `0x010C <what> <arg>`: `01 01` current mode, `02 01` shared hold cycle (`02 03` left, `02 04` right), `04 01` the level Smart chose. */
    fun ancRead(what: Int, arg: Int) = f(0x010C, what, arg)
    fun gestures() = f(0x0108)
    /** `0x010D <count> <ids>`. */
    fun features(ids: List<Int>) = f(0x010D, ids.size, *ids.toIntArray())
    fun eqCurrent() = f(0x010F)
    fun eqCustom() = f(0x0122)
    fun bassLevel() = f(0x0124)

    // --- Writes ---

    /** `0x0404 01 01 <mask LE>`: one bit set, `bit / 8 + 1` mask bytes. [bit] is the model's SET slot. */
    fun ancSet(bit: Int): Frame {
        val p = ByteArray(2 + bit / 8 + 1)
        p[0] = 1; p[1] = 1
        p[2 + bit / 8] = (1 shl (bit % 8)).toByte()
        return Frame(0x0404, p)
    }

    /** `0x0404 02 <kind> <mask LE>`: the modes the hold cycles. The mask takes the fewest bytes, 1 to 4. */
    fun holdCycle(kind: Int, mask: Int): Frame {
        val n = (4 downTo 1).first { it == 1 || (mask ushr ((it - 1) * 8)) and 0xFF != 0 }
        return Frame(0x0404, byteArrayOf(2, kind.toByte()) + ByteArray(n) { (mask ushr (it * 8)).toByte() })
    }

    /** `0x0401 <count> <entries>`: the whole table, every time. */
    fun gestureTable(table: List<Binding>): Frame =
        f(0x0401, table.size, *table.flatMap { listOf(it.dev, it.button, it.act, it.func) }.toIntArray())

    /** `0x0403 <id> <01|00>`. */
    fun feature(id: Int, on: Boolean) = f(0x0403, id, if (on) 1 else 0)

    /** `0x0406 <id>`: select a built-in preset. */
    fun eqSelect(id: Int) = f(0x0406, id)

    /** What `0x0418` does with a preset. */
    enum class EqAction(val code: Int) {
        /** The buds assign the id (send 0) and return it in the reply. */
        Create(1),
        /** The whole preset, every time: also on every drag step and every rename. It selects the preset. */
        Save(2),
        /** The selection falls back to 0. */
        Delete(3),
    }

    /** `0x0418 <action> <min> <max> <id> <nameLen> <name> <bands> [<freq u16 LE> <gain s8>]...` */
    fun eqPreset(action: EqAction, p: EqPreset): Frame {
        val name = p.name.toByteArray(Charsets.UTF_8)
        val out = java.io.ByteArrayOutputStream()
        out.write(byteArrayOf(action.code.toByte(), p.range.first.toByte(), p.range.second.toByte(), p.id.toByte(), name.size.toByte()))
        out.write(name)
        out.write(p.bands.size)
        for ((hz, db) in p.bands) { out.write(hz and 0xFF); out.write(hz shr 8); out.write(db and 0xFF) }
        return Frame(0x0418, out.toByteArray())
    }

    /** `0x041B FB 05 <level>`: BassWave level, -5 to 5. The BassWave switch is feature `1D`. */
    fun bassSet(level: Int) = f(0x041B, 0xFB, 0x05, level and 0xFF)

    // --- Sound and device settings (section 13) ---

    fun alertVolumeRead() = f(0x0130)
    /** `0x0427 <level>`, 1..10. Each write plays a prompt: send on slider release only. */
    fun alertVolume(level: Int) = f(0x0427, level)
    fun tapLevelRead() = f(0x0133)
    /** `0x042D <level>`, 1..5. A lower level triggers more easily. */
    fun tapLevel(level: Int) = f(0x042D, level)
    fun headGestureRead() = f(0x0134)
    /** `0x0431 <type>`: 0 nod answers and shake declines, 1 the reverse. */
    fun headGesture(type: Int) = f(0x0431, type)
    fun spatialRead() = f(0x012A)
    /** `0x0422 <type>`: 0 off, 1 fixed, 2 head tracking. */
    fun spatial(type: Int) = f(0x0422, type)
    /** `0x041E <source>`: 0 the buds render 3D audio, 1 the phone does. */
    fun spatialSource(source: Int) = f(0x041E, source)
    fun codecRead() = f(0x0114)
    fun codecListRead() = f(0x0123)
    /** `0x041A <codec> <hiRes> 00`. The Hi-Res byte is set only with LDAC (3) or LHDC V5 (8). */
    fun codec(codec: Int, hiRes: Boolean) = f(0x041A, codec, if (hiRes && (codec == 3 || codec == 8)) 1 else 0, 0)
    fun gameSoundRead() = f(0x012B)
    /** `0x0423 <type> 01`; type 0 is off. */
    fun gameSound(type: Int) = f(0x0423, type, 1)
    /** `0x0420 01 <state>`: a game app is in the foreground. Applies only while feature `21` is on. */
    fun gameEq(inGame: Boolean) = f(0x0420, 1, if (inGame) 1 else 0)

    /** The codecs in a mask, as the app offers them: LHDC (7) is dropped when LHDC V5 (8) is offered too. */
    fun codecsOf(mask: Int): List<Int> {
        val all = (1..8).filter { mask and (1 shl (it - 1)) != 0 }
        return if (8 in all) all.filter { it != 7 } else all
    }

    /** `0x0400 <01|00>`: ring both buds. */
    fun find(on: Boolean) = f(0x0400, if (on) 1 else 0)
    /** `0x0405 <01|00>`: fit test. The result is push `04`. */
    fun fitTest(on: Boolean) = f(0x0405, if (on) 1 else 0)
    /** `0x0412 <action>`: 1 test, 2 use the stored result, 3 cancel. */
    fun personalAnc(action: Int) = f(0x0412, action)
    fun personalAncStored() = f(0x011A)

    // --- Dual connection (section 14) ---

    fun devicesRead() = f(0x0112)
    /** `0x0429 01|02 <MAC>`: connect or disconnect a device. The MAC is in written order. */
    fun deviceConnect(mac: Mac, on: Boolean) = f(0x0429, if (on) 1 else 2, *mac.toIntArray())
    /** `0x0429 04 00` automatic, `04 01 <MAC>` a chosen device. */
    fun preferredSet(mac: Mac?) = if (mac == null) f(0x0429, 4, 0) else f(0x0429, 4, 1, *mac.toIntArray())
    fun preferredRead() = f(0x0132, 2)

    /** The phone's state for switching between devices. A null field is left out. */
    data class PhoneState(val version: Int? = null, val screenOn: Boolean? = null, val held: Boolean? = null, val autoSwitch: Boolean? = null, val music: Boolean? = null)

    /** `0x0413 <flags> 00 <values>`: flag bit 0 protocol version, 1 screen on, 2 held by hand, 3 auto switch on, 4 music playing. */
    fun phoneState(s: PhoneState): Frame {
        val fields = listOf(s.version, s.screenOn?.let { if (it) 1 else 0 }, s.held?.let { if (it) 1 else 0 },
            s.autoSwitch?.let { if (it) 1 else 0 }, s.music?.let { if (it) 1 else 0 })
        val flags = fields.withIndex().fold(0) { acc, (i, v) -> if (v != null) acc or (1 shl i) else acc }
        return f(0x0413, flags, 0, *fields.filterNotNull().toIntArray())
    }

    // --- Golden Sound (section 16) ---

    private fun be32(v: Int) = byteArrayOf((v ushr 24).toByte(), (v ushr 16).toByte(), (v ushr 8).toByte(), v.toByte())
    private fun le16(v: Int) = byteArrayOf(v.toByte(), (v ushr 8).toByte())
    /** 12 hearing values, left ears 1-6 then right ears 1-6, as `<side> <freq> <value>`. */
    private fun hearingInfo(values: List<Int>) = ByteArray(36) { i ->
        val n = i / 3
        when (i % 3) { 0 -> (n / 6 + 1).toByte(); 1 -> (n % 6 + 1).toByte(); else -> values[n].toByte() }
    }

    fun goldenRecordRead() = f(0x0115)
    fun goldenScanRead() = f(0x011E)
    /** `0x040D 04 <01|00> <id u32 BE>`: ear scan. The result is push `0E`. */
    fun earScan(on: Boolean, id: Int) = Frame(0x040D, byteArrayOf(4, if (on) 1 else 0) + be32(id))
    /** `0x040D 02 <01|00>`: hearing test. */
    fun hearingTest(on: Boolean) = f(0x040D, 2, if (on) 1 else 0)
    /** `0x040E 03 01 <side> <freq> <value>`: a tone on one bud. Send [toneStop] before each new level. */
    fun tone(side: Int, freq: Int, value: Int) = f(0x040E, 3, 1, side, freq, value and 0xFF)
    fun toneStop() = f(0x040E, 4)
    /** `0x0116 0C <12 values> <id>`: the hearing filters of a result. */
    fun goldenFilter(id: Int, values: List<Int>) = Frame(0x0116, byteArrayOf(0x0C) + hearingInfo(values) + be32(id))
    /** `0x011F <len u16 LE> <scan> <id>`: the ear-scan filters. */
    fun goldenScanFilter(id: Int, scan: ByteArray) = Frame(0x011F, le16(scan.size) + scan + be32(id))

    /** Applies a record: the description id, the record, the ear scan when there is one, then the switch on. [desc]: 0 for none. */
    fun goldenApply(id: Int, name: String, values: List<Int>, scan: ByteArray, desc: Int): List<Frame> {
        val out = ArrayList<Frame>()
        if (desc > 0) out += f(0x0411, 1, 1, 1, 0, desc)
        out += Frame(0x040E, byteArrayOf(3, 0x0C) + hearingInfo(values) + be32(id) + name.toByteArray(Charsets.UTF_8))
        if (scan.isNotEmpty()) out += Frame(0x0415, byteArrayOf(3) + le16(scan.size) + scan + be32(id))
        out += feature(0x0B, true)
        return out
    }
}
