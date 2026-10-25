package com.spizganed.quickbuds.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The tour step of a feature ([ModelTest.step]), its log line, and the automatic mark on the return. */
class ModelTestTourTest {

    @Test fun everyFeatureHasAStep() {
        assertTrue(ModelTest.FEATURES.all { ModelTest.step(it.key) != null })
    }

    @Test fun aStepLineHasOneFormat() {
        assertEquals("MODEL TEST: step anc start", ModelTest.stepLine("anc", end = false, observed = false))
        assertEquals("MODEL TEST: step anc end observed=true", ModelTest.stepLine("anc", end = true, observed = true))
        assertEquals("MODEL TEST: step eq end observed=false", ModelTest.stepLine("eq", end = true, observed = false))
    }

    @Test fun onlyAnObservedEffectMarksTheStep() {
        assertEquals(ModelTest.WORKS, ModelTest.autoMark("anc", observed = true))
        assertEquals(ModelTest.WORKS, ModelTest.autoMark("game", observed = true))
        assertEquals(ModelTest.WORKS, ModelTest.autoMark("wear", observed = true))
        assertNull(ModelTest.autoMark("anc", observed = false))
    }

    @Test fun aFeatureWithoutAnObservableReplyStaysManual() {
        for (key in listOf("hires", "spatial", "wind", "dual", "gestures", "find")) {
            assertNull(key, ModelTest.autoMark(key, observed = true))
        }
    }
}
