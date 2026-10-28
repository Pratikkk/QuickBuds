import Foundation

public struct LinkedDevice: Identifiable, Equatable {
    public let mac: [UInt8]
    public let name: String
    public let connected: Bool
    public let flags: UInt8
    public var id: String { mac.map { String(format: "%02X", $0) }.joined() }
    public var isThisDevice: Bool { flags & 1 != 0 }
    public var command: Message { Message(0x0429, payload: [connected ? 2 : 1] + mac) }

    public static func decode(_ data: [UInt8]) -> [LinkedDevice]? {
        guard let count = data.first else { return nil }
        var offset = 1
        var result: [LinkedDevice] = []
        for _ in 0..<Int(count) {
            guard offset + 10 <= data.count else { return nil }
            let length = Int(data[offset + 9])
            guard offset + 10 + length <= data.count else { return nil }
            result.append(LinkedDevice(mac: Array(data[offset..<offset + 6].reversed()),
                name: String(decoding: data[offset + 10..<offset + 10 + length], as: UTF8.self),
                connected: data[offset + 7] == 2, flags: data[offset + 8]))
            offset += 10 + length
        }
        return result
    }
}

public struct CustomEQ: Identifiable, Equatable {
    public var id: UInt8
    public var name: String
    public var selected: Bool
    public var minimum: Int8
    public var maximum: Int8
    public var frequencies: [UInt16]
    public var gains: [Int8]

    public init(name: String, frequencies: [UInt16]) {
        id = 0; self.name = name; selected = false; minimum = -6; maximum = 6
        self.frequencies = frequencies; gains = Array(repeating: 0, count: frequencies.count)
    }

    public func payload(action: UInt8) -> [UInt8]? {
        let bytes = Array(name.utf8)
        guard !bytes.isEmpty, bytes.count <= 64, frequencies.count == gains.count,
              (1...16).contains(gains.count), minimum <= maximum,
              gains.allSatisfy({ $0 >= minimum && $0 <= maximum }) else { return nil }
        var p: [UInt8] = [action, UInt8(bitPattern: minimum), UInt8(bitPattern: maximum), id, UInt8(bytes.count)] + bytes
        p.append(UInt8(gains.count))
        for (frequency, gain) in zip(frequencies, gains) {
            p += [UInt8(frequency & 255), UInt8(frequency >> 8), UInt8(bitPattern: gain)]
        }
        return p
    }

    public static func decode(_ data: [UInt8]) -> [CustomEQ]? {
        guard let count = data.first else { return nil }
        var offset = 1
        var result: [CustomEQ] = []
        for _ in 0..<Int(count) {
            guard offset + 6 <= data.count else { return nil }
            let length = Int(data[offset + 4])
            guard offset + 6 + length <= data.count else { return nil }
            let bands = Int(data[offset + 5 + length])
            guard (1...16).contains(bands), offset + 6 + length + bands * 3 <= data.count else { return nil }
            var preset = CustomEQ(name: String(decoding: data[offset + 5..<offset + 5 + length], as: UTF8.self), frequencies: [])
            preset.selected = data[offset] == 1
            preset.minimum = Int8(bitPattern: data[offset + 1]); preset.maximum = Int8(bitPattern: data[offset + 2])
            guard preset.minimum <= preset.maximum else { return nil }
            preset.id = data[offset + 3]
            for i in 0..<bands {
                let at = offset + 6 + length + i * 3
                preset.frequencies.append(UInt16(data[at]) | UInt16(data[at + 1]) << 8)
                preset.gains.append(Int8(bitPattern: data[at + 2]))
            }
            guard preset.gains.allSatisfy({ $0 >= preset.minimum && $0 <= preset.maximum }) else { return nil }
            result.append(preset)
            offset += 6 + length + bands * 3
        }
        return result
    }
}
