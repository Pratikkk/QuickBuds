package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol2.Wire.Message

/**
 * Typed messages of docs/PROTOCOL-NEXT.md: decodes what the buds send. Written from the spec, not from
 * the `protocol` package. Input is a [Message] from the link layer; output is [Msg].
 */

/** A MAC in written order. */
typealias Mac = List<Int>

/** One battery cell: part 1 left, 2 right, 3 case. */
data class Cell(val part: Int, val level: Int, val charging: Boolean)

sealed class Anc {
    /** The current mode as a bitmask (the report table, not the SET table). */
    data class Mode(val mask: Int) : Anc()
    /** The modes the hold cycles: [kind] 1 shared, 3 left, 4 right; [mask] in SET bits. */
    data class HoldCycle(val kind: Int, val mask: Int) : Anc()
    /** The level Smart chose, in SET bits. */
    data class Smart(val mask: Int) : Anc()
    /** `01 02 <level>`: a level report, never seen on our buds. */
    data class Level(val level: Int) : Anc()
}

/** One gesture binding: [dev] 1 left, 2 right, 4 both. */
data class Binding(val dev: Int, val button: Int, val act: Int, val func: Int)

/** One custom equaliser preset. [range] is echoed back on a save: -6 to 6 is `FA 06`. [bands] are (Hz, dB). */
data class EqPreset(val id: Int, val name: String, val selected: Boolean, val range: Pair<Int, Int>, val bands: List<Pair<Int, Int>>)

data class Device(val mac: Mac, val name: String, val connected: Boolean, val flags: Int) {
    val thisPhone get() = flags and 1 != 0
}

data class Linked(val mac: Mac, val times: Int, val name: String)

sealed class Public {
    data class AlertVolume(val level: Int) : Public()
    /** [auto]: the buds pick the device. [mac]: the chosen handheld. */
    data class Handheld(val auto: Boolean, val mac: Mac?) : Public()
    data class TapLevel(val level: Int) : Public()
    data class Other(val flags: Int) : Public()
}

/** Push `0D`. Each field is present when its flag is set and the bytes are there. */
data class Triangle(
    val capability: Int? = null, val version: Int? = null, val wear: Int? = null,
    /** Bit 0 phone has no active command, 1 connected by the related device, 2 HFP busy, 3 A2DP busy, 4 other device's auto switch closed. */
    val status: Int? = null, val relatedName: String? = null,
)

/** The commands the buds accept. Before the reply, every command is allowed. */
class Bitmap private constructor(private val cmds: Set<Int>?) {
    fun allows(cmd: Int) = cmds?.contains(cmd) ?: true
    val known get() = cmds != null

    companion object {
        val UNKNOWN = Bitmap(null)

        /** Commands behind each bit of the `0x8100` reply (PROTOCOL-NEXT.md section 4). */
        private val BITS: List<List<Int>> = listOf(
            listOf(0x0105), listOf(0x0106), listOf(0x0107), listOf(0x0108, 0x0401, 0x0416), listOf(0x0109), listOf(0x0400),
            listOf(0x0402), listOf(0x0403), listOf(0x010C, 0x0404), listOf(0x0405), listOf(0x0406, 0x010F), listOf(0x0407),
            listOf(), listOf(0x0408), listOf(0x0409), listOf(), listOf(), listOf(0x0114), listOf(),
            listOf(0x040E, 0x040D, 0x0115, 0x0116), listOf(0x040F), listOf(0x0410, 0x0119), listOf(0x0205), listOf(0x0F00),
            listOf(), listOf(0x0118, 0x0411), listOf(0x011A, 0x0412), listOf(0x011C, 0x0413), listOf(), listOf(0x0112, 0x040B),
            listOf(0x011E, 0x011F, 0x0415), listOf(0x040D), listOf(), listOf(0x0121, 0x0417), listOf(0x0122, 0x0418),
            listOf(0x0419), listOf(0x011D, 0x0414), listOf(0x0123, 0x041A), listOf(0x0124, 0x041B),
            listOf(0x0125, 0x041C, 0x0127, 0x041D, 0x041F), listOf(0x0421, 0x0023, 0x0024, 0x0022, 0x0126, 0x0129),
            listOf(0xEF01), listOf(0xEF02), listOf(0xEF03, 0x041E), listOf(0x0420), listOf(0x001C), listOf(),
            listOf(0x0422, 0x012A), listOf(0xEF04), listOf(0x0423, 0x012B), listOf(), listOf(0x0424), listOf(0xEF06),
            listOf(), listOf(), listOf(0x0425, 0x012E, 0x0426), listOf(0x012F), listOf(0x0427, 0x0130), listOf(0x0131, 0x0428),
            listOf(0x0429, 0x0132), listOf(0x0014), listOf(0x042D, 0x0133), listOf(0x042E), listOf(0xEF07),
            listOf(0xEF08, 0x0136, 0x0433), listOf(0xEF09), listOf(0x0431, 0x0134), listOf(0x0432, 0x0135, 0x0435), listOf(0x0117),
        )
        private val ALWAYS = listOf(0x0100, 0x0101, 0x0102, 0x0103, 0x0104, 0x0106, 0x010B, 0x010D, 0x0F00, 0x0F03, 0x0F04)

        /** `00` then bytes, least significant bit first. Bits past the table are ignored. */
        fun parse(p: ByteArray): Bitmap? {
            if (p.size < 2 || p[0].toInt() != 0) return null
            val out = HashSet(ALWAYS)
            BITS.forEachIndexed { bit, cmds ->
                if (1 + bit / 8 < p.size && (p[1 + bit / 8].toInt() shr (bit % 8)) and 1 == 1) out += cmds
            }
            return Bitmap(out)
        }
    }
}

