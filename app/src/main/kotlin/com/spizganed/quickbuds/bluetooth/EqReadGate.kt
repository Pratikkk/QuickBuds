package com.spizganed.quickbuds.bluetooth

/**
 * One EQ read set per link. The screen asks on bind and again when the link comes up.
 * The bind ask must not send: the gate starts taken because there is no link yet.
 * [onLinkUp] opens the gate for the new link, and [take] closes it.
 */
class EqReadGate {
    private var taken = true

    @Synchronized fun onLinkUp() { taken = false }

    /** True once per link: the caller sends the read set. */
    @Synchronized fun take(): Boolean {
        if (taken) return false
        taken = true
        return true
    }
}
