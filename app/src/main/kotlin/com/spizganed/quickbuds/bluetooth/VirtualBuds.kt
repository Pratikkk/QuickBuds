package com.spizganed.quickbuds.bluetooth

import android.content.Context
import com.spizganed.quickbuds.ui.ThemeRes

/**
 * Dev tools › Virtual buds: connect to a local fake earbuds server instead of Bluetooth.
 *
 * Off by default. Nothing changes for a user who never opens Dev tools.
 * The server answers as OnePlus Buds 4, so every feature row can be reached without a phone
 * and without real earbuds (see the developer brief for the fake server).
 */
object VirtualBuds {

    const val KEY_ON = "virtualBudsOn"
    const val KEY_ADDRESS = "virtualBudsAddress"
    const val KEY_NAME = "virtualBudsName"
    /** `10.0.2.2` is the host machine's address as seen by the Android emulator. */
    const val DEFAULT_ADDRESS = "10.0.2.2:7980"
    const val DEFAULT_NAME = "OnePlus Buds 4"

    private fun prefs(context: Context) =
        context.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE)

    fun on(context: Context): Boolean = prefs(context).getBoolean(KEY_ON, false)

    fun address(context: Context): String =
        prefs(context).getString(KEY_ADDRESS, DEFAULT_ADDRESS)?.takeIf { it.isNotBlank() } ?: DEFAULT_ADDRESS

    fun name(context: Context): String =
        prefs(context).getString(KEY_NAME, DEFAULT_NAME)?.takeIf { it.isNotBlank() } ?: DEFAULT_NAME

    /** `host:port` -> (host, port). Null when the port is missing or out of range. Pure, so a test calls it. */
    fun parseAddress(text: String): Pair<String, Int>? {
        val at = text.lastIndexOf(':')
        if (at <= 0) return null
        val host = text.substring(0, at).trim()
        val port = text.substring(at + 1).trim().toIntOrNull() ?: return null
        if (host.isEmpty() || port !in 1..65535) return null
        return host to port
    }
}
