package com.spizganed.quickbuds.protocol

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Runs every case of `testdata/protocol_cases.json` against the legacy protocol layer.
 * The JSON file is the contract that the merged layer and the Rust core must also keep.
 */
class ProtocolCasesTest {
    private val file = File("../testdata/protocol_cases.json")

    @Test fun cases() {
        assertTrue("missing contract: ${file.absolutePath}", file.isFile)
        val root = JSONObject(file.readText())
        assertTrue("version is not 1", root.getInt("version") == 1)
        val cases = root.getJSONArray("cases")
        assertTrue("no cases", cases.length() > 0)
        for (i in 0 until cases.length()) {
            val c = cases.getJSONObject(i)
            val id = c.getString("id")
            val actual = ProtocolCases.actual(c)
            assertTrue(
                "$id: expected ${ProtocolCases.text(c.get("expect"))}, actual ${ProtocolCases.text(actual)}",
                ProtocolCases.same(c.get("expect"), actual)
            )
        }
    }
}

/** The shared codec: it turns one JSON case into the legacy output, and it compares JSON values. */
object ProtocolCases {

    /** A request or reply, with the fixed header offsets removed. */
    data class Packet(val cmd: Int, val seq: Int, val payload: ByteArray)

    /** Builders whose sequence byte is fixed by the caller or by the builder. */
    private val FIXED_SEQ_BUILDERS = setOf("setFeature", "queryStatus", "queryBattery", "queryWearingStatus")

    fun actual(c: JSONObject): Any? = when (c.getString("kind")) {
        "frame" -> frames(c.getJSONArray("input"))
        "decode" -> decode(bytes(c.getString("input")), if (c.isNull("model")) null else c.getString("model"))
        "reply" -> reply(bytes(c.getString("input")), c.optLong("clock", 0L))?.let { hex(it) }
        "build" -> build(c.getJSONObject("input"))
        else -> error("unknown kind ${c.getString("kind")}")
    }

    /** A byte stream through the legacy framer. Chunks are appended in order. */
    private fun frames(chunks: JSONArray): JSONArray {
        val framer = OppoPacketFramer()
        val out = JSONArray()
        for (i in 0 until chunks.length()) {
            val chunk = bytes(chunks.getString(i))
            for (frame in framer.append(chunk, chunk.size)) out.put(hex(frame))
        }
        return out
    }

    /** One whole packet through the legacy decoder that the command selects. */
    private fun decode(frame: ByteArray, model: String?): Any? {
        val p = packet(frame)
        return when (p.cmd) {
            0x8106 -> battery(BatteryParser.parse(frame))
            0x8109 -> wearing(WearingStatusParser.parseQueryResponse(p.payload))
            0x8108 -> table(KeyFunctionParser.parse(p.payload))
            0x8122 -> presets(EqCodec.parseList(p.payload))
            0x0503 -> features(FeaturePushParser.parse(p.payload))
            0x0204 -> when (p.payload.firstOrNull()?.toInt()?.and(0xFF)) {
                0x01 -> battery(BatteryParser.parseActive(frame))
                0x02 -> wearing(WearingStatusParser.parseActiveReport(p.payload))
                0x03 -> anc(p.payload)
                0x05 -> GameModeParser.parseActive(p.payload, model)
                0xF1 -> UserInteractionParser.describe(p.payload)
                else -> error("no legacy decoder for 0x0204 subtype ${hex(p.payload)}")
            }
            else -> error("no legacy decoder for cmd 0x%04X".format(p.cmd))
        }
    }

    /** The answer to a request from the buds. The clock is fixed in the case. */
    private fun reply(frame: ByteArray, clock: Long): ByteArray? {
        val p = packet(frame)
        return OpoProtocol.answerFor(p.cmd, p.seq, p.payload, clock)
    }

    /** One builder call. The expected values are the command and the payload, plus the seq when it is fixed. */
    private fun build(input: JSONObject): JSONObject {
        val name = input.getString("builder")
        val frame = buildFrame(name, input.getJSONObject("args"))
        val p = packet(frame)
        val out = JSONObject().put("cmd", p.cmd).put("payload", hex(p.payload))
        if (name in FIXED_SEQ_BUILDERS) out.put("seq", p.seq)
        return out
    }

