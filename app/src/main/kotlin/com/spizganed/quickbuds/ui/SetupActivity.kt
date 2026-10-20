package com.spizganed.quickbuds.ui

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.spizganed.quickbuds.R
import com.spizganed.quickbuds.bluetooth.BudsService

/**
 * First-run setup, once: welcome and protocol, permissions, the background service and the update check, privacy, done.
 * Every choice stays in Settings. The crash report and the model test keep their own prompts (they depend on what just happened).
 */
class SetupActivity : Activity() {

    private val total = 5
    private var page = 0
    private lateinit var body: LinearLayout
    private lateinit var back: TextView
    private lateinit var next: TextView
    private lateinit var dots: LinearLayout
    private val prefs by lazy { getSharedPreferences(ThemeRes.PREFS_NAME, MODE_PRIVATE) }
    private val p by lazy { ThemeRes.palette(this) }
    private fun dp(v: Float) = ThemeRes.dp(this, v)

    override fun onCreate(savedInstanceState: Bundle?) {
        ThemeRes.select(this)
        super.onCreate(savedInstanceState)
        page = savedInstanceState?.getInt("page") ?: 0

        val root = SettingRowFactory.screen(this)
        body = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(16f), 0, dp(16f), 0) }
        root.addView(ScrollView(this).apply {
            addView(body)
            layoutParams = LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f)
        })

        back = TextView(this).apply {
            setText(R.string.setup_back)
            textSize = 15f
            typeface = ThemeRes.medium(context)
            setTextColor(p.textSecondary)
            gravity = Gravity.CENTER
            setPadding(dp(12f), 0, dp(12f), 0)
            setOnClickListener { show(page - 1) }
        }
        dots = LinearLayout(this).apply { gravity = Gravity.CENTER }
        next = TextView(this).apply {
            textSize = 15f
            typeface = ThemeRes.bold(context)
            gravity = Gravity.CENTER
            setTextColor(ThemeRes.onFill(this@SetupActivity))
            background = ThemeRes.ripple(this@SetupActivity, ThemeRes.pill(this@SetupActivity, p.accent, 22f))
            setPadding(dp(24f), 0, dp(24f), 0)
            setOnClickListener { if (page == total - 1) finishSetup() else show(page + 1) }
        }
        root.addView(LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(16f), 0, dp(16f), 0)
            addView(back, LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, dp(44f)))
            addView(dots, LinearLayout.LayoutParams(0, dp(44f), 1f))
            addView(next, LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, dp(44f)))
        })
        setContentView(root)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT) { goBack() }
        }
        show(page)
    }

    override fun onSaveInstanceState(out: Bundle) { super.onSaveInstanceState(out); out.putInt("page", page) }

    /** Back goes to the previous page, then leaves the app from the first one. */
    private fun goBack() { if (page > 0) show(page - 1) else finish() }

    @Suppress("DEPRECATION", "GestureBackNavigation") // Android 12 and older call this; 13 and newer use the callback in onCreate.
    @Deprecated("Deprecated in Java")
    override fun onBackPressed() = goBack()

    private fun show(i: Int) {
        page = i.coerceIn(0, total - 1)
        body.removeAllViews()
        when (page) {
            0 -> { heading(R.string.welcome_title, R.string.welcome_body); icon() }
            1 -> permissions()
            2 -> background()
            3 -> { heading(R.string.about_privacy_title, R.string.about_privacy_body) }
            else -> { heading(R.string.setup_ready_title, R.string.setup_ready_body); icon() }
        }
        back.visibility = if (page == 0) View.INVISIBLE else View.VISIBLE
        next.setText(if (page == total - 1) R.string.welcome_done else R.string.setup_next)
        dots.removeAllViews()
        for (n in 0 until total) dots.addView(View(this).apply {
            background = ThemeRes.pill(this@SetupActivity, if (n == page) p.accent else p.outline, 4f)
            layoutParams = LinearLayout.LayoutParams(dp(if (n == page) 18f else 8f), dp(8f)).apply { marginStart = dp(3f); marginEnd = dp(3f) }
        })
    }

    private fun icon() = body.addView(ImageView(this).apply {
        setImageDrawable(ThemeRes.tint(this@SetupActivity, R.drawable.ic_launcher_monochrome, p.accent))
        background = ThemeRes.pill(this@SetupActivity, p.card, 48f)
        layoutParams = LinearLayout.LayoutParams(dp(96f), dp(96f)).apply { gravity = Gravity.CENTER_HORIZONTAL; topMargin = dp(40f) }
    })

    private fun heading(title: Int, text: Int) {
        body.addView(TextView(this).apply {
            setText(title)
            textSize = 26f
            typeface = ThemeRes.headline(context)
            setTextColor(p.text)
            setPadding(dp(4f), dp(24f), dp(4f), dp(8f))
        })
        body.addView(TextView(this).apply {
            setText(text)
            textSize = 15f
            setTextColor(p.textSecondary)
            setLineSpacing(0f, 1.2f)
            setPadding(dp(4f), 0, dp(4f), dp(16f))
        })
    }

    private fun switchRow(icon: Int, title: Int, sub: Int, key: String, default: Boolean): View {
        val sw = SettingRowFactory.buildSwitch(this, prefs.getBoolean(key, default))
        sw.setOnCheckedChangeListener { _, on -> prefs.edit().putBoolean(key, on).apply() }
        return SettingRowFactory.build(this, icon, title, sub, sw) { sw.performClick() }
    }

    private fun background() {
        heading(R.string.setup_bg_title, R.string.setup_bg_body)
        body.addView(SettingRowFactory.card(this).apply {
            addView(switchRow(R.drawable.ic_power, R.string.settings_background_title, R.string.settings_background_sub, BudsService.PREF_BACKGROUND, true))
            addView(switchRow(R.drawable.ic_update, R.string.update_auto_title, R.string.update_auto_sub, UpdateChecker.KEY_AUTO, true))
        })
    }

    private fun bluetoothPermissions(): Array<String> =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) arrayOf(Manifest.permission.BLUETOOTH_CONNECT, Manifest.permission.BLUETOOTH_SCAN)
        else arrayOf(Manifest.permission.ACCESS_FINE_LOCATION)

    private fun notificationPermissions(): Array<String> =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) arrayOf(Manifest.permission.POST_NOTIFICATIONS) else emptyArray()

    private fun granted(list: Array<String>) = list.all { checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }

    private fun permissions() {
        heading(R.string.setup_perm_title, R.string.setup_perm_body)
        fun row(title: Int, list: Array<String>, code: Int): View {
            val ok = granted(list)
            val action = TextView(this).apply {
                setText(if (ok) R.string.setup_allowed else R.string.setup_allow)
                textSize = 14f
                typeface = ThemeRes.bold(context)
                gravity = Gravity.CENTER
                isSingleLine = true
                minWidth = dp(96f)
                setPadding(dp(16f), 0, dp(16f), 0)
                if (ok) setTextColor(p.textSecondary)
                else {
                    setTextColor(ThemeRes.onFill(this@SetupActivity))
                    background = ThemeRes.ripple(this@SetupActivity, ThemeRes.pill(this@SetupActivity, p.accent, 18f))
                    setOnClickListener { requestPermissions(list, code) }
                }
                layoutParams = LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, dp(36f))
            }
            return LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(16f), dp(12f), dp(16f), dp(12f))
                addView(TextView(this@SetupActivity).apply {
                    setText(title)
                    textSize = 16f
                    setTextColor(p.text)
                }, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))
                addView(action)
            }
        }
        body.addView(SettingRowFactory.card(this).apply {
            addView(row(R.string.setup_perm_bt, bluetoothPermissions(), 1))
            if (notificationPermissions().isNotEmpty()) addView(row(R.string.setup_perm_notif, notificationPermissions(), 2))
        })
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (page == 1) show(1)
    }

    private fun finishSetup() {
        prefs.edit().putBoolean(KEY_DONE, true).apply()
        startActivity(Intent(this, MainActivity::class.java))
        finish()
    }

    companion object {
        private const val KEY_DONE = "setupDone"
        fun done(a: Activity) = a.getSharedPreferences(ThemeRes.PREFS_NAME, MODE_PRIVATE).getBoolean(KEY_DONE, false)
    }
}
