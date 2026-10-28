import XCTest
@testable import BudsProtocol

final class RuntimeGuardsTests: XCTestCase {
    func testDisconnectInvalidatesOnlyItsOwnQueuedWork() {
        let first = ConnectionGate()
        XCTAssertTrue(first.isActive)
        first.invalidate()
        first.invalidate()
        let next = ConnectionGate()
        XCTAssertFalse(first.isActive)
        XCTAssertTrue(next.isActive)
    }

    func testReadBurstsAreThrottledWithMonotonicTime() {
        var throttle = ReadThrottle()
        XCTAssertTrue(throttle.allow(at: 100))
        for _ in 0..<100 { XCTAssertFalse(throttle.allow(at: 100.25)) }
        XCTAssertTrue(throttle.allow(at: 101))
        XCTAssertFalse(throttle.allow(at: 101.9))
        XCTAssertTrue(throttle.allow(at: 102))
        // A restarted time source must not permanently block reads.
        XCTAssertTrue(throttle.allow(at: 0))
    }

    func testControlJSONRepliesMatchCoreSemantics() throws {
        XCTAssertEqual(ControlReply.jsonAcknowledgment([]), [0, 0xF4])
        XCTAssertEqual(ControlReply.jsonAcknowledgment(Array("{".utf8)), [1, 0xF4])
        for value in ["[]", "null", "42", "{}", "{\"cmd\":5}"] {
            XCTAssertEqual(ControlReply.jsonAcknowledgment(Array(value.utf8)), [0, 0xF4] + Array("{}".utf8))
        }
        let bytes = ControlReply.jsonAcknowledgment(Array("{\"cmd\":\"hello\"}".utf8))
        XCTAssertEqual(Array(bytes.prefix(2)), [0, 0xF4])
        let object = try JSONSerialization.jsonObject(with: Data(bytes.dropFirst(2))) as? [String: String]
        XCTAssertEqual(object?["cmd"], "hello")
    }

    func testBrokenSliceSequenceCannotDeliverCorruptedMessage() {
        var framer = Framer()
        let first: [UInt8] = [0xAA, 6, 1, 0, 0, 6, 0x81, 0xF0]
        let badLast: [UInt8] = [0xAA, 9, 3, 0, 2, 4, 0, 0, 1, 1, 90]
        XCTAssertTrue(framer.feed(first).isEmpty)
        XCTAssertTrue(framer.feed(badLast).isEmpty)
        let valid = Message(0x8106, sequence: 0xF0, payload: [0, 1, 1, 90])
        XCTAssertEqual(framer.feed(valid.encoded), [valid])
    }

    func testDelayedFirmwareConfirmationIsBounded() {
        let window = ConfirmationWindow(now: 100)
        XCTAssertEqual(window.decide(valid: true, matches: false, now: 100.1), .retry)
        XCTAssertEqual(window.decide(valid: true, matches: false, now: 103), .retry)
        XCTAssertEqual(window.decide(valid: true, matches: true, now: 105), .confirmed)
        XCTAssertEqual(window.decide(valid: false, matches: true, now: 101), .failed)
        XCTAssertEqual(window.decide(valid: true, matches: false, now: 106), .failed)
        XCTAssertEqual(window.decide(valid: true, matches: true, now: 107), .failed)
    }

    func testInvalidPresetGainIsRejected() throws {
        let preset = CustomEQ(name: "Test", frequencies: [1000])
        var payload = try XCTUnwrap(preset.payload(action: 2))
        payload[payload.count - 1] = 127
        XCTAssertNil(CustomEQ.decode([1] + payload))
    }
}
