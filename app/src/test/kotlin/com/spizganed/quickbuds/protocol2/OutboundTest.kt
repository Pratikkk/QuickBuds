package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol.EqCodec
import com.spizganed.quickbuds.protocol.Feature
import com.spizganed.quickbuds.protocol.KeyFunctionParser
import com.spizganed.quickbuds.protocol.OpoProtocol
import com.spizganed.quickbuds.protocol.RowRules
import com.spizganed.quickbuds.protocol.Wire
import com.spizganed.quickbuds.protocol.Commands
import com.spizganed.quickbuds.protocol.HearingCommands
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Each sender family: the typed layer's packet equals the shipping layer's packet.
 * Every pair is a whole packet, the sequence byte included. A builder with a fixed sequence must
 * keep it. The command and payload of the typed facade [NewProtocol] are compared too.
 */
class OutboundTest {
    private fun msg(p: ByteArray) = Wire.Reassembler().push(p).single().let { it.cmd to it.payload.toList() }

    private fun hex(b: ByteArray) = b.joinToString(" ") { "%02X".format(it) }

    /** The sequence byte of a whole packet. The header length changes with the payload size. */
    private fun seqOf(packet: ByteArray) = Wire.Reassembler().push(packet).single().seq

    /**
     * Compares the whole packet of the typed [frame] with the shipping [legacy] packet.
     * A frame without a fixed sequence takes the legacy sequence: the shared counter gives the
     * next number to the second call.
     */
    private fun same(label: String, legacy: ByteArray, frame: Frame) {
        val legacySeq = seqOf(legacy)
        // The counter stops at 0xBF: a higher legacy sequence is fixed, and the frame must carry it.
        if (legacySeq > 0xBF) assertEquals("$label fixed seq", legacySeq, frame.seq)
        if (frame.seq != null) assertEquals("$label seq", legacySeq, frame.seq)
        val seq = frame.seq ?: legacySeq
        assertEquals(label, hex(legacy), hex(OpoProtocol.buildPacket(frame.cmd, seq, frame.payload)))
    }

    /** The whole packet, then the command and payload of the typed facade [out]. */
    private fun same(label: String, legacy: ByteArray, frame: Frame, out: () -> ByteArray) {
        same(label, legacy, frame)
        assertEquals(label, msg(legacy), msg(out()))
    }

    private fun bindings(entries: List<KeyFunctionParser.Entry>) =
        entries.map { Binding(it.deviceType, it.button, it.action, it.function) }

    private fun mac(s: String): Mac = s.split(":").map { it.toInt(16) }

    @Test fun connection() {
        val ids = listOf(1, 2, 3, 4, 8, 11)
        same("handshake", Commands.handshake(), Cmd.handshake())
        same("product id", Commands.queryProductId(), Cmd.productId())
        same("broadcast", Commands.queryBroadcastCodes(), Cmd.broadcastCodes())
        same("firmware", Commands.queryFirmware(), Cmd.firmware())
        same("register", Commands.registerNotifications(ids), Cmd.register(ids)) { NewProtocol.register(ids) }
        same("registerOne", Commands.registerOne(4), Cmd.registerOne(4)) { NewProtocol.registerOne(4) }
        same("spatialOnBuds", Commands.spatialOnBuds(), Cmd.spatialSource(0)) { NewProtocol.spatialOnBuds() }
    }

    @Test fun fixedSequenceQueries() {
        // Legacy queryBattery and queryWearingStatus keep 0xF0 and 0xF2.
        same("battery", Commands.queryBattery(), Cmd.battery())
        same("wearing", Commands.queryWearingStatus(), Cmd.wearing())
    }