    private fun buildFrame(name: String, a: JSONObject): ByteArray = when (name) {
        "handshake" -> OpoProtocol.buildHandshake()
        "productId" -> OpoProtocol.buildQueryProductId()
        "broadcastCodes" -> OpoProtocol.buildQueryBroadcastCodes()
        "registerNotifications" -> OpoProtocol.registerNotifications(ints(a.getJSONArray("ids")))
        "registerOne" -> OpoProtocol.registerOne(a.getInt("id"))
        "queryBattery" -> OpoProtocol.queryBattery()
        "queryWearingStatus" -> OpoProtocol.queryWearingStatus()
        "queryStatus" -> OpoProtocol.queryStatus(
            if (a.isNull("statusQuery")) null else a.getString("statusQuery"),
            a.optBoolean("windNoise", false),
            a.optInt("seq", 0)
        )
        "setFeature" -> OpoProtocol.setFeature(a.getInt("id"), a.getBoolean("on"), a.getInt("seq"))
        "anc" -> OpoProtocol.anc(a.getInt("bit"))
        "setHoldAncModes" -> OpoProtocol.setHoldAncModes(a.getInt("mask"), a.optInt("type", OpoProtocol.HOLD_TYPE_SHARED))
        "findTone" -> OpoProtocol.findTone(a.getBoolean("on"))
        "fitTest" -> OpoProtocol.fitTest(a.getBoolean("on"))
        "setAlertVolume" -> OpoProtocol.setAlertVolume(a.getInt("level"))
        "setBuiltInEq" -> OpoProtocol.setBuiltInEq(a.getInt("id"))
        "setBassWaveLevel" -> OpoProtocol.setBassWaveLevel(a.getInt("level"))
        "customEq" -> OpoProtocol.customEq(a.getInt("action"), preset(a.getJSONObject("preset")))
        "setOnCall" -> OpoProtocol.setOnCall(a.getInt("act"), a.getInt("fn"), a.optInt("button", KeyFunctionParser.BUTTON_ON_CALL))
        "setKeyFunction" -> OpoProtocol.setKeyFunction(entries(a.getJSONArray("entries")))
        "setPreferred" -> OpoProtocol.setPreferred(if (a.isNull("mac")) null else a.getString("mac"))
        "dualFollowup" -> OpoProtocol.dualFollowup(a.getBoolean("dualOn"))
        "earScan" -> OpoProtocol.earScan(a.getBoolean("on"), a.getInt("uid"))
        "hearingTone" -> OpoProtocol.hearingTone(a.getInt("side"), a.getInt("freq"), a.getInt("value"))
        "hearingTest" -> OpoProtocol.hearingTest(a.getBoolean("on"))
        "setGameSoundType" -> OpoProtocol.setGameSoundType(a.getInt("type"))
        "setSpatialType" -> OpoProtocol.setSpatialType(a.getInt("type"))
        "setCodec" -> OpoProtocol.setCodec(a.getInt("codec"), a.getBoolean("hiRes"))
        "queryKeyFunction" -> OpoProtocol.queryKeyFunction()
        "queryAncMode" -> OpoProtocol.queryAncMode()
        "querySmartLevel" -> OpoProtocol.querySmartLevel()
        "setTapLevel" -> OpoProtocol.setTapLevel(a.getInt("level"))
        "setHeadMotionType" -> OpoProtocol.setHeadMotionType(a.getInt("type"))
        "personalNoise" -> OpoProtocol.personalNoise(a.getInt("action"))
        "gameEqState" -> OpoProtocol.gameEqState(a.getBoolean("inGame"))
        else -> error("unknown builder $name")
    }

    private fun preset(o: JSONObject) = EqCodec.Preset(
        o.getInt("id"),
        o.getString("name"),
        ints(o.getJSONArray("freqs")),
        ints(o.getJSONArray("gains")),
        o.optBoolean("selected", false),
        bytes(o.getString("tag"))
    )

    private fun entries(a: JSONArray): List<KeyFunctionParser.Entry> = (0 until a.length()).map {
        val e = a.getJSONArray(it)
        KeyFunctionParser.Entry(e.getInt(0), e.getInt(1), e.getInt(2), e.getInt(3))
    }

    // --- Result shapes, as plain JSON ---

