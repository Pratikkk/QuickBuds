package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol.EqCodec
import com.spizganed.quickbuds.protocol.Feature
import com.spizganed.quickbuds.protocol.KeyFunctionParser
import com.spizganed.quickbuds.protocol.OpoProtocol
import com.spizganed.quickbuds.protocol.OppoPacketFramer

/**
 * The only door to the two protocols. [typed] true uses [NewProtocol]; false uses the legacy code in `protocol/LegacyProtocol.kt`,
 * byte for byte as before. The rest of the app never branches on the protocol: it asks this class.
 * Only a [NewProtocol] builder with a differential test (OutboundTest) is used here. Each call takes one sequence number.
 */
class ProtocolSelect(val typed: Boolean) {
    companion object {
        /** The choice the user and the per-device fallback make (see [ProtocolNext.enabled]). */
        fun of(c: android.content.Context) = ProtocolSelect(ProtocolNext.enabled(c))
    }

    // --- In ---
    /** The reader for the link's bytes: the legacy framer, or the new reassembler. [onMessage] sees each new-protocol message (for the log). */
    fun rx(onDiscard: (ByteArray) -> Unit, onMessage: (Wire.Message) -> Unit): (ByteArray, Int) -> List<ByteArray> =
        if (typed) NewProtocol.Rx(onDiscard, onMessage).let { r -> { b, n -> r.push(b, n) } }
        else OppoPacketFramer().also { it.onDiscard = onDiscard }.let { f -> { b, n -> f.append(b, n) } }

    /** The typed reading of a message; null on the legacy protocol, and for a message the new layer does not know. */
    fun decode(cmd: Int, seq: Int, payload: ByteArray): Msg? = if (typed) NewProtocol.decode(cmd, seq, payload) else null

    /** The reply to a request from the buds, as a packet. */
    fun answer(cmd: Int, seq: Int, payload: ByteArray, unixSeconds: Long): ByteArray? =
        if (typed) NewProtocol.answer(cmd, seq, payload, unixSeconds) else OpoProtocol.answerFor(cmd, seq, payload, unixSeconds)

    /** The frames that carry a packet. The legacy protocol sends the packet as it is. */
    fun frames(packet: ByteArray): List<ByteArray> = if (typed) NewProtocol.frames(packet) else listOf(packet)

    // --- Out ---
    fun register(ids: List<Int>) = if (typed) NewProtocol.register(ids) else OpoProtocol.registerNotifications(ids)
    fun registerOne(id: Int) = if (typed) NewProtocol.registerOne(id) else OpoProtocol.registerOne(id)
    fun spatialOnBuds() = if (typed) NewProtocol.spatialOnBuds() else OpoProtocol.spatialOnBuds()
    fun anc(bit: Int) = if (typed) NewProtocol.anc(bit) else OpoProtocol.anc(bit)
    fun holdCycle(mask: Int, kind: Int) = if (typed) NewProtocol.holdCycle(mask, kind) else OpoProtocol.setHoldAncModes(mask, kind)
    fun feature(id: Int, on: Boolean) = if (typed) NewProtocol.feature(id, on) else OpoProtocol.setFeature(id, on)
    fun bassLevel(level: Int) = if (typed) NewProtocol.bassLevel(level) else OpoProtocol.setBassWaveLevel(level)
    /** The legacy builder limits the level to 1..10; so does this. */
    fun alertVolume(level: Int) = if (typed) NewProtocol.alertVolume(level) else OpoProtocol.setAlertVolume(level)
    fun tapLevel(level: Int) = if (typed) NewProtocol.tapLevel(level) else OpoProtocol.setTapLevel(level)
    fun headGesture(type: Int) = if (typed) NewProtocol.headGesture(type) else OpoProtocol.setHeadMotionType(type)
    fun spatial(type: Int) = if (typed) NewProtocol.spatial(type) else OpoProtocol.setSpatialType(type)
    fun codec(codec: Int, hiRes: Boolean) = if (typed) NewProtocol.codec(codec, hiRes) else OpoProtocol.setCodec(codec, hiRes)
    fun gameSound(type: Int) = if (typed) NewProtocol.gameSound(type) else OpoProtocol.setGameSoundType(type)
    fun gameEq(inGame: Boolean) = if (typed) NewProtocol.gameEq(inGame) else OpoProtocol.gameEqState(inGame)
    fun find(on: Boolean) = if (typed) NewProtocol.find(on) else OpoProtocol.findTone(on)
    fun fitTest(on: Boolean) = if (typed) NewProtocol.fitTest(on) else OpoProtocol.fitTest(on)
    fun personalAnc(action: Int) = if (typed) NewProtocol.personalAnc(action) else OpoProtocol.personalNoise(action)
    fun connectDevice(mac: String, on: Boolean) = if (typed) NewProtocol.connectDevice(mac, on) else OpoProtocol.connectDevice(mac, on)
    fun preferredSet(mac: String?) = if (typed) NewProtocol.preferredSet(mac) else OpoProtocol.setPreferred(mac)
    fun preferredRead() = if (typed) NewProtocol.preferredRead() else OpoProtocol.queryPreferred()
    /** After the dual switch: `0x0413` with the auto-switch flag, the opposite of the switch. */
    fun dualFollowup(dualOn: Boolean) = if (typed) NewProtocol.dualFollowup(dualOn) else OpoProtocol.dualFollowup(dualOn)
    fun earScan(on: Boolean, id: Int) = if (typed) NewProtocol.earScan(on, id) else OpoProtocol.earScan(on, id)
    fun hearingTest(on: Boolean) = if (typed) NewProtocol.hearingTest(on) else OpoProtocol.hearingTest(on)
    fun tone(side: Int, freq: Int, value: Int) = if (typed) NewProtocol.tone(side, freq, value) else OpoProtocol.hearingTone(side, freq, value)
    fun hearingFilter(id: Int, values: IntArray) = if (typed) NewProtocol.hearingFilter(id, values) else OpoProtocol.hearingFilter(id, values)
    fun scanFilter(id: Int, scan: ByteArray) = if (typed) NewProtocol.scanFilter(id, scan) else OpoProtocol.earScanFilter(id, scan)
    fun gestureTable(entries: List<KeyFunctionParser.Entry>) =
        if (typed) NewProtocol.gestureTable(entries) else OpoProtocol.setKeyFunction(entries)

    /** A tag that is not two bytes stays on the legacy encoder. */
    fun customEq(action: Int, p: EqCodec.Preset): ByteArray =
        (if (typed) NewProtocol.customEq(action, p) else null) ?: OpoProtocol.customEq(action, p)

    /** The frames that apply a record, in send order: description id, record, ear scan if any, switch on. */
    fun goldenApply(id: Int, name: String, values: IntArray, scan: ByteArray, desc: Int): List<ByteArray> =
        if (typed) NewProtocol.goldenApply(id, name, values, scan, desc)
        else listOfNotNull(
            if (desc > 0) OpoProtocol.hearingRestore(desc) else null,
            OpoProtocol.hearingRecord(id, name, values),
            if (scan.isNotEmpty()) OpoProtocol.earScanData(id, scan) else null,
            OpoProtocol.setFeature(Feature.GOLDEN_SOUND, true)
        )
}
