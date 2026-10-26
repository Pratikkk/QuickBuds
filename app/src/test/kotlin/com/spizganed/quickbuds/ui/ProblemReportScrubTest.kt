package com.spizganed.quickbuds.ui

import com.spizganed.quickbuds.protocol.OpoProtocol
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The report scrub: a frame that carries device addresses and names keeps only its 9-byte header
 * (PROTOCOL-LEGACY.md §9). The header comes from [OpoProtocol.buildPacket], so the test uses the bytes the
 * logger writes: `AA <len> 00 00 <cmd LE> <seq> <payload len LE>`, payload from byte 9.
 */
class ProblemReportScrubTest {

    /** One packet-log line as `PacketLogger` holds it: time, two spaces, then the RX line. */
    private fun line(cmd: Int, payload: ByteArray): String =
        "12:00:00.000  RX: " + OpoProtocol.bytesToHex(OpoProtocol.buildPacket(cmd, 0x11, payload))

    private fun header(cmd: Int, payload: ByteArray): String =
        OpoProtocol.bytesToHex(OpoProtocol.buildPacket(cmd, 0x11, payload).take(9).toByteArray())

    /** Push `0x0204` sub `F2`: count, MAC 6 bytes reversed, connect count u16 LE, name length, name. */
    private val f2 = byteArrayOf(
        0xF2.toByte(), 0x01,
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
        0x02, 0x00,
        0x04,
        0x42, 0x75, 0x64, 0x73
    )

    @Test fun f2ConnectedDevicesLosesItsAddressesAndName() {
        val out = ProblemReport.dropDeviceBytes(line(0x0204, f2))
        assertTrue(out, out.endsWith(" [device data removed]"))
        assertTrue(out, out.contains(header(0x0204, f2)))
        assertFalse(out, out.contains("11 22 33 44 55 66"))
        assertFalse(out, out.contains("42 75 64 73"))
    }

    @Test fun subSixDeviceListStillLosesItsBytes() {
        val payload = byteArrayOf(0x06, 0x01, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66)
        val out = ProblemReport.dropDeviceBytes(line(0x0204, payload))
        assertTrue(out, out.endsWith(" [device data removed]"))
        assertFalse(out, out.contains("11 22 33 44"))
    }

    @Test fun anotherPushSubIsLeftAlone() {
        val payload = byteArrayOf(0xF4.toByte(), 0x7B, 0x7D)
        assertEquals(line(0x0204, payload), ProblemReport.dropDeviceBytes(line(0x0204, payload)))
    }

    @Test fun batteryPushIsLeftAlone() {
        val payload = byteArrayOf(0x01, 0x02, 0x01, 0x64, 0x02, 0x64)
        assertEquals(line(0x0204, payload), ProblemReport.dropDeviceBytes(line(0x0204, payload)))
    }
}
