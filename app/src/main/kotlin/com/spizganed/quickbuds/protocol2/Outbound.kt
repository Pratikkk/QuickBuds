package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol.EqCodec
import com.spizganed.quickbuds.protocol.KeyFunctionParser
import com.spizganed.quickbuds.protocol.OpoProtocol
import com.spizganed.quickbuds.protocol.Feature

/**
 * The packets the app sends. [typed] true builds the frame with [Cmd]; false builds the legacy packet, byte for byte
 * as before. Only a [Cmd] builder with a differential test (OutboundTest) is used here. Each call takes one sequence number.
 */
object Outbound {
    private fun pkt(f: Frame) = OpoProtocol.buildPacket(f.cmd, payload = f.payload)
    private fun parseMac(s: String): Mac = s.split(":").map { it.toInt(16) }

    // --- Connection ---
    fun register(typed: Boolean, ids: List<Int>) = if (typed) pkt(Cmd.register(ids)) else OpoProtocol.registerNotifications(ids)
    fun registerOne(typed: Boolean, id: Int) = if (typed) pkt(Cmd.registerOne(id)) else OpoProtocol.registerOne(id)
    fun spatialOnBuds(typed: Boolean) = if (typed) pkt(Cmd.spatialSource(0)) else OpoProtocol.spatialOnBuds()

    // --- ANC, gestures, features ---
    fun anc(typed: Boolean, bit: Int) = if (typed) pkt(Cmd.ancSet(bit)) else OpoProtocol.anc(bit)
    fun holdCycle(typed: Boolean, mask: Int, kind: Int) =
        if (typed) pkt(Cmd.holdCycle(kind, mask)) else OpoProtocol.setHoldAncModes(mask, kind)
    fun gestureTable(typed: Boolean, entries: List<KeyFunctionParser.Entry>) =
        if (typed) pkt(Cmd.gestureTable(entries.map { Binding(it.deviceType, it.button, it.action, it.function) }))
        else OpoProtocol.setKeyFunction(entries)
    fun feature(typed: Boolean, id: Int, on: Boolean) = if (typed) pkt(Cmd.feature(id, on)) else OpoProtocol.setFeature(id, on)

    // --- EQ ---
    /** A tag that is not two bytes stays on the legacy encoder. */
    fun customEq(typed: Boolean, action: Int, p: EqCodec.Preset): ByteArray {
        if (!typed || p.tag.size != 2) return OpoProtocol.customEq(action, p)
        val a = Cmd.EqAction.values().first { it.code == action }
        val e = EqPreset(p.id, p.name, p.selected, p.tag[0].toInt() to p.tag[1].toInt(), p.freqs.zip(p.gains))
        return pkt(Cmd.eqPreset(a, e))
    }
    fun bassLevel(typed: Boolean, level: Int) = if (typed) pkt(Cmd.bassSet(level)) else OpoProtocol.setBassWaveLevel(level)

    // --- Sound and device settings ---
    /** The legacy builder limits the level to 1..10; so does this. */
    fun alertVolume(typed: Boolean, level: Int) =
        if (typed) pkt(Cmd.alertVolume(level.coerceIn(1, 10))) else OpoProtocol.setAlertVolume(level)
    fun tapLevel(typed: Boolean, level: Int) = if (typed) pkt(Cmd.tapLevel(level)) else OpoProtocol.setTapLevel(level)
    fun headGesture(typed: Boolean, type: Int) = if (typed) pkt(Cmd.headGesture(type)) else OpoProtocol.setHeadMotionType(type)
    fun spatial(typed: Boolean, type: Int) = if (typed) pkt(Cmd.spatial(type)) else OpoProtocol.setSpatialType(type)
    fun codec(typed: Boolean, codec: Int, hiRes: Boolean) = if (typed) pkt(Cmd.codec(codec, hiRes)) else OpoProtocol.setCodec(codec, hiRes)
    fun gameSound(typed: Boolean, type: Int) = if (typed) pkt(Cmd.gameSound(type)) else OpoProtocol.setGameSoundType(type)
    fun gameEq(typed: Boolean, inGame: Boolean) = if (typed) pkt(Cmd.gameEq(inGame)) else OpoProtocol.gameEqState(inGame)
    fun find(typed: Boolean, on: Boolean) = if (typed) pkt(Cmd.find(on)) else OpoProtocol.findTone(on)
    fun fitTest(typed: Boolean, on: Boolean) = if (typed) pkt(Cmd.fitTest(on)) else OpoProtocol.fitTest(on)
    fun personalAnc(typed: Boolean, action: Int) = if (typed) pkt(Cmd.personalAnc(action)) else OpoProtocol.personalNoise(action)

    // --- Dual connection ---
    fun connectDevice(typed: Boolean, mac: String, on: Boolean) =
        if (typed) pkt(Cmd.deviceConnect(parseMac(mac), on)) else OpoProtocol.connectDevice(mac, on)
    fun preferredSet(typed: Boolean, mac: String?) =
        if (typed) pkt(Cmd.preferredSet(mac?.let { parseMac(it) })) else OpoProtocol.setPreferred(mac)
    fun preferredRead(typed: Boolean) = if (typed) pkt(Cmd.preferredRead()) else OpoProtocol.queryPreferred()
    /** After the dual switch: `0x0413` with the auto-switch flag, the opposite of the switch. */
    fun dualFollowup(typed: Boolean, dualOn: Boolean) =
        if (typed) pkt(Cmd.phoneState(Cmd.PhoneState(autoSwitch = !dualOn))) else OpoProtocol.dualFollowup(dualOn)

    // --- Golden Sound ---
    fun earScan(typed: Boolean, on: Boolean, id: Int) = if (typed) pkt(Cmd.earScan(on, id)) else OpoProtocol.earScan(on, id)
    fun hearingTest(typed: Boolean, on: Boolean) = if (typed) pkt(Cmd.hearingTest(on)) else OpoProtocol.hearingTest(on)
    fun tone(typed: Boolean, side: Int, freq: Int, value: Int) =
        if (typed) pkt(Cmd.tone(side, freq, value)) else OpoProtocol.hearingTone(side, freq, value)
    fun hearingFilter(typed: Boolean, id: Int, values: IntArray) =
        if (typed) pkt(Cmd.goldenFilter(id, values.toList())) else OpoProtocol.hearingFilter(id, values)
    fun scanFilter(typed: Boolean, id: Int, scan: ByteArray) =
        if (typed) pkt(Cmd.goldenScanFilter(id, scan)) else OpoProtocol.earScanFilter(id, scan)

    /** The frames that apply a record, in send order: description id, record, ear scan if any, switch on. */
    fun goldenApply(typed: Boolean, id: Int, name: String, values: IntArray, scan: ByteArray, desc: Int): List<ByteArray> =
        if (typed) Cmd.goldenApply(id, name, values.toList(), scan, desc).map { pkt(it) }
        else listOfNotNull(
            if (desc > 0) OpoProtocol.hearingRestore(desc) else null,
            OpoProtocol.hearingRecord(id, name, values),
            if (scan.isNotEmpty()) OpoProtocol.earScanData(id, scan) else null,
            OpoProtocol.setFeature(Feature.GOLDEN_SOUND, true)
        )
}
