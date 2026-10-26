package com.spizganed.quickbuds.bluetooth

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** The diagnostic words and text for a connect: no Android types, so a plain JVM test. */
class ConnectDiagnosticsTest {

    @Test
    fun namesTheBondState() {
        assertEquals("none", ConnectDiagnostics.bondStateWord(10))
        assertEquals("bonding", ConnectDiagnostics.bondStateWord(11))
        assertEquals("bonded", ConnectDiagnostics.bondStateWord(12))
        assertEquals("unknown", ConnectDiagnostics.bondStateWord(99))
    }

    @Test
    fun listsServiceUuidsOrNone() {
        assertEquals("none", ConnectDiagnostics.formatUuids(emptyList()))
        assertEquals(
            "0000079a-d102-11e1-9b23-00025b00a5a5",
            ConnectDiagnostics.formatUuids(listOf("0000079A-D102-11E1-9B23-00025B00A5A5"))
        )
        assertEquals(
            "0000079a-d102-11e1-9b23-00025b00a5a5, 00001107-d102-11e1-9b23-00025b00a5a5",
            ConnectDiagnostics.formatUuids(
                listOf("0000079A-D102-11E1-9B23-00025B00A5A5", "00001107-D102-11E1-9B23-00025B00A5A5")
            )
        )
    }

    @Test
    fun namesKnownSocketErrorCodes() {
        assertEquals("UNSPECIFIED", ConnectDiagnostics.socketErrorName(0))
        assertEquals("L2CAP_ACL_FAILURE", ConnectDiagnostics.socketErrorName(2))
        assertEquals("L2CAP_TIMEOUT", ConnectDiagnostics.socketErrorName(14))
        assertEquals("UNIX_FILE_SOCKET_CREATION_FAILURE", ConnectDiagnostics.socketErrorName(21))
        assertNull(ConnectDiagnostics.socketErrorName(22))
    }

    @Test
    fun writesTheSocketErrorLine() {
        assertEquals("Socket error code: 2 (L2CAP_ACL_FAILURE)", ConnectDiagnostics.socketErrorLine(2))
        assertEquals("Socket error code: 22", ConnectDiagnostics.socketErrorLine(22))
    }
}