/** A decoded message from the buds. */
sealed class Msg {
    // Replies (section 4)
    data class BitmapReply(val bitmap: Bitmap) : Msg()
    /** 24-bit product id, as the buds send it. */
    data class ProductId(val id: Int) : Msg()
    /** (device, version type, version); device 1 left, 2 right, 3 case. */
    data class Firmware(val parts: List<Triple<Int, Int, String>>) : Msg()
    data class Offered(val codes: List<Int>) : Msg()
    /** The `0x8205` reply. */
    object Registered : Msg() { override fun toString() = "Registered" }
    /** A write reply (`0x84xx`): the command written, the status (0 accepted) and the bytes after it. */
    data class Ack(val cmd: Int, val status: Int, val data: List<Int>) : Msg()
    data class Gestures(val table: List<Binding>) : Msg()
    /** `0x810D`: (feature id, value). */
    data class Features(val list: List<Pair<Int, Int>>) : Msg()
    data class EqCurrent(val id: Int) : Msg()
    data class EqCustom(val list: List<EqPreset>) : Msg()
    data class BassLevel(val level: Int) : Msg()

    // Pushes (section 5); the same facts also arrive as replies to a query
    data class Battery(val cells: List<Cell>) : Msg()
    data class Wear(val parts: List<Pair<Int, Int>>) : Msg()
    data class AncMsg(val anc: Anc) : Msg()
    data class FitTest(val left: Int?, val right: Int?) : Msg()
    /** Both candidates; the model decides which one is the state. */
    data class GameMode(val at1: Int?, val at3: Int?) : Msg()
    data class Devices(val list: List<Device>) : Msg()
    data class Hearing(val kind: Int, val status: Int) : Msg()
    data class ZenState(val state: Int) : Msg()
    data class PersonalAnc(val result: Int) : Msg()
    data class TriangleMsg(val info: Triangle) : Msg()
    data class EarScan(val id: Int, val data: List<Int>) : Msg()
    data class PublicMsg(val event: Public) : Msg()
    data class Gesture(val side: Int, val button: Int, val act: Int, val func: Int, val context: Int, val options: List<Int>) : Msg()
    data class LinkedMsg(val list: List<Linked>) : Msg()
    data class Diagnostic(val json: String) : Msg()
    data class HeadGesture(val type: Int) : Msg()
    data class HeadTry(val result: Int) : Msg()
    /** [kind]: 0, 1 or 2 from flag bits 0-2; [flag]: bit 7. */
    data class VoiceAssist(val kind: Int?, val flag: Boolean) : Msg()
    data class LinkSwitch(val state: Int, val owner: Int, val reset: Int) : Msg()
    /** A push the spec names but no feature uses (10, 11, F3, F9, ...). */
    data class Unused(val code: Int, val data: List<Int>) : Msg()
}

object Decode {
    const val PUSH = 0x0204
    private const val REPLY = 0x8000
    private val NAMED = setOf(1, 2, 3, 4, 5, 6, 8, 0x0A, 0x0B, 0x0D, 0x0E, 0x0F, 0xF1, 0xF2, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8)

