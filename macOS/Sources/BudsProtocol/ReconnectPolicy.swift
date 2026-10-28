/// Bounded retry pacing. A successful connection or an explicit user retry resets it.
public struct ReconnectPolicy {
    private var failures = 0
    public init() {}
    public var delay: Int { [2, 5, 10, 20, 30][min(failures, 4)] }
    public mutating func failed() { failures = min(failures + 1, 4) }
    public mutating func reset() { failures = 0 }
}
