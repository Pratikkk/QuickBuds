/// ANC confirms through a real push, including one arriving before the write ack.
/// Status reads update the display but must never settle a pending operation.
public struct NoiseOperation {
    public enum Result: Equatable { case applied, refused(UInt8), notChanged }
    public let sequence: UInt8
    public let wanted: Int
    private var acknowledged = false
    private var pushed: Int?

    public init(sequence: UInt8, wanted: Int) { self.sequence = sequence; self.wanted = wanted }

    public mutating func ack(sequence: UInt8, status: UInt8) -> Result? {
        guard sequence == self.sequence else { return nil }
        guard status == 0 else { return .refused(status) }
        acknowledged = true
        return finish()
    }

    public mutating func push(mode: Int) -> Result? {
        if pushed == nil { pushed = mode }
        return finish()
    }

    private func finish() -> Result? {
        guard acknowledged, let pushed else { return nil }
        return pushed == wanted ? .applied : .notChanged
    }
}
