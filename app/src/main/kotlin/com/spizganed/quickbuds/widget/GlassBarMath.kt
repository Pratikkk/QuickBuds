package com.spizganed.quickbuds.widget

/**
 * The Glass case bar's arithmetic, separate from Android so a JVM test can check it.
 * Distances are dp, [numberSp] is sp.
 */
internal object GlassBarMath {

    /** The capsule is never shorter than this. */
    const val MIN_BAR_DP = 16f

    /** The number in the bar never falls below this size. */
    const val MIN_NUMBER_SP = 11f

    /** A 1% level draws at least this wide. */
    const val FILL_FLOOR_DP = 2f

    /** The bar's height: 70% of the case row, but at least [MIN_BAR_DP]. */
    fun barHeightDp(caseRowDp: Float): Float = maxOf(MIN_BAR_DP, 0.7f * caseRowDp)

    /** The number's size: 52% of the bar's height, but at least [MIN_NUMBER_SP]. */
    fun numberSp(barHeightDp: Float): Float = maxOf(MIN_NUMBER_SP, 0.52f * barHeightDp)

    /** The fill's width: the level's share of the bar, with a floor for a small level and no fill at 0. */
    fun fillWidthDp(barWidthDp: Float, level: Int): Float {
        if (level < 1) return 0f
        return maxOf(FILL_FLOOR_DP, barWidthDp * level.coerceAtMost(100) / 100f)
    }

    /** The gradient's bottom colour: the accent darkened by 12%, with the same alpha. */
    fun gradientBottom(accent: Int): Int {
        fun channel(shift: Int) = Math.round(((accent ushr shift) and 0xFF) * 0.88f)
        return (accent and 0xFF000000.toInt()) or (channel(16) shl 16) or (channel(8) shl 8) or channel(0)
    }

    /**
     * The height of the compact Glass battery page: page padding, the ring panel, the gap and the
     * 34 dp case row. [numberDp] is the percentage's line height under the 26 dp ring.
     */
    fun compactPageHeightDp(numberDp: Float): Float = 2 * 8f + (3f + 26f + 3f + numberDp + 3f) + 4f + 34f
}
