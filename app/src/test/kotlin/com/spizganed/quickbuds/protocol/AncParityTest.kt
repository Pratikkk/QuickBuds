package com.spizganed.quickbuds.protocol

import com.spizganed.quickbuds.protocol.Wire.Message

import org.junit.Assert.assertEquals
import org.junit.Test

/** The ANC event gate is the source of truth for push 03. */
class AncParityTest {
    private fun b(vararg v: Int) = ByteArray(v.size) { v[it].toByte() }
    private fun push(vararg p: Int) = Message(0x0204, 0xFF, b(*p))

    @Test fun pushMatchesTheEventGate() {
        for (p in listOf(
            intArrayOf(3, 1, 1, 0x20, 0),
            intArrayOf(3, 1, 1, 0x00, 1),
            intArrayOf(3, 2, 1, 7, 8),
            intArrayOf(3, 4, 1, 0x20, 0),
            intArrayOf(3, 1, 1),
        )) {
            val m = push(*p)
            val old = if (AncEventParser.isAncEvent(m.payload)) {
                Msg.AncMsg(Anc.Mode(AncEventParser.rawValue(m.payload)))
            } else {
                null
            }
            assertEquals("${m.payload.toList()}", old, Decode.decode(m))
        }
    }
}
