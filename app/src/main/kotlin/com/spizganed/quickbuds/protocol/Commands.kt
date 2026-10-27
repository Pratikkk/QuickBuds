package com.spizganed.quickbuds.protocol

/*
 * The command builders: one place builds each payload the app sends.
 * OpoProtocol keeps the names and signatures and delegates here. The Golden Sound builders are in
 * HearingCommands.kt. PROTOCOL-LEGACY.md documents every command.
 */

internal object Commands {

    // --- Connection ---

    fun handshake(): ByteArray = OpoProtocol.buildPacket(OpoProtocol.CMD_HANDSHAKE)
    fun queryProductId(): ByteArray = OpoProtocol.buildPacket(OpoProtocol.CMD_QUERY_PRODUCT_ID)
    fun queryBroadcastCodes(): ByteArray = OpoProtocol.buildPacket(OpoProtocol.CMD_QUERY_BROADCAST)
    fun queryFirmware(): ByteArray = OpoProtocol.buildPacket(OpoProtocol.CMD_QUERY_FIRMWARE)

    /**
     * The events to subscribe to: ours, kept to those the buds offer in `0x8200` when they sent one,
     * as HeyMelody subscribes to the offered list only (PROTOCOL-LEGACY.md §4). Some firmwares never ack a
     * list with others.
     */
    fun notifyIds(fitTest: Boolean, golden: Boolean, personalNoise: Boolean, offered: Set<Int>?): List<Int> =
        (listOf(0x01, 0x02, 0x03) + (if (fitTest) listOf(OpoProtocol.EVT_FIT_TEST) else emptyList()) +
            (if (golden) listOf(OpoProtocol.EVT_GOLDEN_STATUS) else emptyList()) +
            (if (personalNoise) listOf(OpoProtocol.EVT_PERSONAL_NOISE) else emptyList()))
            .filter { offered == null || it in offered }

    /**
     * Subscribe to spontaneous 0x0204 notifications (battery + wearing + ANC).
     *
     * Payload is canonical: a count byte followed by event ids.
     *
     * CONFIRMED on device: with count=2 and events battery (0x01) + wearing (0x02),
     * the ACK lists BOTH and the buds then emit 0x0204 subtype 02 on every wear
     * change. Wear updates are effectively instant — no polling needed.
     *
     * The old legacy literal 01 01 02 02 was a misread: under the count-first
     * shape it means count=1 (register battery only) with 02 02 left over. The
     * firmware ACKed that happily and silently never sent wear events, which is
     * why wear appeared to be poll-only. Do NOT revert to that.
     *
     * 0x03 (ANC): the buds advertise their event ids in
     * the 0x8200 broadcast-codes reply, which lists `01 02 03 04 08 0B F1 F2 F3` —
     * 03 is offered, and the ANC subType 0x03 push arrived only unreliably while we
     * had not subscribed to it. Subscribing is the documented way to make a 0x0204
     * subType arrive, and it is how wear was fixed, so the same reasoning applies.
     *
     * CONFIRMED ON DEVICE: the 0x8205 ACK lists `01 02 03`, and a bud-side
     * gesture now updates the app circles AND the widget, including a gesture REBOUND
     * to a different mode cycle. If ANC ever stops following gestures, check the ACK
     * still lists `01 02 03` first — a firmware that rejects the longer list would ACK
     * a shorter one.
     *
     * 0x04 (fit test result) is added when the buds have the fit test (`0x0405`):
     * HeyMelody registers every id the buds list in 0x8200 (`registerMultiNotification`).
     * 0x08 (Golden Sound test status) likewise, when the buds have the test (`0x040D`).
     */
    fun registerNotifications(ids: List<Int>): ByteArray =
        OpoProtocol.buildPacket(
            OpoProtocol.CMD_REGISTER_NOTIFY,
            payload = registerNotificationsPayload(ids)
        )

    internal fun registerNotificationsPayload(ids: List<Int>): ByteArray =
        byteArrayOf(ids.size.toByte()) + ids.map { it.toByte() }

    /** Subscribe one event, for buds without `0x0205` in their bitmap. `[VENDOR]` */
    fun registerOne(id: Int): ByteArray =
        OpoProtocol.buildPacket(OpoProtocol.CMD_REGISTER_ONE, payload = registerOnePayload(id))

    internal fun registerOnePayload(id: Int): ByteArray = byteArrayOf(id.toByte())
}
