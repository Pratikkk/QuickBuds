package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol.EqCodec
import com.spizganed.quickbuds.protocol.KeyFunctionParser
import com.spizganed.quickbuds.protocol.OpoProtocol
import org.junit.Assert.assertEquals
import org.junit.Test

/** Each sender family: the typed bytes equal the legacy bytes, and the legacy switch sends the legacy bytes. Sequence numbers are ignored. */
class OutboundTest {
    private fun msg(p: ByteArray) = Wire.Reassembler().push(p).single().let { it.cmd to it.payload.toList() }

    /** [legacy] is the old builder; [out] is the sender with the switch. */
    private fun same(label: String, legacy: ByteArray, out: (Boolean) -> ByteArray) {
        assertEquals("$label typed", msg(legacy), msg(out(true)))
        assertEquals("$label legacy", msg(legacy), msg(out(false)))
    }

    @Test fun connection() {
        same("register", OpoProtocol.registerNotifications(listOf(1, 2, 3, 4, 8, 11))) { Outbound.register(it, listOf(1, 2, 3, 4, 8, 11)) }
        same("registerOne", OpoProtocol.registerOne(4)) { Outbound.registerOne(it, 4) }
        same("spatialOnBuds", OpoProtocol.spatialOnBuds()) { Outbound.spatialOnBuds(it) }
    }

    @Test fun anc() {
        for (bit in listOf(0, 1, 2, 4, 5, 6, 11)) same("anc $bit", OpoProtocol.anc(bit)) { Outbound.anc(it, bit) }
        for ((kind, mask) in listOf(1 to 3, 1 to 0x0807, 3 to 1, 4 to 0x0100, 1 to 0x01020304, 1 to 0))
            same("hold $kind $mask", OpoProtocol.setHoldAncModes(mask, kind)) { Outbound.holdCycle(it, mask, kind) }
    }

    @Test fun featureSwitches() {
        // Game mode, dual device, power saving, Golden Sound and the rest of the 0x0403 writes.
        for (id in listOf(0x06, 0x0B, 0x11, 0x1B, 0x1D, 0x28)) for (on in listOf(true, false))
            same("feature $id $on", OpoProtocol.setFeature(id, on)) { Outbound.feature(it, id, on) }
    }

    @Test fun gestures() {
        val e = listOf(KeyFunctionParser.Entry(1, 1, 2, 0x11), KeyFunctionParser.Entry(4, 6, 6, 0x1C), KeyFunctionParser.Entry(2, 3, 1, 0x05))
        same("table", OpoProtocol.setKeyFunction(e)) { Outbound.gestureTable(it, e) }
        same("one entry", OpoProtocol.setKeyFunction(e.drop(1).take(1))) { Outbound.gestureTable(it, e.drop(1).take(1)) }
    }

    @Test fun eq() {
        val p = EqCodec.Preset(5, "Mine", listOf(62, 250, 1000, 4000), listOf(-6, 0, 3, 6), false, EqCodec.DEFAULT_TAG)
        for (a in listOf(EqCodec.ACTION_CREATE, EqCodec.ACTION_SAVE, EqCodec.ACTION_DELETE))
            same("eq $a", OpoProtocol.customEq(a, p)) { Outbound.customEq(it, a, p) }
        val n = EqCodec.newPreset("Ünï", listOf(100, 200, 300))
        same("new", OpoProtocol.customEq(EqCodec.ACTION_CREATE, n)) { Outbound.customEq(it, EqCodec.ACTION_CREATE, n) }
        for (l in -5..5) same("bass $l", OpoProtocol.setBassWaveLevel(l)) { Outbound.bassLevel(it, l) }
    }

    @Test fun settings() {
        for (l in listOf(-1, 0, 1, 6, 10, 11)) same("alert $l", OpoProtocol.setAlertVolume(l)) { Outbound.alertVolume(it, l) }
        for (l in 1..5) same("tap $l", OpoProtocol.setTapLevel(l)) { Outbound.tapLevel(it, l) }
        for (t in 0..1) same("head $t", OpoProtocol.setHeadMotionType(t)) { Outbound.headGesture(it, t) }
        for (t in 0..2) same("spatial $t", OpoProtocol.setSpatialType(t)) { Outbound.spatial(it, t) }
        for (c in 1..8) for (h in listOf(true, false)) same("codec $c $h", OpoProtocol.setCodec(c, h)) { Outbound.codec(it, c, h) }
        for (t in 0..3) same("game sound $t", OpoProtocol.setGameSoundType(t)) { Outbound.gameSound(it, t) }
        for (g in listOf(true, false)) {
            same("game eq $g", OpoProtocol.gameEqState(g)) { Outbound.gameEq(it, g) }
            same("find $g", OpoProtocol.findTone(g)) { Outbound.find(it, g) }
            same("fit $g", OpoProtocol.fitTest(g)) { Outbound.fitTest(it, g) }
        }
        for (a in 1..3) same("personal $a", OpoProtocol.personalNoise(a)) { Outbound.personalAnc(it, a) }
    }

    @Test fun dualConnection() {
        val mac = "0A:1B:2C:3D:4E:5F"
        for (on in listOf(true, false)) {
            same("connect $on", OpoProtocol.connectDevice(mac, on)) { Outbound.connectDevice(it, mac, on) }
            same("followup $on", OpoProtocol.dualFollowup(on)) { Outbound.dualFollowup(it, on) }
        }
        same("preferred", OpoProtocol.setPreferred(mac)) { Outbound.preferredSet(it, mac) }
        same("preferred auto", OpoProtocol.setPreferred(null)) { Outbound.preferredSet(it, null) }
        same("preferred read", OpoProtocol.queryPreferred()) { Outbound.preferredRead(it) }
    }

    @Test fun goldenSound() {
        val v = IntArray(12) { -it * 5 }
        val scan = ByteArray(168) { it.toByte() }
        for (on in listOf(true, false)) {
            same("scan $on", OpoProtocol.earScan(on, 0x01020304)) { Outbound.earScan(it, on, 0x01020304) }
            same("test $on", OpoProtocol.hearingTest(on)) { Outbound.hearingTest(it, on) }
        }
        same("tone", OpoProtocol.hearingTone(2, 3, -30)) { Outbound.tone(it, 2, 3, -30) }
        same("filter", OpoProtocol.hearingFilter(7, v)) { Outbound.hearingFilter(it, 7, v) }
        same("scan filter", OpoProtocol.earScanFilter(7, scan)) { Outbound.scanFilter(it, 7, scan) }
        for ((s, d) in listOf(scan to 11, ByteArray(0) to 11, scan to 0, ByteArray(0) to 0)) {
            val legacy = listOfNotNull(
                if (d > 0) OpoProtocol.hearingRestore(d) else null,
                OpoProtocol.hearingRecord(0x01020304, "2026/09/29 01:53", v),
                if (s.isNotEmpty()) OpoProtocol.earScanData(0x01020304, s) else null,
                OpoProtocol.setFeature(OpoProtocol.FEATURE_GOLDEN_SOUND, true)
            ).map { msg(it) }
            for (t in listOf(true, false))
                assertEquals("apply $t ${s.size} $d", legacy, Outbound.goldenApply(t, 0x01020304, "2026/09/29 01:53", v, s, d).map { msg(it) })
        }
    }
}
