package com.spizganed.quickbuds.widget

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** The Glass case bar's size and colour arithmetic, without Android classes. */
class GlassBarMathTest {

    @Test
    fun `the smallest case row gives an 18 dp bar at least 16 dp tall`() {
        assertEquals(18.2f, GlassBarMath.barHeightDp(26f), 0.01f)
        assertEquals(16f, GlassBarMath.barHeightDp(20f), 0.01f)
        assertTrue(GlassBarMath.barHeightDp(26f) >= 16f)
    }

    @Test
    fun `the number never falls below 11 sp`() {
        assertEquals(11f, GlassBarMath.numberSp(18.2f), 0f)
        assertEquals(16.64f, GlassBarMath.numberSp(32f), 0.01f)
    }

    @Test
    fun `a 1 percent level draws at least a 2 dp fill`() {
        assertEquals(0f, GlassBarMath.fillWidthDp(66f, 0), 0f)
        assertEquals(2f, GlassBarMath.fillWidthDp(66f, 1), 0f)
        assertEquals(33f, GlassBarMath.fillWidthDp(66f, 50), 0f)
        assertEquals(66f, GlassBarMath.fillWidthDp(66f, 100), 0f)
    }

    @Test
    fun `the gradient ends at the accent darkened by 12 percent`() {
        assertEquals(0xFF4160E0.toInt(), GlassBarMath.gradientBottom(0xFF4A6DFF.toInt()))
        assertEquals(0x804160E0.toInt(), GlassBarMath.gradientBottom(0x804A6DFF.toInt()))
    }

    @Test
    fun `the compact Glass battery page fits 110 dp`() {
        assertEquals(108.2f, GlassBarMath.compactPageHeightDp(19.2f), 0.01f)
        assertTrue(GlassBarMath.compactPageHeightDp(19.2f) <= 110f)
    }
}
