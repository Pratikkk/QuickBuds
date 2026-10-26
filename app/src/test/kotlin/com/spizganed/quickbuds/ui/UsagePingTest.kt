package com.spizganed.quickbuds.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The ping body and the reply dedupe rule ([UsagePing]). */
class UsagePingTest {

    private fun body(ack: Long) = JSONObject(UsagePing.pingBody("4.13.0", "install-1", ack))

    @Test fun thePingBodyCarriesTheStoredReplyIdAsAck() {
        val json = body(42L)
        assertEquals("Android", json.getString("platform"))
        assertEquals("4.13.0", json.getString("version"))
        assertEquals("install-1", json.getString("id"))
        assertEquals(42L, json.getLong("ack"))
    }

    @Test fun thePingBodySendsAckZeroWithNoStoredReply() {
        assertEquals(0L, body(0L).getLong("ack"))
    }

    @Test fun aReplyAlreadyStoredOrShownIsNotNew() {
        assertFalse(UsagePing.isNewReply(42L, 42L))
        assertFalse(UsagePing.isNewReply(41L, 42L))
        assertFalse(UsagePing.isNewReply(0L, 0L))
    }

    @Test fun aNewReplyIdIsNew() {
        assertTrue(UsagePing.isNewReply(43L, 42L))
        assertTrue(UsagePing.isNewReply(7L, 0L))
    }
}
