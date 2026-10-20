package com.spizganed.quickbuds.ui

import android.app.Activity
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import com.spizganed.quickbuds.R
import com.spizganed.quickbuds.bluetooth.BudsService

/**
 * The one launch sheet: what the background service is for, the update check, which protocol runs and what the daily
 * count sends. Shown once; every choice stays in Settings. The crash report and the model test have their own prompts
 * (they depend on what just happened).
 */
object WelcomeSheet {
    private const val KEY_SHOWN = "welcomeShown"

    fun shouldShow(a: Activity) = !a.getSharedPreferences(ThemeRes.PREFS_NAME, Activity.MODE_PRIVATE).getBoolean(KEY_SHOWN, false)

    fun show(a: Activity) {
        val prefs = a.getSharedPreferences(ThemeRes.PREFS_NAME, Activity.MODE_PRIVATE)
        prefs.edit().putBoolean(KEY_SHOWN, true).apply()
        val p = ThemeRes.palette(a)
        val dp = { v: Float -> ThemeRes.dp(a, v) }

        fun row(icon: Int, title: Int, sub: Int, key: String, default: Boolean): android.view.View {
            val sw = SettingRowFactory.buildSwitch(a, prefs.getBoolean(key, default))
            sw.setOnCheckedChangeListener { _, on -> prefs.edit().putBoolean(key, on).apply() }
            return SettingRowFactory.build(a, icon, title, sub, sw) { sw.performClick() }
        }

        val column = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            addView(SettingRowFactory.card(a).apply {
                addView(row(R.drawable.ic_power, R.string.settings_background_title, R.string.settings_background_sub, BudsService.PREF_BACKGROUND, true))
                addView(row(R.drawable.ic_update, R.string.update_auto_title, R.string.update_auto_sub, UpdateChecker.KEY_AUTO, true))
            })
            addView(TextView(a).apply {
                setText(R.string.about_privacy_body)
                textSize = 13f
                setTextColor(p.textSecondary)
                setLineSpacing(0f, 1.15f)
                layoutParams = LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)
                    .apply { topMargin = dp(12f); marginStart = dp(4f); marginEnd = dp(4f) }
            })
        }
        BottomSheetDialog(a)
            .title(a.getString(R.string.welcome_title))
            .message(a.getString(R.string.welcome_body))
            .content(column)
            .confirm(a.getString(R.string.welcome_done)) { }
            .also { sheet -> sheet.confirm(a.getString(R.string.welcome_done)) { sheet.close() } }
            .show()
    }
}
