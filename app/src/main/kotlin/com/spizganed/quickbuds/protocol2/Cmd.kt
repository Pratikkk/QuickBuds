package com.spizganed.quickbuds.protocol2

/**
 * Typed messages of docs/PROTOCOL-NEXT.md: builds what the app sends. Each function returns a [Frame]
 * (command, payload); the link layer adds the sequence number and the frame. Written from the spec.
 */
data class Frame(val cmd: Int, val payload: ByteArray) {
    override fun equals(other: Any?) = other is Frame && cmd == other.cmd && payload.contentEquals(other.payload)
    override fun hashCode() = cmd * 31 + payload.contentHashCode()
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

    fun battery() = f(0x0106)
    fun wearing() = f(0x0109)
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
}
