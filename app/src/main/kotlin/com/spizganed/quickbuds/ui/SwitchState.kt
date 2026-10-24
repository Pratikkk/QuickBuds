package com.spizganed.quickbuds.ui

import android.widget.Switch

/** Sets a state that comes from the earbuds or from code, not from a tap: no slide, no fade, the same in every style. */
fun Switch.setFromBuds(on: Boolean) {
    isChecked = on
    jumpDrawablesToCurrentState()
}
