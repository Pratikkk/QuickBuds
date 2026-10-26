package com.spizganed.quickbuds.protocol2

import com.spizganed.quickbuds.protocol.OpoProtocol
import org.junit.Assert.assertEquals
import org.junit.Test

/** The battery and wearing queries use the fixed sequence numbers of the shipping layer. */
class SeqTest {
    /** The packet that the shipping layer writes for a typed frame. */
    private fun packet(f: Frame) = OpoProtocol.buildPacket(f.cmd, f.seq, f.payload)

    private fun hex(b: ByteArray) = b.joinToString(" ") { "%02X".format(it) }

    @Test fun batteryUsesSeqF0() {
        val f = Cmd.battery()
        assertEquals("battery seq", 0xF0, f.seq)
        // Legacy queryBattery, LegacyProtocol.kt 391: AA 07 00 00 06 01 F0 00 00.
        assertEquals("AA 07 00 00 06 01 F0 00 00", hex(packet(f)))
    }

    @Test fun wearingUsesSeqF2() {
        val f = Cmd.wearing()
        assertEquals("wearing seq", 0xF2, f.seq)
        // Legacy queryWearingStatus, LegacyProtocol.kt 227: AA 07 00 00 09 01 F2 00 00.
        assertEquals("AA 07 00 00 09 01 F2 00 00", hex(packet(f)))
    }
}
