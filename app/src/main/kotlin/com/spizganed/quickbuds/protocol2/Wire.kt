package com.spizganed.quickbuds.protocol2

/**
 * The link layer of docs/PROTOCOL.md §2: frames, slices, reassembly. No Bluetooth, no commands.
 * Plain Kotlin, so a JVM test runs it.
 */
object Wire {
    /** One message: what a frame, or the slices of one message, carry. */
    data class Message(val cmd: Int, val seq: Int, val payload: ByteArray) {
        override fun equals(other: Any?) = other is Message && cmd == other.cmd && seq == other.seq && payload.contentEquals(other.payload)
        override fun hashCode() = (cmd * 31 + seq) * 31 + payload.contentHashCode()
    }

    /** The largest frame the sender writes unless the buds report a smaller one. */
    const val BUDGET = 660
    /** A longer `Len` is noise, not a frame. */
    private const val MAX_LEN = 4096
    private const val START = 0xAA.toByte()

    /** Bytes `Len` takes: LEB128, 7 bits per byte. */
    private fun lenSize(len: Int) = if (len < 1 shl 7) 1 else if (len < 1 shl 14) 2 else 3

    private fun frameSize(data: Int, sliced: Boolean): Int {
        val len = 2 + (if (sliced) 1 else 0) + data
        return 1 + lenSize(len) + len
    }

    private fun frame(slice: Int, fsn: Int?, data: ByteArray, from: Int, to: Int): ByteArray {
        val len = 2 + (if (fsn != null) 1 else 0) + (to - from)
        val out = java.io.ByteArrayOutputStream(1 + lenSize(len) + len)
        out.write(START.toInt())
        var v = len
        do {
            val b = v and 0x7F
            v = v ushr 7
            out.write(if (v != 0) b or 0x80 else b)
        } while (v != 0)
        out.write(slice)
        out.write(0)
        if (fsn != null) out.write(fsn)
        out.write(data, from, to - from)
        return out.toByteArray()
    }

    /** The frames that carry [m]: one whole frame, or slices when the whole frame exceeds [budget]. */
    fun encode(m: Message, budget: Int = BUDGET): List<ByteArray> {
        val n = m.payload.size
        val msg = ByteArray(5 + n)
        msg[0] = m.cmd.toByte(); msg[1] = (m.cmd shr 8).toByte(); msg[2] = m.seq.toByte()
        msg[3] = n.toByte(); msg[4] = (n shr 8).toByte()
        m.payload.copyInto(msg, 5)
        if (frameSize(msg.size, false) <= budget) return listOf(frame(0, null, msg, 0, msg.size))
        // The most data a slice holds, at least 1 byte so a tiny budget still ends.
        val per = (budget downTo 1).firstOrNull { frameSize(it, true) <= budget } ?: 1
        val count = (msg.size + per - 1) / per
        return (0 until count).map { i ->
            frame(if (i == 0) 1 else if (i == count - 1) 3 else 2, i, msg, i * per, minOf(msg.size, (i + 1) * per))
        }
    }

    /** Turns the byte stream into messages. Noise before a frame is skipped and handed to [onDiscard]. */
    class Reassembler(var onDiscard: ((ByteArray) -> Unit)? = null) {
        private var buf = ByteArray(0)
        private var slices: java.io.ByteArrayOutputStream? = null

        fun push(bytes: ByteArray, length: Int = bytes.size): List<Message> {
            buf += bytes.copyOfRange(0, length)
            val out = mutableListOf<Message>()
            while (buf.isNotEmpty()) {
                val start = buf.indexOf(START)
                if (start < 0) { onDiscard?.invoke(buf); buf = ByteArray(0); break }
                if (start > 0) { onDiscard?.invoke(buf.copyOfRange(0, start)); buf = buf.copyOfRange(start, buf.size) }
                val length = readLen(buf)
                if (length == null) {
                    // Three length bytes with no terminator cannot be a header: drop the AA and keep scanning.
                    if (buf.size > 3) {
                        onDiscard?.invoke(buf.copyOfRange(0, 1))
                        buf = buf.copyOfRange(1, buf.size)
                        continue
                    }
                    break
                }
                val (len, n) = length
                if (len < 2 || len > MAX_LEN) { onDiscard?.invoke(buf.copyOfRange(0, 1)); buf = buf.copyOfRange(1, buf.size); continue }
                val end = 1 + n + len
                if (buf.size < end) break
                val body = buf.copyOfRange(1 + n, end)
                buf = buf.copyOfRange(end, buf.size)
                body(body)?.let { out += it }
            }
            return out
        }

        /** One frame's bytes after `Len`: `Slice 00 [Fsn] data`. */
        private fun body(b: ByteArray): Message? {
            when (b[0].toInt()) {
                0 -> return parse(b, 2)
                1 -> if (b.size >= 3) slices = java.io.ByteArrayOutputStream().also { it.write(b, 3, b.size - 3) }
                // A middle or last slice with no first slice before it is dropped.
                2, 3 -> if (b.size >= 3) {
                    val all = slices ?: return null
                    all.write(b, 3, b.size - 3)
                    if (b[0].toInt() == 3) { slices = null; return parse(all.toByteArray(), 0) }
                }
            }
            return null
        }

        /** `Len` and the bytes it took, or null while incomplete. */
        private fun readLen(b: ByteArray): Pair<Int, Int>? {
            var v = 0
            for (i in 1 until minOf(b.size, 4)) {
                val x = b[i].toInt() and 0xFF
                v = v or ((x and 0x7F) shl (7 * (i - 1)))
                if (x and 0x80 == 0) return v to i
            }
            return null
        }

        /** `Cmd (2 LE)  Seq  PayLen (2 LE)  Payload` from [from]. A `PayLen` past the bytes present keeps the bytes present. */
        private fun parse(m: ByteArray, from: Int): Message? {
            if (m.size - from < 5) return null
            val n = (m[from + 3].toInt() and 0xFF) or ((m[from + 4].toInt() and 0xFF) shl 8)
            val rest = m.size - from - 5
            return Message(
                (m[from].toInt() and 0xFF) or ((m[from + 1].toInt() and 0xFF) shl 8), m[from + 2].toInt() and 0xFF,
                m.copyOfRange(from + 5, from + 5 + minOf(n, rest))
            )
        }
    }
}
