package com.spizganed.quickbuds.ui

import android.content.Context
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

/**
 * Counts active installs: at most once a day the app tells the report Worker its platform, its version and its random install ID
 * (see the README, Privacy). A failed send is tried again on the next start.
 */
object UsagePing {
    private const val KEY_ID = "appId"

    /**
     * The install ID: a random UUID made on the first start and kept in the app's settings. It ties to no device, account
     * or address, and it goes away with the app's data. Sent with the daily ping and with a problem report.
     */
    fun appId(c: Context): String {
        val prefs = c.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE)
        return prefs.getString(KEY_ID, null) ?: java.util.UUID.randomUUID().toString().also { prefs.edit().putString(KEY_ID, it).apply() }
    }

    /** Forgets the install ID: the next ping or report makes a new one. */
    fun resetId(c: Context) {
        c.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE).edit().remove(KEY_ID).apply()
    }

    private const val URL_PING = "https://quickbuds-report.spizganed.workers.dev/ping"
    private const val KEY_DAY = "usagePingDay"
    private const val KEY_REPLY = "pendingReply"

    /** The developer's reply that the last ping brought, taken once; null when there is none. */
    fun takeReply(c: Context): String? {
        val prefs = c.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE)
        return prefs.getString(KEY_REPLY, null)?.also { prefs.edit().remove(KEY_REPLY).apply() }
    }

    /** Settings and the setup guide: the daily count is on unless the user turns it off. */
    const val KEY_ENABLED = "usagePingEnabled"

    fun send(c: Context) {
        val prefs = c.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE)
        if (!prefs.getBoolean(KEY_ENABLED, true)) return
        val today = System.currentTimeMillis() / 86_400_000L
        if (prefs.getLong(KEY_DAY, -1L) == today) return
        val body = JSONObject().put("platform", "Android").put("version", UpdateChecker.installed(c)).put("id", appId(c)).toString()
        Thread {
            try {
                val conn = URL(URL_PING).openConnection() as HttpURLConnection
                try {
                    conn.requestMethod = "POST"
                    conn.doOutput = true
                    conn.connectTimeout = 10_000
                    conn.readTimeout = 10_000
                    conn.setRequestProperty("Content-Type", "application/json; charset=UTF-8")
                    conn.outputStream.use { it.write(body.toByteArray()) }
                    if (conn.responseCode == 200) {
                        prefs.edit().putLong(KEY_DAY, today).apply()
                        // The answer can carry the developer's reply to a report of this install, once.
                        val reply = runCatching {
                            JSONObject(conn.inputStream.bufferedReader().readText()).optJSONObject("reply")?.optString("text")
                        }.getOrNull()
                        if (!reply.isNullOrBlank()) prefs.edit().putString(KEY_REPLY, reply).apply()
                    }
                } finally { conn.disconnect() }
            } catch (_: Exception) { }
        }.start()
    }
}
