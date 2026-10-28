import XCTest
@testable import BudsProtocol

final class AudioControlsTests: XCTestCase {
    func testDeviceListReversesAddressForManagementWrites() throws {
        let name = Array("Phone".utf8)
        let packet: [UInt8] = [1, 6, 5, 4, 3, 2, 1, 8, 2, 1, 5] + name
        let device = try XCTUnwrap(LinkedDevice.decode(packet)?.first)
        XCTAssertEqual(device.mac, [1, 2, 3, 4, 5, 6])
        XCTAssertEqual(device.name, "Phone")
        XCTAssertTrue(device.connected)
        XCTAssertTrue(device.isThisDevice)
        XCTAssertEqual(device.command.command, 0x0429)
        XCTAssertEqual(device.command.payload, [2, 1, 2, 3, 4, 5, 6])
        XCTAssertNil(LinkedDevice.decode(Array(packet.dropLast())))
        XCTAssertEqual(LinkedDevice.decode([0]), [])
    }

    func testCustomEQLayoutSignedGainsAndUTF8() throws {
        var preset = CustomEQ(name: "EQ é", frequencies: [31, 1000, 16000])
        preset.gains = [-6, 0, 6]
        let payload = try XCTUnwrap(preset.payload(action: 1))
        XCTAssertEqual(Array(payload.prefix(5)), [1, 250, 6, 0, 5])
        XCTAssertEqual(Array(payload.suffix(10)), [3, 31, 0, 250, 232, 3, 0, 128, 62, 6])
        // List format is action replaced by selected, plus a leading count.
        var entry = payload
        entry[0] = 1
        entry[3] = 8
        let decoded = try XCTUnwrap(CustomEQ.decode([1] + entry)?.first)
        XCTAssertEqual(decoded.name, preset.name)
        XCTAssertEqual(decoded.gains, preset.gains)
        XCTAssertEqual(decoded.frequencies, preset.frequencies)
        XCTAssertEqual(decoded.id, 8)
        XCTAssertTrue(decoded.selected)
        XCTAssertNil(CustomEQ.decode([1] + entry.dropLast()))
        XCTAssertEqual(CustomEQ.decode([0]), [])
        preset.gains[0] = -7
        XCTAssertNil(preset.payload(action: 2))
        preset.gains = [0]
        XCTAssertNil(preset.payload(action: 2))
    }

    func testAir5PresetSlotsAndCustomBandsFromCatalog() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        struct Catalog: Decodable { let whiteList: [EarbudModel] }
        let catalog = try JSONDecoder().decode(Catalog.self, from: Data(contentsOf: root.appendingPathComponent("app/src/main/assets/models.json")))
        let model = try XCTUnwrap(catalog.whiteList.first { $0.id == "06D810" })
        XCTAssertEqual(model.eqPresets?.map(\.slot), [0, 1, 2])
        XCTAssertEqual(model.eqPresets?.map(\.titleKey), ["eq_ultimate", "eq_pure_vocals", "eq_thundering_bass"])
        XCTAssertEqual(model.customEqBands, [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000])
        XCTAssertEqual(model.dualDevice, ["connectDisconnectDevice"])
        var preset = CustomEQ(name: "My EQ", frequencies: model.customEqBands!)
        XCTAssertEqual(preset.payload(action: 1)?.count, 41)
        preset.name = String(repeating: "é", count: 33)
        XCTAssertNil(preset.payload(action: 1))
    }
}
