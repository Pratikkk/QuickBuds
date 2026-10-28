import XCTest
@testable import BudsProtocol

final class ProtocolTests: XCTestCase {
    func testSharedModelCatalogDecodes() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        struct Catalog: Decodable { let whiteList: [EarbudModel] }
        let data = try Data(contentsOf: root.appendingPathComponent("app/src/main/assets/models.json"))
        let catalog = try JSONDecoder().decode(Catalog.self, from: data)
        XCTAssertGreaterThan(catalog.whiteList.count, 100)
        let buds = try XCTUnwrap(catalog.whiteList.first { $0.id == "065414" })
        XCTAssertFalse(buds.selectableModes.isEmpty)
        XCTAssertEqual(buds.reportedMode(8), 1)
    }

    func testCapturedFrames() {
        XCTAssertEqual(Message(0x0100, sequence: 1).encoded, [0xAA, 7, 0, 0, 0, 1, 1, 0, 0])
        XCTAssertEqual(Message(0x0404, sequence: 6, payload: [1, 1, 1]).encoded,
                       [0xAA, 10, 0, 0, 4, 4, 6, 3, 0, 1, 1, 1])
        var framer = Framer()
        let capture: [UInt8] = [0xAA, 12, 0, 0, 4, 2, 255, 5, 0, 3, 1, 1, 8, 0]
        let message = framer.feed(capture).first
        XCTAssertEqual(message?.command, 0x0204)
        XCTAssertEqual(message?.payload, [3, 1, 1, 8, 0])
        XCTAssertEqual(Decode.anc(Array(message!.payload.dropFirst())), 8)
    }

    func testFragmentationNoiseAndMultipleFrames() {
        let message = Message(0x8106, sequence: 0xF0, payload: [0, 3, 1, 80, 2, 0xD5, 3, 0])
        var framer = Framer()
        XCTAssertEqual(framer.feed([1, 2, 0xAA, 0xFF, 0xFF, 0xFF]), [])
        for byte in message.encoded.dropLast() { XCTAssertEqual(framer.feed([byte]), []) }
        XCTAssertEqual(framer.feed([message.encoded.last!] + message.encoded), [message, message])
        let cells = Decode.battery(Array(message.payload.dropFirst()))
        XCTAssertEqual(cells?.map(\.level), [80, 85])
        XCTAssertEqual(cells?.map(\.charging), [false, true])
        XCTAssertNil(Decode.battery([3, 1, 99]))
    }

    func testLongLengthsAndSlicedMessages() {
        let message = Message(0x0418, sequence: 3, payload: Array(repeating: 7, count: 124))
        XCTAssertEqual(Array(message.encoded.prefix(3)), [0xAA, 0x83, 1])
        var framer = Framer()
        XCTAssertEqual(framer.feed(message.encoded), [message])
        let first: [UInt8] = [0xAA, 6, 1, 0, 0, 6, 0x81, 0xF0]
        let last: [UInt8] = [0xAA, 9, 3, 0, 1, 4, 0, 0, 1, 1, 90]
        XCTAssertEqual(framer.feed(first), [])
        XCTAssertEqual(framer.feed(last), [Message(0x8106, sequence: 0xF0, payload: [0, 1, 1, 90])])
        var orphan = Framer()
        XCTAssertEqual(orphan.feed(last), [])
        // A short middle slice must not access bytes past its end.
        XCTAssertEqual(orphan.feed([0xAA, 2, 2, 0]), [])
    }

    func testProductFoldingAndReportOnlyAliases() throws {
        XCTAssertEqual(Decode.product([0, 1, 1, 0x10]), "060414")
        XCTAssertNil(Decode.product([1, 0, 0, 0]))
        let json = """
        {"id":"065414","name":"OnePlus Buds 4","ancModes":[{"mode":1,"slot":0,"variants":[{"mode":1,"slot":3}]},{"mode":2,"slot":1}]}
        """
        let model = try JSONDecoder().decode(EarbudModel.self, from: Data(json.utf8))
        XCTAssertEqual(model.selectableModes.compactMap(\.slot), [0, 1])
        XCTAssertEqual(model.reportedMode(8), 1)
        XCTAssertNil(model.reportedMode(0))
        XCTAssertNil(model.reportedMode(0x8000))
    }

    func testWriteRequiresAckAndRealPush() {
        var operation = NoiseOperation(sequence: 9, wanted: 2)
        XCTAssertNil(operation.ack(sequence: 8, status: 1))
        XCTAssertNil(operation.ack(sequence: 9, status: 0))
        XCTAssertEqual(operation.push(mode: 2), .applied)
        var early = NoiseOperation(sequence: 10, wanted: 1)
        XCTAssertNil(early.push(mode: 1))
        XCTAssertEqual(early.ack(sequence: 10, status: 0), .applied)
        var mismatch = NoiseOperation(sequence: 11, wanted: 7)
        XCTAssertNil(mismatch.ack(sequence: 11, status: 0))
        XCTAssertEqual(mismatch.push(mode: 1), .notChanged)
        var refused = NoiseOperation(sequence: 12, wanted: 2)
        XCTAssertEqual(refused.ack(sequence: 12, status: 14), .refused(14))
    }
}
