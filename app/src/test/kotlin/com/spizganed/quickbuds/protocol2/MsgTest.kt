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
}
