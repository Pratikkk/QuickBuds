package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol.EqCodec
import com.spizganed.quickbuds.protocol.Feature
import com.spizganed.quickbuds.protocol.KeyFunctionParser
import com.spizganed.quickbuds.protocol.OpoProtocol

/**
 * The new protocol: the one entry point of this package. The app reaches it through [ProtocolSelect].
 * Out: every frame the app sends is built here from a [Cmd] builder. In: [decode], [answer] and [rx] read what the buds send.
 * The frame envelope and the sequence counter come from the legacy builder, so both protocols share one counter.
 * Plain Kotlin apart from the envelope, so a JVM test runs it.
 */
object NewProtocol {
    private fun pkt(f: Frame) = OpoProtocol.buildPacket(f.cmd, payload = f.payload)
    private fun parseMac(s: String): Mac = s.split(":").map { it.toInt(16) }

    // --- In ---
    /** Turns the bytes of the link into messages. [onMessage] sees each one. Each message comes out as one packet with the fixed 9-byte header. */
    class Rx(onDiscard: (ByteArray) -> Unit, private val onMessage: (Wire.Message) -> Unit) {
        private val wire = Wire.Reassembler(onDiscard)
        fun push(buffer: ByteArray, length: Int): List<ByteArray> = wire.push(buffer, length).map {
            onMessage(it)
            com.spizganed.quickbuds.protocol.OppoPacketFramer.normalise(OpoProtocol.buildPacket(it.cmd, it.seq, it.payload))
        }
    }

    /** The typed reading of a message, or null when the layer does not know it. */
    fun decode(cmd: Int, seq: Int, payload: ByteArray): Msg? = Decode.decode(Wire.Message(cmd, seq, payload))

    /** The reply to a request from the buds (the time request), as a packet; null when no reply is due. */
    fun answer(cmd: Int, seq: Int, payload: ByteArray, unixSeconds: Long): ByteArray? =
        Decode.answer(Wire.Message(cmd, seq, payload), unixSeconds)?.let { OpoProtocol.buildPacket(it.cmd, it.seq, it.payload) }

    /** The frames that carry a packet: a message too big for one frame goes out in slices (docs/PROTOCOL.md §2). */
    fun frames(packet: ByteArray): List<ByteArray> =
        Wire.Reassembler().push(packet).flatMap { Wire.encode(it) }.ifEmpty { listOf(packet) }

    // --- Out ---
    fun register(ids: List<Int>) = pkt(Cmd.register(ids))
    fun registerOne(id: Int) = pkt(Cmd.registerOne(id))
    fun spatialOnBuds() = pkt(Cmd.spatialSource(0))
    fun anc(bit: Int) = pkt(Cmd.ancSet(bit))
    fun holdCycle(mask: Int, kind: Int) = pkt(Cmd.holdCycle(kind, mask))
    fun feature(id: Int, on: Boolean) = pkt(Cmd.feature(id, on))
    fun bassLevel(level: Int) = pkt(Cmd.bassSet(level))
    /** The legacy builder limits the level to 1..10; so does this. */
    fun alertVolume(level: Int) = pkt(Cmd.alertVolume(level.coerceIn(1, 10)))
    fun tapLevel(level: Int) = pkt(Cmd.tapLevel(level))
    fun headGesture(type: Int) = pkt(Cmd.headGesture(type))
    fun spatial(type: Int) = pkt(Cmd.spatial(type))
    fun codec(codec: Int, hiRes: Boolean) = pkt(Cmd.codec(codec, hiRes))
    fun gameSound(type: Int) = pkt(Cmd.gameSound(type))
    fun gameEq(inGame: Boolean) = pkt(Cmd.gameEq(inGame))
    fun find(on: Boolean) = pkt(Cmd.find(on))
    fun fitTest(on: Boolean) = pkt(Cmd.fitTest(on))
    fun personalAnc(action: Int) = pkt(Cmd.personalAnc(action))
    fun connectDevice(mac: String, on: Boolean) = pkt(Cmd.deviceConnect(parseMac(mac), on))
    fun preferredSet(mac: String?) = pkt(Cmd.preferredSet(mac?.let { parseMac(it) }))
    fun preferredRead() = pkt(Cmd.preferredRead())
    /** After the dual switch: `0x0413` with the auto-switch flag, the opposite of the switch. */
    fun dualFollowup(dualOn: Boolean) = pkt(Cmd.phoneState(Cmd.PhoneState(autoSwitch = !dualOn)))
    fun earScan(on: Boolean, id: Int) = pkt(Cmd.earScan(on, id))
    fun hearingTest(on: Boolean) = pkt(Cmd.hearingTest(on))
    fun tone(side: Int, freq: Int, value: Int) = pkt(Cmd.tone(side, freq, value))
    fun hearingFilter(id: Int, values: IntArray) = pkt(Cmd.goldenFilter(id, values.toList()))
    fun scanFilter(id: Int, scan: ByteArray) = pkt(Cmd.goldenScanFilter(id, scan))
    fun gestureTable(entries: List<KeyFunctionParser.Entry>) =
        pkt(Cmd.gestureTable(entries.map { Binding(it.deviceType, it.button, it.action, it.function) }))

    /** Null when the tag is not two bytes: only the legacy encoder writes that. */
    fun customEq(action: Int, p: EqCodec.Preset): ByteArray? {
        if (p.tag.size != 2) return null
        val a = Cmd.EqAction.values().first { it.code == action }
        val e = EqPreset(p.id, p.name, p.selected, p.tag[0].toInt() to p.tag[1].toInt(), p.freqs.zip(p.gains))
        return pkt(Cmd.eqPreset(a, e))
    }

    /** The frames that apply a record, in send order: description id, record, ear scan if any, switch on. */
    fun goldenApply(id: Int, name: String, values: IntArray, scan: ByteArray, desc: Int): List<ByteArray> =
        Cmd.goldenApply(id, name, values.toList(), scan, desc).map { pkt(it) }
}