    @Test fun reads() {
        same("devices", Commands.queryDevices(), Cmd.devicesRead())
        same("eq", Commands.queryEq(), Cmd.eqCurrent())
        same("eq all", Commands.queryEqAll(), Cmd.eqCustom())
        same("bass", Commands.queryBassWaveLevel(), Cmd.bassLevel())
        same("alert", Commands.queryAlertVolume(), Cmd.alertVolumeRead())
        same("tap", Commands.queryTapLevel(), Cmd.tapLevelRead())
        same("head", Commands.queryHeadMotionType(), Cmd.headGestureRead())
        same("spatial", Commands.querySpatialType(), Cmd.spatialRead())
        same("codec", Commands.queryCodec(), Cmd.codecRead())
        same("codec list", Commands.queryCodecList(), Cmd.codecListRead())
        same("game sound", Commands.queryGameSound(), Cmd.gameSoundRead())
        same("personal anc", Commands.queryPersonalNoise(), Cmd.personalAncStored())
        same("gestures", Commands.queryKeyFunction(), Cmd.gestures())
        same("golden record", HearingCommands.queryGoldenActive(), Cmd.goldenRecordRead())
        same("golden scan", HearingCommands.queryGoldenActiveScan(), Cmd.goldenScanRead())
        same("preferred", Commands.queryPreferred(), Cmd.preferredRead()) { NewProtocol.preferredRead() }
        // 0x010C carries four questions, selected by the payload.
        same("anc mode", Commands.queryAncMode(), Cmd.ancRead(1, 1))
        same("smart level", Commands.querySmartLevel(), Cmd.ancRead(4, 1))
        for (t in listOf(OpoProtocol.HOLD_TYPE_SHARED, OpoProtocol.HOLD_TYPE_LEFT, OpoProtocol.HOLD_TYPE_RIGHT))
            same("hold modes $t", Commands.queryNoiseSwitchModes(t), Cmd.ancRead(2, t))
    }

    @Test fun status() {
        // One default id list, in RowRules only. The count byte comes first.
        same("default list", Commands.queryStatus(), Cmd.features(RowRules.DEFAULT_IDS.toList()))
        same("wind list", Commands.queryStatus(null, true), Cmd.features((RowRules.DEFAULT_IDS + Feature.WIND_NOISE).toList()))
        same("model list", Commands.queryStatus("05 04 0B"), Cmd.features(listOf(0x05, 0x04, 0x0B)))
    }

    @Test fun anc() {
        for (bit in listOf(0, 1, 2, 4, 5, 6, 11)) same("anc $bit", Commands.anc(bit), Cmd.ancSet(bit)) { NewProtocol.anc(bit) }
        for ((kind, mask) in listOf(1 to 3, 1 to 0x0807, 3 to 1, 4 to 0x0100, 1 to 0x01020304, 1 to 0))
            same("hold $kind $mask", Commands.holdAncModes(mask, kind), Cmd.holdCycle(kind, mask)) { NewProtocol.holdCycle(mask, kind) }
    }

    @Test fun featureSwitches() {
        // Game mode, dual device, power saving, Golden Sound and the rest of the 0x0403 writes.
        for (id in listOf(0x06, 0x0B, 0x11, 0x1B, 0x1D, 0x28)) for (on in listOf(true, false))
            same("feature $id $on", Commands.feature(id, on), Cmd.feature(id, on)) { NewProtocol.feature(id, on) }
    }

    @Test fun gestures() {
        val e = listOf(KeyFunctionParser.Entry(1, 1, 2, 0x11), KeyFunctionParser.Entry(4, 6, 6, 0x1C), KeyFunctionParser.Entry(2, 3, 1, 0x05))
        same("table", Commands.setKeyFunction(e), Cmd.gestureTable(bindings(e))) { NewProtocol.gestureTable(e) }
        same("one entry", Commands.setKeyFunction(e.drop(1).take(1)), Cmd.gestureTable(bindings(e.drop(1).take(1)))) { NewProtocol.gestureTable(e.drop(1).take(1)) }
    }

    @Test fun eq() {
        val p = EqCodec.Preset(5, "Mine", listOf(62, 250, 1000, 4000), listOf(-6, 0, 3, 6), false, EqCodec.DEFAULT_TAG)
        for (a in listOf(EqCodec.ACTION_CREATE, EqCodec.ACTION_SAVE, EqCodec.ACTION_DELETE)) {
            same("eq $a", Commands.customEq(a, p), Cmd.eqPreset(Cmd.EqAction.values().first { it.code == a }, p)) { NewProtocol.customEq(a, p) }
        }
        val n = EqCodec.newPreset("Ünï", listOf(100, 200, 300))
        same("new", Commands.customEq(EqCodec.ACTION_CREATE, n), Cmd.eqPreset(Cmd.EqAction.Create, n)) { NewProtocol.customEq(EqCodec.ACTION_CREATE, n) }
        for (l in -5..5) same("bass $l", Commands.setBassWaveLevel(l), Cmd.bassSet(l)) { NewProtocol.bassLevel(l) }
    }

    @Test fun alertVolumeClampsTo1To10() {
        for ((l, b) in listOf(-1 to 1, 0 to 1, 1 to 1, 10 to 10, 11 to 10, 255 to 10)) {
            assertEquals("alert $l", listOf(b.toByte()), msg(Commands.alertVolume(l)).second)
        }
    }