    private fun u(b: ByteArray, i: Int) = b[i].toInt() and 0xFF
    private fun le16(b: ByteArray, i: Int): Int? = if (i + 2 <= b.size) u(b, i) or (u(b, i + 1) shl 8) else null
    private fun be32(b: ByteArray, i: Int): Int? =
        if (i >= 0 && i + 4 <= b.size) (u(b, i) shl 24) or (u(b, i + 1) shl 16) or (u(b, i + 2) shl 8) or u(b, i + 3) else null
    /** 1 to 4 bytes, little endian. */
    private fun leN(b: ByteArray, from: Int): Int = (from until minOf(b.size, from + 4)).reversed().fold(0) { v, i -> v shl 8 or u(b, i) }
    /** A MAC on the wire is reversed; this returns it in written order. */
    private fun mac(b: ByteArray, i: Int): Mac? = if (i + 6 <= b.size) (5 downTo 0).map { u(b, i + it) } else null
    private fun ints(b: ByteArray, from: Int = 0) = (from until b.size).map { u(b, it) }

    /** Decodes one message. null: not a message this layer knows, or a malformed one. */
    fun decode(m: Message): Msg? {
        val p = m.payload
        if (m.cmd == PUSH) return push(p)
        return when (m.cmd) {
            0x8100 -> Bitmap.parse(p)?.let { Msg.BitmapReply(it) }
            0x8103 -> if (p.size == 4 && p[0].toInt() == 0) Msg.ProductId(u(p, 1) or (u(p, 2) shl 8) or (u(p, 3) shl 16)) else null
            0x8105 -> firmware(p)?.let { Msg.Firmware(it) }
            0x8200 -> if (p.size >= 2 && p[0].toInt() == 0 && p.size >= 2 + u(p, 1)) Msg.Offered(ints(p, 2).take(u(p, 1))) else null
            0x8205 -> Msg.Registered
            0x8106 -> ok(p)?.let { battery(it, 0) }?.let { Msg.Battery(it) }
            0x8109 -> {
                // `<count> <pairs>`, or a status byte first; a list is valid when its parts are 1-3.
                val list = pairs(p, 0)?.takeIf { l -> l.all { it.first in 1..3 } }
                    ?: pairs(p, 1)?.takeIf { l -> l.all { it.first in 1..3 } }
                list?.let { l -> Msg.Wear(l.map { it.first to (it.second and 7) }) }
            }
            0x810C -> ok(p)?.let { anc(it, 0) }?.let { Msg.AncMsg(it) }
            0x8108 -> {
                if (p.size >= 2 && p[0].toInt() == 0) {
                    val n = minOf(u(p, 1), (p.size - 2) / 4)
                    // A reply with no entries is no table: a write is built from the last good one.
                    if (n == 0) null else Msg.Gestures((0 until n).map { Binding(u(p, 2 + it * 4), u(p, 3 + it * 4), u(p, 4 + it * 4), u(p, 5 + it * 4)) })
                } else null
            }
            0x810D -> ok(p)?.let { pairs(it, 0) }?.let { Msg.Features(it) }
            0x810F -> ok(p)?.takeIf { it.isNotEmpty() }?.let { Msg.EqCurrent(u(it, 0)) }
            0x8122 -> ok(p)?.let { presets(it) }?.let { Msg.EqCustom(it) }
            0x8124 -> ok(p)?.takeIf { it.size >= 3 }?.let { Msg.BassLevel(it[2].toInt()) }
            0x0504 -> if (p.isNotEmpty()) Msg.EqCurrent(u(p, 0)) else null
            else -> if (m.cmd and 0xFF00 == 0x8400 && p.isNotEmpty()) Msg.Ack(m.cmd and 0x7FFF, u(p, 0), ints(p, 1)) else null
        }
    }

    /** The bytes after a `00` status byte. A reply with another status carries no data. */
    private fun ok(p: ByteArray): ByteArray? = if (p.isNotEmpty() && p[0].toInt() == 0) p.copyOfRange(1, p.size) else null

    /** ANC data: `01 01 <mode LE>`, `01 02 <level>`, `02 <kind> <mask LE>`, `04 01 <mask LE>`. Used by `0x810C` and push `03`. */
    private fun anc(p: ByteArray, from: Int): Anc? {
        val n = p.size - from
        fun d(i: Int) = u(p, from + i)
        return when {
            n >= 3 && d(0) == 1 && d(1) == 1 -> Anc.Mode(leN(p, from + 2))
            n >= 3 && d(0) == 1 && d(1) == 2 -> Anc.Level(d(2))
            n >= 3 && d(0) == 2 -> Anc.HoldCycle(d(1), leN(p, from + 2))
            n >= 3 && d(0) == 4 && d(1) == 1 -> Anc.Smart(leN(p, from + 2))
            else -> null
        }
    }

