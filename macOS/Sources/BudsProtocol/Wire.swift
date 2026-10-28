import Foundation

/// Native implementation of core/src/wire.rs; see docs/PROTOCOL.md §2.
public struct Message: Equatable {
    public let command: UInt16
    public let sequence: UInt8
    public let payload: [UInt8]

    public init(_ command: UInt16, sequence: UInt8 = 0, payload: [UInt8] = []) {
        self.command = command
        self.sequence = sequence
        self.payload = payload
    }

    public var encoded: [UInt8] {
        precondition(payload.count <= 4096 - 7)
        var length = payload.count + 7
        var bytes: [UInt8] = [0xAA]
        repeat {
            let byte = UInt8(length & 0x7F)
            length >>= 7
            bytes.append(byte | (length > 0 ? 0x80 : 0))
        } while length > 0
        return bytes + [0, 0, UInt8(command & 255), UInt8(command >> 8), sequence,
                        UInt8(payload.count & 255), UInt8(payload.count >> 8)] + payload
    }
}

public struct Framer {
    private var buffer: [UInt8] = []
    private var slices: [UInt8]?
    public init() {}

    public mutating func feed(_ bytes: [UInt8]) -> [Message] {
        buffer += bytes
        var messages: [Message] = []
        while !buffer.isEmpty {
            guard let start = buffer.firstIndex(of: 0xAA) else { buffer.removeAll(); break }
            buffer.removeFirst(start)
            var length = 0
            var header: Int?
            for index in 1..<min(buffer.count, 4) {
                length |= Int(buffer[index] & 0x7F) << ((index - 1) * 7)
                if buffer[index] & 0x80 == 0 { header = index + 1; break }
            }
            guard let header else {
                if buffer.count >= 4 { buffer.removeFirst(); continue }
                break
            }
            guard (2...4096).contains(length) else { buffer.removeFirst(); continue }
            guard buffer.count >= header + length else { break }
            let body = Array(buffer[header..<header + length])
            buffer.removeFirst(header + length)
            var complete: [UInt8]?
            switch body[0] {
            case 0: complete = Array(body.dropFirst(2))
            case 1 where body.count >= 3: slices = Array(body.dropFirst(3))
            case 2 where body.count >= 3, 3 where body.count >= 3:
                if slices != nil {
                    slices! += body.dropFirst(3)
                    if slices!.count > 65540 { slices = nil }
                    if body[0] == 3 { complete = slices; slices = nil }
                }
            default: break
            }
            guard let data = complete, data.count >= 5 else { continue }
            let count = Int(data[3]) | Int(data[4]) << 8
            messages.append(Message(UInt16(data[0]) | UInt16(data[1]) << 8,
                                    sequence: data[2], payload: Array(data.dropFirst(5).prefix(count))))
        }
        return messages
    }
}

public struct BatteryCell: Equatable {
    public let part: UInt8
    public let level: UInt8
    public let charging: Bool
}

public enum Decode {
    public static func battery(_ data: [UInt8]) -> [BatteryCell]? {
        guard let count = data.first, count > 0, data.count >= 1 + Int(count) * 2 else { return nil }
        return stride(from: 1, to: 1 + Int(count) * 2, by: 2).compactMap { i in
            let level = data[i + 1] & 0x7F
            return (1...100).contains(level) ? BatteryCell(part: data[i], level: level, charging: data[i + 1] & 0x80 != 0) : nil
        }
    }

    public static func anc(_ data: [UInt8]) -> UInt32? {
        guard data.count >= 3, data[0] == 1, data[1] == 1 else { return nil }
        return data.dropFirst(2).prefix(4).reversed().reduce(UInt32(0)) { $0 << 8 | UInt32($1) }
    }

    public static func product(_ data: [UInt8]) -> String? {
        guard data.count == 4, data[0] == 0 else { return nil }
        var id = UInt32(data[1]) | UInt32(data[2]) << 8 | UInt32(data[3]) << 16
        switch id {
        case 0x100100...0x100102: id = 0x060414
        case 0x100200...0x100202: id = 0x060814
        case 0x108100...0x108102: id = 0x068414
        case 0x108200...0x108202: id = 0x068814
        default: break
        }
        return String(format: "%06X", id)
    }
}

public struct EarbudModel: Decodable {
    public let id: String
    public let name: String
    public let ancModes: [NoiseMode]?
    public let eqPresets: [EQChoice]?
    public let customEq: Int?
    public let customEqBands: [UInt16]?
    public let customEqLimit: Int?
    public let dualDevice: [String]?
}

public struct NoiseMode: Decodable, Identifiable {
    public let mode: Int
    public let slot: Int?
    public let variants: [NoiseMode]?
    public let decideByEarDevice: Bool?
    public var id: Int { mode }

    public var titleKey: String {
        switch mode {
        case 1: return "anc_seg_off"
        case 2, 6: return "anc_seg_trans"
        case 3: return "anc_mode_low"
        case 4: return "anc_mode_high"
        case 7: return "anc_mode_smart"
        case 8: return "anc_mode_medium"
        case 10: return "anc_seg_adapt"
        default: return "anc_seg_anc"
        }
    }
}

extension EarbudModel {
    /// SET slots and report aliases are distinct (e.g. Buds 4 reports Off at bit 3).
    public var selectableModes: [NoiseMode] {
        var result: [NoiseMode] = []
        for parent in ancModes ?? [] {
            if parent.decideByEarDevice != true, parent.slot != nil, parent.mode != 6 {
                result.append(parent)
            }
            for child in parent.variants ?? [] where child.mode != parent.mode && child.mode != 6 && child.slot != nil {
                if !result.contains(where: { $0.mode == child.mode }) { result.append(child) }
            }
        }
        if result.contains(where: { [3, 4, 7, 8].contains($0.mode) }) { result.removeAll { $0.mode == 5 } }
        return result
    }

    public func reportedMode(_ mask: UInt32) -> Int? {
        guard mask != 0 else { return nil }
        let bit = mask.trailingZeroBitCount
        for parent in ancModes ?? [] {
            if parent.slot == bit { return parent.mode == 6 ? 2 : parent.mode }
            if let child = parent.variants?.first(where: { $0.slot == bit }) {
                return child.mode == 6 ? parent.mode : child.mode
            }
        }
        return nil
    }
}


public struct EQChoice: Decodable, Identifiable {
    public let mode: Int
    public let slot: UInt8
    public var id: UInt8 { slot }
    public var titleKey: String {
        switch mode {
        case 26: return "eq_ultimate"
        case 28: return "eq_pure_vocals"
        case 29: return "eq_thundering_bass"
        default: return "eq_default"
        }
    }
}