    @Test fun settings() {
        for (l in listOf(-1, 0, 1, 6, 10, 11)) same("alert $l", Commands.alertVolume(l), Cmd.alertVolume(l)) { NewProtocol.alertVolume(l) }
        for (l in 1..5) same("tap $l", Commands.setTapLevel(l), Cmd.tapLevel(l)) { NewProtocol.tapLevel(l) }
        for (t in 0..1) same("head $t", Commands.headMotionType(t), Cmd.headGesture(t)) { NewProtocol.headGesture(t) }
        for (t in 0..2) same("spatial $t", Commands.spatialType(t), Cmd.spatial(t)) { NewProtocol.spatial(t) }
        for (c in 1..8) for (h in listOf(true, false)) same("codec $c $h", Commands.codec(c, h), Cmd.codec(c, h)) { NewProtocol.codec(c, h) }
        for (t in 0..3) same("game sound $t", Commands.gameSoundType(t), Cmd.gameSound(t)) { NewProtocol.gameSound(t) }
        for (g in listOf(true, false)) {
            same("game eq $g", Commands.gameEqState(g), Cmd.gameEq(g)) { NewProtocol.gameEq(g) }
            same("find $g", Commands.findTone(g), Cmd.find(g)) { NewProtocol.find(g) }
            same("fit $g", Commands.fitTest(g), Cmd.fitTest(g)) { NewProtocol.fitTest(g) }
        }
        for (a in 1..3) same("personal $a", Commands.personalNoise(a), Cmd.personalAnc(a)) { NewProtocol.personalAnc(a) }
    }

    @Test fun dualConnection() {
        val s = "0A:1B:2C:3D:4E:5F"
        for (on in listOf(true, false)) {
            same("connect $on", Commands.connectDevice(s, on), Cmd.deviceConnect(mac(s), on)) { NewProtocol.connectDevice(s, on) }
            same("followup $on", Commands.dualFollowup(on), Cmd.dualFollowup(on)) { NewProtocol.dualFollowup(on) }
        }
        same("preferred", Commands.setPreferred(s), Cmd.preferredSet(mac(s))) { NewProtocol.preferredSet(s) }
        same("preferred auto", Commands.setPreferred(null), Cmd.preferredSet(null)) { NewProtocol.preferredSet(null) }
    }

    @Test fun goldenSound() {
        val v = IntArray(12) { -it * 5 }
        val scan = ByteArray(168) { it.toByte() }
        for (on in listOf(true, false)) {
            same("scan $on", HearingCommands.earScan(on, 0x01020304), Cmd.earScan(on, 0x01020304)) { NewProtocol.earScan(on, 0x01020304) }
            same("test $on", HearingCommands.hearingTest(on), Cmd.hearingTest(on)) { NewProtocol.hearingTest(on) }
        }
        same("tone", HearingCommands.hearingTone(2, 3, -30), Cmd.tone(2, 3, -30)) { NewProtocol.tone(2, 3, -30) }
        same("tone stop", HearingCommands.hearingToneStop(), Cmd.toneStop())
        same("filter", HearingCommands.hearingFilter(7, v), Cmd.goldenFilter(7, v.toList())) { NewProtocol.hearingFilter(7, v) }
        same("scan filter", HearingCommands.earScanFilter(7, scan), Cmd.goldenScanFilter(7, scan)) { NewProtocol.scanFilter(7, scan) }
        for ((s, d) in listOf(scan to 11, ByteArray(0) to 11, scan to 0, ByteArray(0) to 0)) {
            val legacy = listOfNotNull(
                if (d > 0) HearingCommands.hearingRestore(d) else null,
                HearingCommands.hearingRecord(0x01020304, "2026/09/29 01:53", v),
                if (s.isNotEmpty()) HearingCommands.earScanData(0x01020304, s) else null,
                Commands.feature(Feature.GOLDEN_SOUND, true)
            )
            val typed = Cmd.goldenApply(0x01020304, "2026/09/29 01:53", v.toList(), s, d)
            assertEquals("apply ${s.size} $d", legacy.size, typed.size)
            for ((i, packet) in legacy.withIndex()) same("apply ${s.size} $d $i", packet, typed[i])
            assertEquals("apply facade ${s.size} $d", legacy.map { msg(it) },
                NewProtocol.goldenApply(0x01020304, "2026/09/29 01:53", v, s, d).map { msg(it) })
        }
    }
}
