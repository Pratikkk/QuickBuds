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

    /** The compact Glass page's spacing, from widget_pages_gl.xml: page padding, row padding and icon margin. */
    const val COMPACT_PAGE_PAD_DP = 5f
    const val COMPACT_ROW_PAD_DP = 4f
    const val COMPACT_ICON_MARGIN_DP = 6f

    /** The compact Glass case icon's glyph height: 14 dp, as approved. */
    const val COMPACT_ICON_DP = 14f

    /** The case glyph's width / height (496 / 400). */
    const val CASE_ICON_RATIO = 496f / 400f

    /** The case icon bitmap's transparent border on each side, px (room for the glyph's shadow). */
    const val CASE_ICON_BORDER_PX = 6f

    /**
     * The compact case icon view's width in dp: the bitmap's width, from its 14 dp glyph and the border.
     * The view uses the bitmap's real size, so the glyph shows at 14 dp.
     */
    fun barIconWidthDp(density: Float): Float {
        val hpx = (COMPACT_ICON_DP * density).toInt()
        val wpx = Math.round(hpx * CASE_ICON_RATIO)
        return (wpx + 2 * CASE_ICON_BORDER_PX) / density
    }

    /** The compact row's width for the bar: the page less the page, row and icon space. */
    fun barSlotDp(pageWidthDp: Float, density: Float): Float =
        pageWidthDp - 2 * COMPACT_PAGE_PAD_DP - 2 * COMPACT_ROW_PAD_DP - barIconWidthDp(density) - COMPACT_ICON_MARGIN_DP

    /** The compact bar's bitmap width: the slot less 1 dp, so pixel rounding never scales it down. */
    fun barWidthDp(pageWidthDp: Float, density: Float): Float =
        (barSlotDp(pageWidthDp, density) - 1f).coerceAtLeast(20f)

    /** The bar's width inside the 1 dp rim on each side. */
    fun innerBarWidthDp(outerWidthDp: Float): Float = (outerWidthDp - 2f).coerceAtLeast(1f)

    /**
     * The height of the compact Glass battery page: page padding, the ring panel, the gap and the
     * 34 dp case row. [numberDp] is the percentage's line height under the 26 dp ring.
     */
    fun compactPageHeightDp(numberDp: Float): Float = 2 * 8f + (3f + 26f + 3f + numberDp + 3f) + 4f + 34f
}
