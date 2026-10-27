package com.spizganed.quickbuds.bluetooth

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** One EQ read set per link: the bind call before the link does not send one. */
class EqReadGateTest {

    @Test fun noReadBeforeALink() {
        assertFalse(EqReadGate().take())
    }

    @Test fun oneReadPerLink() {
        val gate = EqReadGate()
        gate.onLinkUp()
        assertTrue("the first ask after the link sends", gate.take())
        assertFalse("a second ask in the same link is skipped", gate.take())
        gate.onLinkUp()
        assertTrue("the next link sends again", gate.take())
    }
}
