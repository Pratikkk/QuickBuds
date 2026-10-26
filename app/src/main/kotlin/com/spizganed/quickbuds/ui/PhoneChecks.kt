package com.spizganed.quickbuds.ui

/** Facts about the phone that change what the app shows. These functions are pure, so a JVM test can call them. */
object PhoneChecks {

    /** True when the phone's manufacturer is OnePlus, in any letter case. */
    fun isOnePlus(manufacturer: String?): Boolean =
        manufacturer?.equals("OnePlus", ignoreCase = true) == true
}
