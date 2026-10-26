package com.spizganed.quickbuds.bluetooth

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The 5 s auto-retry decision after a failed connect: no Android types, so a plain JVM test. */
class ConnectRetryTest {

    @Test
    fun retriesWhileTheGenerationIsUnchangedAndTriesRemain() {
        assertTrue(ConnectRetry.shouldAutoRetry(generationAtStart = 7, currentGeneration = 7, attempts = 0))
        assertTrue(ConnectRetry.shouldAutoRetry(generationAtStart = 7, currentGeneration = 7, attempts = 4))
    }

    @Test
    fun doesNotRetryAfterAUserDisconnectOrANewConnect() {
        assertFalse(ConnectRetry.shouldAutoRetry(generationAtStart = 7, currentGeneration = 8, attempts = 0))
    }

    @Test
    fun doesNotRetryWhenTheFiveTriesAreUsed() {
        assertFalse(ConnectRetry.shouldAutoRetry(generationAtStart = 7, currentGeneration = 7, attempts = 5))
        assertFalse(ConnectRetry.shouldAutoRetry(generationAtStart = 7, currentGeneration = 7, attempts = 6))
    }
}
