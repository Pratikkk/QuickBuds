package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol.Capabilities
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The shipping bitmap table holds all 69 bits of PROTOCOL.md section 4. */
class CapabilitiesTest {
    private fun bytes(vararg v: Int) = ByteArray(v.size) { v[it].toByte() }
    private fun onlyBit(bit: Int) = ByteArray(10).also { it[1 + bit / 8] = (1 shl (bit % 8)).toByte() }

    @Test fun lateBitsEnableTheirCommands() {
        assertTrue(0x0419 in Capabilities.parse(onlyBit(35))!!)
        assertEquals(setOf(0x0136, 0x0433), setOf(0x0136, 0x0433).filter { it in Capabilities.parse(onlyBit(64))!! }.toSet())
        assertTrue(listOf(0x0432, 0x0135, 0x0435).all { it in Capabilities.parse(onlyBit(67))!! })
        assertTrue(0x0117 in Capabilities.parse(onlyBit(68))!!)
        assertTrue(0x0419 !in Capabilities.parse(onlyBit(34))!!)
    }

    @Test fun buds4Capture() {
        val c = Capabilities.parse(bytes(0, 0xFF, 0x77, 0x5A, 0xEA, 0x67, 0x0E, 0x20, 0x07))!!
        assertTrue(0x012F in c && 0x0427 in c && 0x0100 in c)
        assertTrue(0x0422 !in c)
    }

    @Test fun malformedIsNull() {
        assertNull(Capabilities.parse(bytes(1, 0xFF)))
        assertNull(Capabilities.parse(bytes(0)))
    }
}