    private fun battery(r: BatteryParser.Result?): Any? {
        if (r == null) return null
        return JSONObject().put("left", info(r.left)).put("right", info(r.right)).put("case", info(r.case))
    }

    private fun info(i: BatteryParser.Info?): Any =
        if (i == null) JSONObject.NULL else JSONObject().put("level", i.level).put("charging", i.isCharging)

    private fun wearing(r: WearingStatusParser.Result?): Any? {
        if (r == null) return null
        return JSONObject()
            .put("leftValid", r.leftValid)
            .put("rightValid", r.rightValid)
            .put("leftStatus", r.leftStatus)
            .put("rightStatus", r.rightStatus)
            .put("caseStatus", r.caseStatus)
    }

    private fun anc(payload: ByteArray): Any? {
        if (!AncEventParser.isAncEvent(payload)) return JSONObject().put("isEvent", false)
        return JSONObject()
            .put("isEvent", true)
            .put("raw", AncEventParser.rawValue(payload))
            .put("describe", AncEventParser.describe(payload))
    }

    private fun table(t: KeyFunctionParser.Table) = JSONObject()
        .put("status", t.status)
        .put("count", t.count)
        .put("entries", JSONArray(t.entries.map { e ->
            JSONArray(listOf(e.deviceType, e.button, e.action, e.function))
        }))
        .put("trailing", hex(t.trailing))
        .put("layoutMismatch", t.layoutMismatch)

    private fun presets(list: List<EqCodec.Preset>?): Any? {
        if (list == null) return null
        return JSONArray(list.map { p ->
            JSONObject()
                .put("id", p.id)
                .put("name", p.name)
                .put("freqs", JSONArray(p.freqs))
                .put("gains", JSONArray(p.gains))
                .put("selected", p.selected)
                .put("tag", hex(p.tag))
        })
    }

    private fun features(map: Map<Int, Int>?): Any? {
        if (map == null) return null
        val out = JSONObject()
        for ((id, value) in map.toSortedMap()) out.put(id.toString(), value)
        return out
    }

    // --- Bytes and JSON ---

    /** `AA`, LEB128 length, slice 00, cmd LE, seq, payLen LE, payload. */
    fun packet(frame: ByteArray): Packet {
        val lenBytes = OppoPacketFramer.readLength(frame)?.second ?: error("bad frame length ${hex(frame)}")
        val cmdAt = lenBytes + 3
        require(frame.size >= cmdAt + 5) { "short frame ${hex(frame)}" }
        val payLen = OpoProtocol.u16(frame, cmdAt + 3)
        val start = cmdAt + 5
        return Packet(
            OpoProtocol.u16(frame, cmdAt),
            frame[cmdAt + 2].toInt() and 0xFF,
            frame.copyOfRange(start, minOf(start + payLen, frame.size))
        )
    }

    fun bytes(text: String): ByteArray {
        val clean = text.trim()
        if (clean.isEmpty()) return ByteArray(0)
        return clean.split(Regex("\\s+")).map { it.toInt(16).toByte() }.toByteArray()
    }

    fun hex(bytes: ByteArray): String = OpoProtocol.bytesToHex(bytes)

    private fun ints(a: JSONArray): List<Int> = (0 until a.length()).map { a.getInt(it) }

    fun text(value: Any?): String = when (value) {
        null, JSONObject.NULL -> "null"
        else -> value.toString()
    }

    /** A JSON value comparison that ignores object key order. */
    fun same(expected: Any?, actual: Any?): Boolean {
        val e = expected ?: JSONObject.NULL
        val a = actual ?: JSONObject.NULL
        if (e == JSONObject.NULL || a == JSONObject.NULL) return e == JSONObject.NULL && a == JSONObject.NULL
        return when (e) {
            is JSONObject -> {
                if (a !is JSONObject) return false
                val eKeys = keys(e)
                eKeys == keys(a) && eKeys.all { same(e.get(it), a.get(it)) }
            }
            is JSONArray -> a is JSONArray && e.length() == a.length() &&
                (0 until e.length()).all { same(e.get(it), a.get(it)) }
            is Number -> a is Number && e.toString() == a.toString()
            is Boolean -> a is Boolean && e == a
            is String -> a == e
            else -> e == a
        }
    }

    private fun keys(o: JSONObject): Set<String> = o.keys().asSequence().toSet()
}
