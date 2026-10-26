package com.spizganed.quickbuds.bluetooth

/**
 * The decision to schedule the 5 s auto-retry after a failed connect. No Android types, so the
 * JVM tests cover it.
 */
object ConnectRetry {

    /**
     * True when no connect or disconnect changed [currentGeneration] since the attempt started at
     * [generationAtStart], and [attempts] still allows a retry. A user Disconnect or a new connect
     * raises the generation and stops the retry.
     */
    fun shouldAutoRetry(generationAtStart: Int, currentGeneration: Int, attempts: Int): Boolean =
        generationAtStart == currentGeneration && attempts < 5
}
