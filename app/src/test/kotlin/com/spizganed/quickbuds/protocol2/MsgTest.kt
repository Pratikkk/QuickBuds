package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol.AncEventParser
import com.spizganed.quickbuds.protocol.BatteryParser
import com.spizganed.quickbuds.protocol.Capabilities
import com.spizganed.quickbuds.protocol.OpoProtocol
import com.spizganed.quickbuds.protocol2.Wire.Message
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MsgTest {
    private fun b(vararg v: Int) = ByteArray(v.size) { v[it].toByte() }
    private fun m(cmd: Int, vararg v: Int) = Message(cmd, 1, b(*v))
    private fun push(vararg v: Int) = Message(0x0204, 0xFF, b(*v))
    private fun dec(msg: Message) = Decode.decode(msg)

    @Test fun bitmapMatchesTheShippingTable() {
        val maps = mutableListOf(
            b(0, 0xFF, 0x77, 0x5A, 0xEA, 0x67, 0x0E, 0x20, 0x07), b(0, 0x9B, 0x2C, 0x50, 0x80),
        )
        for (bit in 0 until 72) maps += ByteArray(11).also { it[1 + bit / 8] = (1 shl (bit % 8)).toByte() }
        // The shipping table lacks the commands added for bits 35, 64, 67 and 68.
        val added = setOf(0x0419, 0x0136, 0x0433, 0x0432, 0x0135, 0x0435, 0x0117)
        for (map in maps) {
            val new = Bitmap.parse(map)!!
            val old = Capabilities.parse(map)!!
            for (cmd in 0..0x0FFF) if (cmd !in added) assertEquals("cmd $cmd", cmd in old, new.allows(cmd))
        }
        assertTrue(Bitmap.UNKNOWN.allows(0x0422) && !Bitmap.UNKNOWN.known)
        assertTrue(Bitmap.parse(b(0, 0xFF, 0x77, 0x5A, 0xEA, 0x67, 0x0E, 0x20, 0x07))!!.allows(0x012F))
        assertNull(Bitmap.parse(b(1, 0xFF)))
        assertNull(Bitmap.parse(b(0)))
    }

    @Test fun repliesFromTheBuds4Capture() {
        assertEquals(Msg.ProductId(0x065414), dec(m(0x8103, 0, 0x14, 0x54, 0x06)))
        assertNull(dec(m(0x8103, 1, 0, 0, 0)))
        assertEquals(Msg.Offered(listOf(1, 2, 3, 4, 8, 0x0B, 0xF1, 0xF2, 0xF3)), dec(m(0x8200, 0, 9, 1, 2, 3, 4, 8, 0x0B, 0xF1, 0xF2, 0xF3)))
        assertEquals(Msg.Offered(listOf(2, 1)), dec(m(0x8200, 0, 2, 2, 1)))
        val text = "1,2,138,2,2,138,3,1,01,3,2,105".toByteArray()
        val fw = Decode.decode(Message(0x8105, 1, byteArrayOf(0, 4) + text)) as Msg.Firmware
        assertEquals("138.138.105", fw.parts.filter { it.second == 2 }.joinToString(".") { it.third })
        assertEquals(Msg.Registered, dec(m(0x8205, 1, 2, 1, 0, 2, 0)))
    }

    @Test fun batchMatchesTheShippingSplit() {
        val replies = listOf(0x0106 to b(0, 1, 1, 0x5A), 0x010C to b(0, 1, 1, 8, 0), 0x010F to b(0, 2))
        var pl = b(0, replies.size)
        for ((c, p) in replies) pl += b(c and 0xFF, c shr 8, p.size, 0) + p
        val parts = Decode.unbatch(Message(0x812F, 7, pl))
        assertEquals(3, parts.size)
        val old = OpoProtocol.batchReplies(pl)
        old.zip(parts).forEach { (o, n) -> assertEquals(o.first or 0x8000, n.cmd); assertTrue(o.second.contentEquals(n.payload)); assertEquals(7, n.seq) }
        assertEquals(old.size, parts.size)
        assertEquals(2, Decode.unbatch(Message(0x812F, 7, pl.copyOf(pl.size - 1))).size)
    }

    @Test fun pushesMatchTheShippingDecoders() {
        // Battery, from PROTOCOL.md section 7: level 0 left out, charging bit.
        val p = intArrayOf(1, 3, 1, 0x64, 2, 0xE4, 3, 0)
        val new = dec(push(*p)) as Msg.Battery
        val old = BatteryParser.parseActive(OpoProtocol.buildPacket(0x0204, 0xFF, b(*p)))!!
        assertEquals(listOf(Cell(1, 100, false), Cell(2, 100, true)), new.cells)
        assertEquals(old.left!!.level, 100); assertEquals(old.right!!.isCharging, true); assertNull(old.case)
        assertEquals(Msg.AncMsg(Anc.Mode(8)), dec(push(3, 1, 1, 0x08, 0x00)))
        assertEquals(Msg.AncMsg(Anc.Smart(0x20)), dec(push(3, 4, 1, 0x20, 0)))
        assertEquals(8, AncEventParser.rawValue(b(3, 1, 1, 8, 0)))
        // The hold cycle push: the shipping decoder drops it, this one names it.
        assertEquals(Msg.AncMsg(Anc.HoldCycle(1, 7)), dec(push(3, 2, 1, 7)))
        assertEquals(Msg.FitTest(1, 1), dec(push(4, 1, 1, 2, 1)))
        assertEquals(Msg.FitTest(null, 6), dec(push(4, 2, 6)))
        assertEquals(Msg.GameMode(1, null), dec(push(5, 1)))
        assertEquals(Msg.GameMode(0, 1), dec(push(5, 0, 0, 1)))
        assertEquals(true, com.spizganed.quickbuds.protocol.GameModeParser.parseActive(b(5, 0, 0, 1), "062410"))
        assertEquals(Msg.PersonalAnc(2), dec(push(0x0B, 2)))
        assertEquals(Msg.HeadGesture(1), dec(push(0xF5, 1)))
        assertEquals(Msg.Hearing(2, 5), dec(push(0x08, 2, 5)))
        assertEquals(Msg.PublicMsg(Public.TapLevel(4)), dec(push(0x0F, 0x10, 0, 4)))
    }

    @Test fun devicesAndEarScan() {
        val p = ArrayList<Int>().apply {
            add(6); add(2)
            for ((mac, st, fl, name) in listOf(
                listOf(listOf(6, 5, 4, 3, 2, 1), 2, 1, "Phone"), listOf(listOf(12, 11, 10, 9, 8, 7), 0, 0, "PC"),
            )) {
                @Suppress("UNCHECKED_CAST") addAll(mac as List<Int>)
                val n = name as String
                add(3 + n.length); add(st as Int); add(fl as Int); add(n.length); addAll(n.map { it.code })
            }
        }
        val d = (dec(push(*p.toIntArray())) as Msg.Devices).list
        assertEquals(listOf(1, 2, 3, 4, 5, 6), d[0].mac)
        assertTrue(d[0].connected && d[0].thisPhone && !d[1].connected)
        assertNull(dec(push(6, 1, 1, 2, 3)))
        val scan = IntArray(168) { 9 }
        val e = dec(push(0x0E, 3, 168, 0, *scan, 1, 2, 3, 4)) as Msg.EarScan
        assertEquals(0x01020304, e.id); assertEquals(168, e.data.size)
        assertNull(dec(push(0x0E, 3, 0, 0)))
    }

    @Test fun pushesTheShippingDecoderIgnores() {
        assertEquals(Msg.Gesture(2, 1, 4, 8, 3, emptyList()), dec(push(0xF1, 2, 1, 4, 8, 3)))
        assertNull(dec(push(0xF1, 1, 1)))
        assertEquals(
            Msg.LinkedMsg(listOf(Linked(listOf(1, 2, 3, 4, 5, 6), 3, "PC"))),
            dec(push(0xF2, 1, 6, 5, 4, 3, 2, 1, 3, 0, 2, 'P'.code, 'C'.code)),
        )
        assertEquals(Msg.Diagnostic("{\"x\":1}"), Decode.decode(Message(0x0204, 0xFF, byteArrayOf(0xF4.toByte()) + "{\"x\":1}".toByteArray())))
        assertEquals(Msg.ZenState(1), dec(push(0x0A, 1)))
        assertEquals(Msg.HeadTry(2), dec(push(0xF6, 2)))
        assertEquals(Msg.VoiceAssist(2, true), dec(push(0xF7, 0x84)))
        assertEquals(Msg.VoiceAssist(null, false), dec(push(0xF7, 0)))
        assertEquals(Msg.LinkSwitch(1, 0, 1), dec(push(0xF8, 5)))
        assertEquals(Msg.Unused(0xF3, listOf(9)), dec(push(0xF3, 9)))
        assertEquals(Msg.Unused(0x10, listOf(1, 2)), dec(push(0x10, 1, 2)))
    }

    @Test fun publicEventsAndTriangle() {
        assertEquals(Msg.PublicMsg(Public.AlertVolume(6)), dec(push(0x0F, 1, 0, 6)))
        assertEquals(Msg.PublicMsg(Public.Handheld(true, null)), dec(push(0x0F, 4, 0, 0)))
        assertEquals(Msg.PublicMsg(Public.Handheld(false, listOf(1, 2, 3, 4, 5, 6))), dec(push(0x0F, 4, 0, 1, 6, 5, 4, 3, 2, 1)))
        assertNull(dec(push(0x0F, 4, 0, 1, 6, 5)))
        assertNull(dec(push(0x0F, 1, 0)))
        assertEquals(
            Msg.TriangleMsg(Triangle(2, 1, 7, 3, "PC")), dec(push(0x0D, 0x1F, 0, 2, 1, 7, 3, 2, 'P'.code, 'C'.code)),
        )
        assertEquals(Msg.TriangleMsg(Triangle(wear = 5)), dec(push(0x0D, 0x04, 0, 5)))
        assertEquals(Msg.TriangleMsg(Triangle(capability = 2)), dec(push(0x0D, 0x03, 0, 2)))
        assertFalse(Bitmap.UNKNOWN.known)
    }

    private fun payloadOf(packet: ByteArray) = packet.copyOfRange(9, packet.size)
    /** The last [n] bytes: a long frame's length field takes two bytes, so the payload does not start at 9. */
    private fun tail(packet: ByteArray, n: Int) = packet.copyOfRange(packet.size - n, packet.size)

    @Test fun batteryAndWearReplies() {
        assertEquals(
            listOf(Cell(1, 100, false), Cell(2, 100, false), Cell(3, 80, false)),
            (dec(m(0x8106, 0, 3, 1, 0x64, 2, 0x64, 3, 0x50)) as Msg.Battery).cells,
        )
        assertEquals(listOf(Cell(1, 90, false)), (dec(m(0x8106, 0, 1, 1, 0x5A)) as Msg.Battery).cells)
        assertNull(dec(m(0x8106, 1, 1, 1, 0x5A)))
        assertNull(dec(push(1, 0)))
        assertEquals(Msg.Wear(listOf(1 to 7, 2 to 7, 3 to 4)), dec(m(0x8109, 3, 1, 7, 2, 7, 3, 4)))
        assertEquals(Msg.Wear(listOf(1 to 3, 2 to 3, 3 to 4)), dec(m(0x8109, 0, 3, 1, 3, 2, 3, 3, 4)))
        assertNull(dec(m(0x8109, 3, 9, 7, 2, 7, 3, 4)))
        // Same facts through the shipping parser.
        val old = com.spizganed.quickbuds.protocol.WearingStatusParser.parseQueryResponse(b(3, 1, 7, 2, 7, 3, 4))!!
        assertEquals(7 and 7, old.leftStatus and 7)
    }

    @Test fun ancRepliesAndPushes() {
        assertEquals(Msg.AncMsg(Anc.Mode(8)), dec(m(0x810C, 0, 1, 1, 0x08, 0x00)))
        assertEquals(Msg.AncMsg(Anc.Mode(0x0800)), dec(m(0x810C, 0, 1, 1, 0x00, 0x08)))
        assertEquals(Msg.AncMsg(Anc.Smart(0x40)), dec(m(0x810C, 0, 4, 1, 0x40)))
        assertEquals(Msg.AncMsg(Anc.HoldCycle(1, 0x0807)), dec(m(0x810C, 0, 2, 1, 0x07, 0x08)))
        assertEquals(Msg.AncMsg(Anc.Level(3)), dec(m(0x810C, 0, 1, 2, 3)))
        assertNull(dec(m(0x810C, 0xE, 1, 1, 8, 0)))
        assertEquals(Msg.Ack(0x0404, 14, emptyList()), dec(m(0x8404, 0x0E)))
        assertEquals(Msg.Ack(0x0404, 0, emptyList()), dec(m(0x8404, 0)))
        assertEquals(Msg.Ack(0x0427, 0, listOf(6)), dec(m(0x8427, 0, 6)))
    }

    @Test fun gestureTableFromTheBuds4Capture() {
        val rows = listOf(
            Triple(1, 1, listOf(1 to 0, 2 to 0x11, 3 to 0, 4 to 8, 6 to 0, 5 to 7)), Triple(1, 6, listOf(2 to 0, 3 to 0, 6 to 0)),
            Triple(2, 1, listOf(1 to 0, 2 to 0, 3 to 0, 4 to 8, 6 to 0, 5 to 7)), Triple(2, 6, listOf(2 to 0, 3 to 0, 6 to 0)),
        ).flatMap { (d, bt, acts) -> acts.map { (a, fn) -> Binding(d, bt, a, fn) } }
        assertEquals(18, rows.size)
        val payload = byteArrayOf(0, 18) + rows.flatMap { listOf(it.dev, it.button, it.act, it.func) }.map { it.toByte() }
        val g = (Decode.decode(Message(0x8108, 1, payload)) as Msg.Gestures).table
        assertEquals(rows, g)
        val old = com.spizganed.quickbuds.protocol.KeyFunctionParser.parse(payload)
        assertEquals(old.entries.map { listOf(it.deviceType, it.button, it.action, it.function) }, g.map { listOf(it.dev, it.button, it.act, it.func) })
        assertNull(dec(m(0x8108, 0, 0)))
        assertEquals(1, (dec(m(0x8108, 0, 1, 1, 1, 1, 0, 9, 9, 9, 9)) as Msg.Gestures).table.size)
    }

    @Test fun featureAndEqReplies() {
        val f = dec(m(0x810D, 0, 8, 5, 0, 4, 0, 0x0B, 1, 0x11, 1, 0x18, 1, 6, 0, 0x1B, 0, 0x1D, 1)) as Msg.Features
        assertEquals(8, f.list.size)
        assertEquals(4 to 0, f.list[1])
        assertEquals(Msg.EqCurrent(2), dec(m(0x810F, 0, 2)))
        assertEquals(Msg.EqCurrent(1), Decode.decode(Message(0x0504, 0xFF, b(1))))
        assertEquals(Msg.BassLevel(-2), dec(m(0x8124, 0, 0xFB, 5, 0xFE)))
        val body = byteArrayOf(0, 1, 1, 0xFA.toByte(), 6, 4, 4) + "Rock".toByteArray() + b(2, 62, 0, 3, 0xFA, 0, 0xFF)
        val list = (Decode.decode(Message(0x8122, 1, body)) as Msg.EqCustom).list
        assertEquals(EqPreset(4, "Rock", true, -6 to 6, listOf(62 to 3, 250 to -1)), list[0])
        val old = com.spizganed.quickbuds.protocol.EqCodec.parseList(body)!!
        assertEquals(old[0].freqs, list[0].bands.map { it.first })
        assertEquals(old[0].gains, list[0].bands.map { it.second })
        assertNull(dec(m(0x8122, 0, 2, 0, 0xFA, 6, 1, 1, 'a'.code, 0)))
    }

    @Test fun connectionFramesMatchTheShippingBuilders() {
        assertEquals(Frame(0x0205, b(3, 1, 2, 3)), Cmd.register(listOf(1, 2, 3)))
        assertTrue(payloadOf(OpoProtocol.registerNotifications(listOf(1, 2, 3))).contentEquals(Cmd.register(listOf(1, 2, 3)).payload))
        assertEquals(Frame(0x0201, b(4)), Cmd.registerOne(4))
        val buds4 = Bitmap.parse(b(0, 0xFF, 0x77, 0x5A, 0xEA, 0x67, 0x0E, 0x20, 0x07))!!
        val offered = listOf(1, 2, 3, 4, 8, 0x0B, 0xF1, 0xF2, 0xF3)
        assertEquals(OpoProtocol.notifyIds(buds4.allows(0x0405), buds4.allows(0x040D), buds4.allows(0x0412), offered.toSet()).sorted(), Cmd.notifyIds(buds4, offered).sorted())
        assertEquals(listOf(1, 2), Cmd.notifyIds(Bitmap.UNKNOWN, listOf(2, 1)))
        for (id in listOf(0x065414, 0x100100, 0x100102, 0x100103, 0x108201, 0x060C12))
            assertEquals(com.spizganed.quickbuds.protocol.Capabilities.productId(b(0, id and 0xFF, (id shr 8) and 0xFF, id shr 16)), Cmd.productText(id))
        assertEquals("138.138.105", Cmd.firmwareText(listOf(Triple(1, 2, "138"), Triple(2, 2, "138"), Triple(3, 1, "01"), Triple(3, 2, "105"))))
        val qs = listOf(Frame(0x0102, b(0xFF, 0xFF)), Frame(0x0106, b()), Frame(0x010C, b(1, 1)))
        val oldBatch = payloadOf(OpoProtocol.batch(qs.map { OpoProtocol.buildPacket(it.cmd, payload = it.payload) }))
        assertTrue(oldBatch.contentEquals(Cmd.batch(qs).payload))
    }

    @Test fun writesMatchTheShippingBuilders() {
        for ((bit, bytes) in listOf(0 to b(1, 1, 1), 1 to b(1, 1, 2), 2 to b(1, 1, 4), 4 to b(1, 1, 0x10), 11 to b(1, 1, 0, 8))) {
            assertEquals(Frame(0x0404, bytes), Cmd.ancSet(bit))
            assertTrue(payloadOf(OpoProtocol.anc(bit)).contentEquals(Cmd.ancSet(bit).payload))
        }
        for ((kind, mask) in listOf(1 to 3, 1 to 0x0807, 3 to 1, 4 to 0x0100, 1 to 0x01020304, 1 to 0))
            assertTrue("$kind $mask", payloadOf(OpoProtocol.setHoldAncModes(mask, kind)).contentEquals(Cmd.holdCycle(kind, mask).payload))
        val t = listOf(Binding(1, 1, 2, 0x11), Binding(4, 6, 6, 0x1C))
        val oldT = OpoProtocol.setKeyFunction(t.map { com.spizganed.quickbuds.protocol.KeyFunctionParser.Entry(it.dev, it.button, it.act, it.func) })
        assertTrue(payloadOf(oldT).contentEquals(Cmd.gestureTable(t).payload))
        // Call-control capture: `0401 01 04 06 06 1C`.
        assertEquals(Frame(0x0401, b(1, 4, 6, 6, 0x1C)), Cmd.gestureTable(t.drop(1)))
        assertEquals(Frame(0x0403, b(0x1D, 1)), Cmd.feature(0x1D, true))
        assertTrue(payloadOf(OpoProtocol.setFeature(0x1D, true)).contentEquals(Cmd.feature(0x1D, true).payload))
        assertEquals(Frame(0x0406, b(2)), Cmd.eqSelect(2))
        assertEquals(Frame(0x041B, b(0xFB, 5, 0xFD)), Cmd.bassSet(-3))
        assertTrue(payloadOf(OpoProtocol.setBassWaveLevel(-3)).contentEquals(Cmd.bassSet(-3).payload))
        assertEquals(Frame(0x010C, b(1, 1)), Cmd.ancRead(1, 1))
        val p = EqPreset(5, "Mine", false, -6 to 6, listOf(62 to -2, 250 to 0, 1000 to 3))
        val o = com.spizganed.quickbuds.protocol.EqCodec.Preset(5, "Mine", listOf(62, 250, 1000), listOf(-2, 0, 3), false, byteArrayOf(0xFA.toByte(), 6))
        for ((a, code) in listOf(Cmd.EqAction.Create to 1, Cmd.EqAction.Save to 2, Cmd.EqAction.Delete to 3))
            assertTrue(payloadOf(OpoProtocol.customEq(code, o)).contentEquals(Cmd.eqPreset(a, p).payload))
    }

    private fun at(n: Int, vararg tail: Int) = IntArray(n) + tail

    @Test fun settingRepliesMatchTheShippingDecoder() {
        assertEquals(Msg.AlertVolume(6), dec(m(0x8130, 0, 6)))
        assertNull(dec(m(0x8130, 1, 6)))
        assertEquals(Msg.TapLevel(2, 3), dec(m(0x8133, 0, 2, 3)))
        assertEquals(Msg.Spatial(2), dec(m(0x812A, 0, 2)))
        assertEquals(Msg.Spatial(1), Decode.decode(Message(0x0510, 0xFF, b(1))))
        assertEquals(Msg.HeadGesture(1), dec(m(0x8134, 0, 1)))
        assertEquals(Msg.Codec(8), dec(m(0x8114, 0, 8)))
        assertEquals(Msg.CodecMask(0x0146), dec(m(0x8123, 0, 0x46, 0x01)))
        assertEquals(Msg.GameSound(3, listOf(0, 3)), dec(m(0x812B, 0, 3, 2, 0, 3)))
        assertEquals(Msg.PersonalAncStored(true), dec(m(0x811A, 0, 1)))
        assertEquals(Msg.Preferred(null), dec(m(0x8132, 0, 2, 0)))
        assertEquals(Msg.Preferred(listOf(1, 2, 3, 4, 5, 6)), dec(m(0x8132, 0, 2, 1, 1, 2, 3, 4, 5, 6)))
        val list = b(0, 1, 6, 5, 4, 3, 2, 1, 8, 2, 1, 5) + "Phone".toByteArray()
        val d = (Decode.decode(Message(0x8112, 1, list)) as Msg.Devices).list[0]
        assertEquals(Device(listOf(1, 2, 3, 4, 5, 6), "Phone", true, 1), d)
        assertEquals(listOf(2, 3, 7), Cmd.codecsOf(0x0046))
    }

    @Test fun goldenSoundRepliesAndFrames() {
        val v = listOf(-55, -49, -41, -35, -25, -19, -11, -5, 3, 7, 13, 17)
        val apply = Cmd.goldenApply(0x01020304, "2026/09/29 01:53", v, ByteArray(168) { 9 }, 11)
        assertEquals(listOf(0x0411, 0x040E, 0x0415, 0x0403), apply.map { it.cmd })
        // The record the apply frame writes reads back as the active record.
        assertEquals(Msg.GoldenRecord(0x01020304, "2026/09/29 01:53", v), Decode.decode(Message(0x8115, 1, byteArrayOf(0) + apply[1].payload)))
        val scan = byteArrayOf(0, 3) + b(168, 0) + ByteArray(168) { 9 } + b(0, 0, 0, 7)
        assertEquals(Msg.GoldenScan(7, List(168) { 9 }), dec(Message(0x811E, 1, scan)))
        val floats = (0 until 12).flatMap { java.nio.ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putFloat(it.toFloat()).array().toList() }.toByteArray()
        val hf = byteArrayOf(0) + b(0, 0, 0, 9, 4, 12, 0, 0, 2) + floats
        val g = dec(Message(0x8116, 1, hf)) as Msg.GoldenFilter
        assertEquals(9, g.id); assertEquals(2, g.enhance)
        assertEquals(Curves(44100, (0..5).map { it.toFloat() }, (6..11).map { it.toFloat() }), g.curves)
        val sf = byteArrayOf(0) + b(0, 0, 0, 9, 4, 0x44, 0xAC, 12, 0, 0) + floats
        assertEquals(44100, (dec(Message(0x811F, 1, sf)) as Msg.GoldenScanFilter).curves.rate)
        assertNull(dec(m(0x811F, 0, 0, 0, 0, 9, 3, 0, 0, 0, 0, 0)))
        // Frames against the shipping builders.
        val iv = v.toIntArray()
        assertTrue(payloadOf(OpoProtocol.earScan(true, 0x01020304)).contentEquals(Cmd.earScan(true, 0x01020304).payload))
        assertTrue(payloadOf(OpoProtocol.hearingTest(false)).contentEquals(Cmd.hearingTest(false).payload))
        assertTrue(payloadOf(OpoProtocol.hearingTone(2, 3, -30)).contentEquals(Cmd.tone(2, 3, -30).payload))
        assertTrue(payloadOf(OpoProtocol.hearingFilter(7, iv)).contentEquals(Cmd.goldenFilter(7, v).payload))
        assertTrue(tail(OpoProtocol.earScanFilter(7, ByteArray(168) { 9 }), 174).contentEquals(Cmd.goldenScanFilter(7, ByteArray(168) { 9 }).payload))
        assertTrue(tail(OpoProtocol.hearingRecord(0x01020304, "2026/09/29 01:53", iv), apply[1].payload.size).contentEquals(apply[1].payload))
        assertTrue(payloadOf(OpoProtocol.hearingRestore(11)).contentEquals(apply[0].payload))
        assertTrue(tail(OpoProtocol.earScanData(0x01020304, ByteArray(168) { 9 }), 175).contentEquals(apply[2].payload))
    }

    @Test fun settingFramesMatchTheShippingBuilders() {
        assertTrue(payloadOf(OpoProtocol.setCodec(8, true)).contentEquals(Cmd.codec(8, true).payload))
        assertTrue(payloadOf(OpoProtocol.setCodec(2, true)).contentEquals(Cmd.codec(2, true).payload))
        assertEquals(Frame(0x041A, b(3, 0, 0)), Cmd.codec(3, false))
        // LHDC (7) is dropped when LHDC V5 (8) is offered too.
        assertEquals(listOf(1, 2, 3, 6, 8), Cmd.codecsOf(0x00E7))
        assertTrue(payloadOf(OpoProtocol.setAlertVolume(6)).contentEquals(Cmd.alertVolume(6).payload))
        assertTrue(payloadOf(OpoProtocol.setTapLevel(3)).contentEquals(Cmd.tapLevel(3).payload))
        assertTrue(payloadOf(OpoProtocol.setGameSoundType(3)).contentEquals(Cmd.gameSound(3).payload))
        assertTrue(payloadOf(OpoProtocol.gameEqState(true)).contentEquals(Cmd.gameEq(true).payload))
        assertTrue(payloadOf(OpoProtocol.setSpatialType(2)).contentEquals(Cmd.spatial(2).payload))
        assertTrue(payloadOf(OpoProtocol.spatialOnBuds()).contentEquals(Cmd.spatialSource(0).payload))
        assertTrue(payloadOf(OpoProtocol.setHeadMotionType(1)).contentEquals(Cmd.headGesture(1).payload))
        assertTrue(payloadOf(OpoProtocol.findTone(true)).contentEquals(Cmd.find(true).payload))
        assertTrue(payloadOf(OpoProtocol.fitTest(false)).contentEquals(Cmd.fitTest(false).payload))
        assertTrue(payloadOf(OpoProtocol.personalNoise(2)).contentEquals(Cmd.personalAnc(2).payload))
        assertTrue(payloadOf(OpoProtocol.queryPreferred()).contentEquals(Cmd.preferredRead().payload))
        assertTrue(payloadOf(OpoProtocol.connectDevice("01:02:03:04:05:06", true)).contentEquals(Cmd.deviceConnect(listOf(1, 2, 3, 4, 5, 6), true).payload))
        assertTrue(payloadOf(OpoProtocol.setPreferred(null)).contentEquals(Cmd.preferredSet(null).payload))
        assertTrue(payloadOf(OpoProtocol.setPreferred("01:02:03:04:05:06")).contentEquals(Cmd.preferredSet(listOf(1, 2, 3, 4, 5, 6)).payload))
        // Phone state: `08 00 01` after dual off, `08 00 00` after on.
        assertEquals(Frame(0x0413, b(8, 0, 1)), Cmd.phoneState(Cmd.PhoneState(autoSwitch = true)))
        assertTrue(payloadOf(OpoProtocol.dualFollowup(false)).contentEquals(Cmd.phoneState(Cmd.PhoneState(autoSwitch = true)).payload))
        assertTrue(payloadOf(OpoProtocol.dualFollowup(true)).contentEquals(Cmd.phoneState(Cmd.PhoneState(autoSwitch = false)).payload))
        assertEquals(Frame(0x0413, b(0x1F, 0, 1, 1, 0, 1, 1)), Cmd.phoneState(Cmd.PhoneState(1, true, false, true, true)))
        assertEquals(Frame(0x0413, b(0, 0)), Cmd.phoneState(Cmd.PhoneState()))
    }

    @Test fun requestsFromTheBudsAreAnsweredLikeTheShippingCode() {
        val unix = 0x67001234L
        val cases = listOf(
            0x0500 to b(0, 0, 0, 0), 0x0500 to b(), 0x050C to b(), 0x051C to b(), 0x051D to b(),
            0x0204 to b(0xF2, 1), 0x0204 to b(0xF4), 0x0204 to b(3, 2, 3, 1, 3), 0x0501 to b(), 0x0502 to b(), 0x0504 to b(1), 0x0600 to b(), 0x0900 to b(9),
            0x8105 to b(0), 0x0F00 to b(),
        )
        for ((cmd, p) in cases) {
            val new = Decode.answer(Message(cmd, 0x14, p), unix)
            val old = OpoProtocol.answerFor(cmd, 0x14, p, unix)
            if (old == null) assertNull("$cmd", new)
            else {
                assertTrue("$cmd", new != null)
                // The shipping answer is a whole packet: its cmd at 4, payload from 9.
                assertEquals(OpoProtocol.u16(old, 4), new!!.cmd)
                assertTrue("$cmd ${p.toList()}", payloadOf(old).contentEquals(new.payload))
            }
        }
        // The JSON echo (the shipping code parses with the Android library, which a JVM test lacks).
        val ack = Decode.answer(Message(0x0204, 0xFF, byteArrayOf(0xF4.toByte()) + "{\"cmd\":\"a2dp_caton\",\"ver\":\"1\"}".toByteArray()), 0)!!
        assertEquals(0x8204, ack.cmd)
        assertEquals("{\"cmd\":\"a2dp_caton\"}", String(ack.payload, 2, ack.payload.size - 2))
        assertEquals("{}", String(Decode.answer(Message(0x0204, 1, byteArrayOf(0xF4.toByte()) + "{\"x\":[1,{\"a\":null}],\"cmd\":3}".toByteArray()), 0)!!.payload, 2, 2))
        val bad = Decode.answer(Message(0x0204, 1, byteArrayOf(0xF4.toByte()) + "not json".toByteArray()), 0)!!
        assertTrue(bad.payload.contentEquals(byteArrayOf(1, 0xF4.toByte())))
        assertEquals(0x21, Decode.answer(Message(0x0500, 0x21, b(0, 0, 0, 0)), 0)!!.seq)
        assertEquals(listOf(0, 0x34, 0x12, 0, 0x67), Decode.answer(Message(0x0500, 0x14, b()), unix)!!.payload.map { it.toInt() and 0xFF })
    }
}
