package com.spizganed.quickbuds.bluetooth

/**
 * The decision to schedule the 5 s auto-retry after a failed connect. No Android types, so the
 * JVM tests cover it.
 */
object ConnectRetry {

    /** Connect tries in one cycle, the first one included. */
    const val MAX_TRIES = 5

    /**
     * True when no connect or disconnect changed [currentGeneration] since the attempt started at
     * [generationAtStart], and fewer than [MAX_TRIES] tries ([attempts]) have failed. A user Disconnect or a new connect
     * raises the generation and stops the retry.
     */
    fun shouldAutoRetry(generationAtStart: Int, currentGeneration: Int, attempts: Int): Boolean =
        generationAtStart == currentGeneration && attempts < MAX_TRIES
}
