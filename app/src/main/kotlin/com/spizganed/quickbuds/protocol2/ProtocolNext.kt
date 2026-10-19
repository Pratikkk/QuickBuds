package com.spizganed.quickbuds.protocol2

import android.content.Context
import com.spizganed.quickbuds.ui.ThemeRes

/** The "new protocol" setting: the rebuilt protocol layer (docs/PROTOCOL-NEXT.md). Off unless the user turns it on. */
object ProtocolNext {
    const val KEY = "protocolNext"
    /** Set once the update dialog has been answered, so it shows once. */
    private const val KEY_ASKED = "protocolNextAsked"
    /** The first release the update dialog is for. */
    private const val FIRST_MAJOR = 5

    private fun prefs(c: Context) = c.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE)

    fun enabled(c: Context): Boolean = prefs(c).getBoolean(KEY, false)

    fun set(c: Context, on: Boolean) { prefs(c).edit().putBoolean(KEY, on).apply() }

    /** True once, from [FIRST_MAJOR] on: the app has just been updated and the user has not heard about the switch. */
    fun shouldAsk(c: Context, versionName: String): Boolean {
        val major = versionName.substringBefore('.').toIntOrNull() ?: return false
        return major >= FIRST_MAJOR && !prefs(c).getBoolean(KEY_ASKED, false)
    }

    fun markAsked(c: Context) { prefs(c).edit().putBoolean(KEY_ASKED, true).apply() }
}
