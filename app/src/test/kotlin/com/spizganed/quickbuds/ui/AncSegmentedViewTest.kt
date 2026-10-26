package com.spizganed.quickbuds.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The noise-control switcher keeps the pill on the track while it springs. A move to segment 0
 * passes below 0, and that must not mean "nothing selected".
 */
class AncSegmentedViewTest {

    /** OvershootInterpolator(1.1f) peaks at 1.0447. */
    private val peak = 1.0447f

    @Test fun moveToOneZeroAtTheOvershootPeakStopsAtZero() {
        assertEquals(0f, segmentPosition(1f, 0f, peak, 3), 0.0001f)
    }

    @Test fun overshootPeakFromEveryStartStaysOnTheTrack() {
        assertEquals(0f, segmentPosition(2f, 0f, peak, 3), 0.0001f)
        assertEquals(0f, segmentPosition(3f, 0f, peak, 3), 0.0001f)
        assertEquals(3f, segmentPosition(2f, 3f, peak, 3), 0.0001f)
        // A move between inner segments keeps the spring: only the track edges clamp.
        assertEquals(2.0447f, segmentPosition(1f, 2f, peak, 3), 0.0001f)
    }

    @Test fun everyFractionStaysOnTheTrack() {
        for (step in 0..1100) {
            val fraction = step / 1000f
            for (from in 0..3) {
                for (to in 0..3) {
                    val position = segmentPosition(from.toFloat(), to.toFloat(), fraction, 3)
                    assertTrue(
                        "from $from to $to at fraction $fraction gave $position",
                        position in 0f..3f
                    )
                }
            }
        }
    }

    @Test fun segmentZeroIsSelectedWhateverThePositionIs() {
        assertTrue(hasSelection(0))
        assertTrue(hasSelection(3))
        assertFalse(hasSelection(-1))
    }
}
