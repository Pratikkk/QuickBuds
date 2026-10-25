package com.spizganed.quickbuds.bluetooth

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** The "Host and port" field of Dev tools › Virtual buds. */
class VirtualBudsTest {

    @Test
    fun readsHostAndPort() {
        assertEquals("10.0.2.2" to 7980, VirtualBuds.parseAddress("10.0.2.2:7980"))
        assertEquals("localhost" to 1, VirtualBuds.parseAddress("localhost:1"))
        assertEquals("::1" to 65535, VirtualBuds.parseAddress("::1:65535"))
        assertEquals("host" to 7981, VirtualBuds.parseAddress(" host : 7981 "))
    }

    @Test
    fun refusesWhatCannotBeASocketAddress() {
        assertNull(VirtualBuds.parseAddress(""))
        assertNull(VirtualBuds.parseAddress("10.0.2.2"))
        assertNull(VirtualBuds.parseAddress(":7980"))
        assertNull(VirtualBuds.parseAddress("10.0.2.2:"))
        assertNull(VirtualBuds.parseAddress("10.0.2.2:0"))
        assertNull(VirtualBuds.parseAddress("10.0.2.2:65536"))
        assertNull(VirtualBuds.parseAddress("10.0.2.2:abc"))
    }
}