    /** `<count>` then per preset `<flag> <min> <max> <id> <nameLen> <name> <bands> [<freq u16 LE> <gain s8>]...`. */
    private fun presets(p: ByteArray): List<EqPreset>? {
        if (p.isEmpty()) return null
        var i = 1
        val out = ArrayList<EqPreset>()
        repeat(u(p, 0)) {
            if (i + 5 > p.size) return null
            val k = u(p, i + 4)
            if (i + 6 + k > p.size) return null
            val bands = u(p, i + 5 + k)
            if (i + 6 + k + bands * 3 > p.size) return null
            val at = i + 6 + k
            out += EqPreset(
                u(p, i + 3), String(p, i + 5, k, Charsets.UTF_8), u(p, i) == 1, p[i + 1].toInt() to p[i + 2].toInt(),
                (0 until bands).map { (u(p, at + it * 3) or (u(p, at + it * 3 + 1) shl 8)) to p[at + it * 3 + 2].toInt() },
            )
            i = at + bands * 3
        }
        return out
    }

    /** `00 <count>` then UTF-8 `device,type,version` triples joined by commas. */
    private fun firmware(p: ByteArray): List<Triple<Int, Int, String>>? {
        if (p.size < 2 || p[0].toInt() != 0) return null
        val parts = String(p, 2, p.size - 2, Charsets.UTF_8).split(',').map { it.trim() }
        val v = parts.chunked(3).filter { it.size == 3 }.mapNotNull { c ->
            val d = c[0].toIntOrNull(); val t = c[1].toIntOrNull()
            if (d == null || t == null) null else Triple(d, t, c[2])
        }
        return v.ifEmpty { null }
    }

    /** `<count>` then `<a> <b>` pairs from [from]. null when the pairs run past the end; a count of 0 is no list. */
    private fun pairs(p: ByteArray, from: Int = 0): List<Pair<Int, Int>>? {
        if (from >= p.size) return null
        val n = u(p, from)
        if (n == 0 || p.size - from - 1 < n * 2) return null
        return (0 until n).map { u(p, from + 1 + it * 2) to u(p, from + 2 + it * 2) }
    }

    /** Level 0 means "not known" (a closed case): left out. */
    private fun battery(p: ByteArray, from: Int): List<Cell>? =
        pairs(p, from)?.filter { it.second and 0x7F != 0 }?.map { Cell(it.first, it.second and 0x7F, it.second and 0x80 != 0) }

    /** `<count>` then `<MAC 6> <len> <state> <flags> <nameLen> <name>`. */
    fun devices(p: ByteArray, from: Int = 0): List<Device>? {
        if (from >= p.size) return null
        var i = from + 1
        val out = ArrayList<Device>()
        repeat(u(p, from)) {
            if (i + 10 > p.size) return null
            val k = u(p, i + 9)
            if (i + 10 + k > p.size) return null
            out += Device(mac(p, i)!!, String(p, i + 10, k, Charsets.UTF_8), u(p, i + 7) == 2, u(p, i + 8))
            i += 10 + k
        }
        return out
    }

    /** `<count>` then `<MAC 6> <times u16 LE> <nameLen> <name>`. */
    private fun linked(p: ByteArray, from: Int): List<Linked>? {
        if (from >= p.size) return null
        var i = from + 1
        val out = ArrayList<Linked>()
        repeat(u(p, from)) {
            if (i + 9 > p.size) return null
            val k = u(p, i + 8)
            if (i + 9 + k > p.size) return null
            out += Linked(mac(p, i)!!, le16(p, i + 6)!!, String(p, i + 9, k, Charsets.UTF_8))
            i += 9 + k
        }
        return out
    }

    private fun public(p: ByteArray, from: Int): Public? {
        if (p.size - from < 3) return null
        val flags = le16(p, from)!!
        val d = from + 2
        return when {
            flags and 1 != 0 -> Public.AlertVolume(u(p, d))
            flags and 2 != 0 -> Public.Other(flags)
            flags and 4 != 0 -> {
                val auto = u(p, d) == 0
                if (auto) Public.Handheld(true, null) else if (p.size - from > 8) Public.Handheld(false, mac(p, d + 1)) else null
            }
            flags and 16 != 0 -> Public.TapLevel(u(p, d))
            else -> Public.Other(flags)
        }
    }

