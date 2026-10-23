package com.spizganed.quickbuds.ui

import android.content.Context
import android.os.Build
import com.spizganed.quickbuds.R
import com.spizganed.quickbuds.protocol.ModelCatalog

/**
 * Limits of the current earbuds and phone that the person should know. The keys come from the model's
 * `knownIssues` in `models.json` and from two checks on the phone and the rows. Each key has a title and a body string.
 */
object KnownIssues {

    class Issue(val key: String, val title: Int, val body: Int)

    private val ALL = listOf(
        Issue("spatial_silent", R.string.issue_spatial_silent_title, R.string.issue_spatial_silent_body),
        Issue("samsung_codec", R.string.issue_samsung_codec_title, R.string.issue_samsung_codec_body),
        Issue("hires_spatial", R.string.issue_hires_spatial_title, R.string.issue_hires_spatial_body),
    )

    /** The issues of the current model and phone, in the order of [ALL]. Empty for an unknown model. */
    fun current(c: Context): List<Issue> {
        val model = ModelCatalog.current(c) ?: return emptyList()
        val keys = mutableSetOf<String>()
        model.json.optJSONArray("knownIssues")?.let { a -> for (i in 0 until a.length()) keys += a.getString(i) }
        val hires = MainActivity.rowSupported(c, "hires")
        if (hires && Build.MANUFACTURER.equals("samsung", ignoreCase = true)) keys += "samsung_codec"
        if (hires && MainActivity.rowSupported(c, "spatial") && !MainActivity.spatialSwitchOnly(c)) keys += "hires_spatial"
        return ALL.filter { it.key in keys }
    }

    /** The launch prompt asks once per model that has issues. */
    fun shouldAnnounce(c: Context): Boolean {
        val model = ModelCatalog.current(c) ?: return false
        return model.id !in announced(c) && current(c).isNotEmpty()
    }

    fun markAnnounced(c: Context) {
        val model = ModelCatalog.current(c) ?: return
        prefs(c).edit().putString(KEY_ANNOUNCED, (announced(c) + model.id).joinToString(",")).apply()
    }

    private fun announced(c: Context) = prefs(c).getString(KEY_ANNOUNCED, null)?.split(',').orEmpty().toSet()

    private fun prefs(c: Context) = c.getSharedPreferences(ThemeRes.PREFS_NAME, Context.MODE_PRIVATE)

    private const val KEY_ANNOUNCED = "knownIssuesAnnounced"
}
