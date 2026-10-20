package com.spizganed.quickbuds.protocol2

import android.content.Context
import com.spizganed.quickbuds.ui.ThemeRes

/** The "new protocol" setting: the rebuilt protocol layer (docs/PROTOCOL.md). Off unless the user turns it on. */
object ProtocolNext {
    const val KEY = "protocolNext"
    /** Set once the update dialog has been answered, so it shows once. */
    private const val KEY_ASKED = "protocolNextAsked"
    /** The first release the update dialog is for. */
    private const val FIRST_MAJOR = 5

    private fun prefs(c: Context) = c.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE)

    /** Set by the connection manager: the address of the buds in use, for the per-device fallback. */
    @Volatile var device: String? = null

    /** The user chose the legacy protocol. */
    const val KEY_LEGACY = "protocolLegacy"
    /** `<address>|<versionName>` entries: buds that did not work with the new protocol in this app version. */
    private const val KEY_FALLBACK = "protocolFellBack"

    fun legacy(c: Context): Boolean = prefs(c).getBoolean(KEY_LEGACY, false)

    fun setLegacy(c: Context, on: Boolean) { prefs(c).edit().putBoolean(KEY_LEGACY, on).apply() }

    private fun version(c: Context): String =
        try { c.packageManager.getPackageInfo(c.packageName, 0).versionName ?: "" } catch (e: Exception) { "" }

    /** The new protocol failed on [address] in this app version: the legacy path runs there until the app updates. */
    fun fellBack(c: Context, address: String? = device): Boolean =
        address != null && prefs(c).getStringSet(KEY_FALLBACK, emptySet())!!.contains("$address|${version(c)}")

    fun markFellBack(c: Context, address: String) {
        val p = prefs(c)
        // Entries of other app versions are dropped.
        val keep = p.getStringSet(KEY_FALLBACK, emptySet())!!.filter { it.endsWith("|${version(c)}") }.toMutableSet()
        keep += "$address|${version(c)}"
        p.edit().putStringSet(KEY_FALLBACK, keep).apply()
    }

    /** The new protocol is the default: on unless the user chose legacy or this device fell back. */
    fun enabled(c: Context): Boolean = !legacy(c) && !fellBack(c)

    @Deprecated("The new protocol is the default; see setLegacy.")
    fun set(c: Context, on: Boolean) { prefs(c).edit().putBoolean(KEY, on).apply() }

    /** True once, from [FIRST_MAJOR] on: the app has just been updated and the user has not heard about the switch. */
    fun shouldAsk(c: Context, versionName: String): Boolean {
        val major = versionName.substringBefore('.').toIntOrNull() ?: return false
        return major >= FIRST_MAJOR && !prefs(c).getBoolean(KEY_ASKED, false)
    }

    fun markAsked(c: Context) { prefs(c).edit().putBoolean(KEY_ASKED, true).apply() }
}
