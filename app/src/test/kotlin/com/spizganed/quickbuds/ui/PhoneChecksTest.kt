package com.spizganed.quickbuds.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The OnePlus phone check uses the manufacturer string in any letter case. */
class PhoneChecksTest {

    @Test fun onePlusInAnyCaseIsOnePlus() {
        assertTrue(PhoneChecks.isOnePlus("OnePlus"))
        assertTrue(PhoneChecks.isOnePlus("oneplus"))
        assertTrue(PhoneChecks.isOnePlus("ONEPLUS"))
    }

    @Test fun otherManufacturersAreNotOnePlus() {
        assertFalse(PhoneChecks.isOnePlus("OPPO"))
        assertFalse(PhoneChecks.isOnePlus("realme"))
        assertFalse(PhoneChecks.isOnePlus("Nothing"))
        assertFalse(PhoneChecks.isOnePlus(""))
    }
}