    private fun triangle(p: ByteArray, from: Int): Triangle? {
        val flags = le16(p, from) ?: return null
        var i = from + 2
        fun byte(): Int? = if (i < p.size) u(p, i++) else null
        var t = Triangle()
        if (i >= p.size) return t
        if (flags and 1 != 0) t = t.copy(capability = byte())
        if (flags and 2 != 0) t = t.copy(version = byte())
        if (flags and 4 != 0) t = t.copy(wear = byte())
        if (flags and 8 != 0) t = t.copy(status = byte())
        if (flags and 16 != 0) byte()?.let { k -> if (i + k <= p.size) t = t.copy(relatedName = String(p, i, k, Charsets.UTF_8)) }
        return t
    }

    /** `payload[0]` is the event code. */
    private fun push(p: ByteArray): Msg? {
        if (p.isEmpty()) return null
        val code = u(p, 0)
        val n = p.size - 1  // bytes after the code
        fun d(i: Int) = u(p, 1 + i)
        return when {
            code == 0x01 -> battery(p, 1)?.let { Msg.Battery(it) }
            code == 0x02 -> pairs(p, 1)?.let { l -> Msg.Wear(l.map { it.first to (it.second and 7) }) }
            code == 0x03 -> anc(p, 1)?.let { Msg.AncMsg(it) }
            code == 0x04 -> {
                val r = (0 until n / 2).map { d(it * 2) to d(it * 2 + 1) }
                Msg.FitTest(r.firstOrNull { it.first == 1 }?.second, r.firstOrNull { it.first == 2 }?.second)
            }
            code == 0x05 && n >= 1 -> Msg.GameMode(d(0), if (n >= 3) d(2) else null)
            code == 0x06 -> devices(p, 1)?.let { Msg.Devices(it) }
            code == 0x08 && n >= 2 -> Msg.Hearing(d(0), d(1))
            code == 0x0A && n >= 1 -> Msg.ZenState(d(0))
            code == 0x0B && n >= 1 -> Msg.PersonalAnc(d(0))
            code == 0x0D -> triangle(p, 1)?.let { Msg.TriangleMsg(it) }
            code == 0x0E -> {
                val len = le16(p, 2) ?: return null
                val id = be32(p, 4 + len)
                if (len == 0 || id == null) null else Msg.EarScan(id, ints(p, 4).take(len))
            }
            code == 0x0F -> public(p, 1)?.let { Msg.PublicMsg(it) }
            code == 0xF1 && n >= 5 -> Msg.Gesture(
                d(0), d(1), d(2), d(3), d(4),
                (0 until (n - 5) / 2).map { (d(5 + it * 2) or (d(6 + it * 2) shl 8)).toShort().toInt() }
            )
            code == 0xF2 -> linked(p, 1)?.let { Msg.LinkedMsg(it) }
            code == 0xF4 -> Msg.Diagnostic(String(p, 1, n, Charsets.UTF_8))
            code == 0xF5 && n >= 1 -> Msg.HeadGesture(d(0))
            code == 0xF6 && n >= 1 -> Msg.HeadTry(d(0))
            code == 0xF7 && n >= 1 -> Msg.VoiceAssist(
                listOf(4 to 2, 2 to 1, 1 to 0).firstOrNull { d(0) and it.first != 0 }?.second, d(0) and 0x80 != 0
            )
            code == 0xF8 && n >= 1 -> Msg.LinkSwitch(d(0) and 1, (d(0) shr 1) and 1, (d(0) shr 2) and 1)
            // A named code with too few bytes is malformed and dropped.
            code !in NAMED -> Msg.Unused(code, ints(p, 1))
            else -> null
        }
    }

    /** `0x812F`: `00 <count>`, then `<cmd LE> <len LE> <reply payload>`. Each answer is the reply `cmd | 0x8000` with the batch's seq. */
    fun unbatch(m: Message): List<Message> {
        val p = m.payload
        if (p.size < 2 || p[0].toInt() != 0) return emptyList()
        val out = ArrayList<Message>()
        var i = 2
        repeat(u(p, 1)) {
            if (i + 4 > p.size) return out
            val len = le16(p, i + 2)!!
            if (i + 4 + len > p.size) return out
            out += Message((le16(p, i)!!) or REPLY, m.seq, p.copyOfRange(i + 4, i + 4 + len))
            i += 4 + len
        }
        return out
    }
}
