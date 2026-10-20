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

    private const val URL_PING = "https://quickbuds-report.spizganed.workers.dev/ping"
    private const val KEY_DAY = "usagePingDay"

    fun send(c: Context) {
        val prefs = c.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE)
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
                    if (conn.responseCode == 200) prefs.edit().putLong(KEY_DAY, today).apply()
                } finally { conn.disconnect() }
            } catch (_: Exception) { }
        }.start()
    }
}
