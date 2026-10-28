import Foundation

/// Invalidated before a transport disconnect; queued work must check this before sending.
public final class ConnectionGate {
    private let lock = NSLock()
    private var active = true
    public init() {}
    public var isActive: Bool {
        lock.lock(); defer { lock.unlock() }
        return active
    }
    public func invalidate() {
        lock.lock(); defer { lock.unlock() }
        active = false
    }
}

/// Uses monotonic time supplied by the caller, so wall-clock adjustments do not affect pacing.
public struct ReadThrottle {
    private var last: TimeInterval?
    public init() {}
    public mutating func allow(at now: TimeInterval, interval: TimeInterval = 1) -> Bool {
        if let last, now >= last, now - last < interval { return false }
        last = now
        return true
    }
}

public enum ControlReply {
    /// F4 accepts any valid JSON value; only a top-level string cmd is echoed.
    public static func jsonAcknowledgment(_ bytes: [UInt8]) -> [UInt8] {
        if bytes.isEmpty { return [0, 0xF4] }
        guard let value = try? JSONSerialization.jsonObject(with: Data(bytes), options: [.fragmentsAllowed]) else { return [1, 0xF4] }
        let command = (value as? [String: Any])?["cmd"] as? String
        let echo = command.map { ["cmd": $0] } ?? [:]
        guard let data = try? JSONSerialization.data(withJSONObject: echo, options: [.sortedKeys]) else { return [1, 0xF4] }
        return [0, 0xF4] + data
    }
}

/// Lets asynchronous firmware changes settle without displaying optimistic state.
public struct ConfirmationWindow {
    public enum Decision: Equatable { case confirmed, retry, failed }
    private let deadline: TimeInterval
    public init(now: TimeInterval, timeout: TimeInterval = 6) { deadline = now + timeout }
    public func decide(valid: Bool, matches: Bool, now: TimeInterval) -> Decision {
        guard valid, now < deadline else { return .failed }
        return matches ? .confirmed : .retry
    }
}
