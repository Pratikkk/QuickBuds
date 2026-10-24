package com.spizganed.quickbuds.ui

import android.content.Context
import android.widget.Switch

/** Sets a state that comes from the earbuds or from code, not from a tap: no slide, no fade, the same in every style. */
fun Switch.setFromBuds(on: Boolean) {
    isChecked = on
    jumpDrawablesToCurrentState()
}

/** A switch whose tap does not toggle it: the tap becomes a request for the opposite value. */
open class RequestSwitch(context: Context) : Switch(context) {
    var onRequest: ((wantOn: Boolean) -> Unit)? = null
    override fun toggle() { val r = onRequest; if (r != null) r(!isChecked) else super.toggle() }
}
