import AppKit
import Combine
import BudsProtocol

final class BudsStore: ObservableObject {
    enum Connection { case disconnected, connecting, connected }
    @Published var devices: [PairedDevice] = []
    @Published var connection = Connection.disconnected
    @Published var deviceName = "QuickBuds"
    @Published var model: EarbudModel?
    @Published var battery: [BatteryCell] = []
    @Published var noiseMode: Int?
    @Published var error: String?
    @Published var applying = false
    @Published var firmware = "—"
    private let transport = BluetoothTransport()
    private var framer = Framer()
    private var sequence: UInt8 = 0
    private var bitmap: [UInt8] = []
    private var offered: [UInt8]?
    private var initialization: Task<Void, Never>?
    private var pending: NoiseOperation?
    private var failureTimer: Task<Void, Never>?

    init() {
        transport.onOpen = { [weak self] in self?.opened() }
        transport.onData = { [weak self] in self?.receive($0) }
        transport.onClose = { [weak self] reason in
            guard let self else { return }
            self.disconnect()
            self.error = reason
        }
        refreshDevices()
    }

    var status: String {
        switch connection {
        case .disconnected: return text("conn_off")
        case .connecting: return text("conn_connecting")
        case .connected: return text("dual_connected")
        }
    }

    var canControlNoise: Bool { connection == .connected && supports(bit: 8) && model != nil }
    func refreshDevices() { devices = BluetoothTransport.paired().sorted { ($0.name == "OPPO Enco Air5" ? 0 : 1) < ($1.name == "OPPO Enco Air5" ? 0 : 1) } }

    func connect(_ device: PairedDevice) {
        disconnect()
        error = nil
        deviceName = device.name
        model = Catalog.models.first { $0.name == device.name }
        battery = []
        noiseMode = nil
        firmware = "—"
        bitmap = []
        offered = nil
        sequence = 0
        connection = .connecting
        transport.connect(device.device)
    }

    func disconnect() {
        initialization?.cancel()
        initialization = nil
        failureTimer?.cancel()
        failureTimer = nil
        if pending != nil { error = featureError("feature_no_answer") }
        pending = nil
        applying = false
        transport.disconnect()
        framer = Framer()
        connection = .disconnected
    }

    private func supports(bit: Int) -> Bool {
        bitmap.count > bit / 8 && bitmap[bit / 8] & (1 << (bit % 8)) != 0
    }

    @discardableResult private func send(_ command: UInt16, _ payload: [UInt8] = [], sequence fixed: UInt8? = nil) -> UInt8 {
        sequence = sequence >= 0xBF ? 1 : sequence + 1
        let seq = fixed ?? sequence
        transport.write(Message(command, sequence: seq, payload: payload).encoded)
        return seq
    }

    private func opened() {
        connection = .connected
        initialization = Task { @MainActor [weak self] in
            guard let self else { return }
            do {
                try await Task.sleep(nanoseconds: 300_000_000)
                for command: UInt16 in [0x0100, 0x0103, 0x0200] {
                    try Task.checkCancellation()
                    self.send(command)
                    try await Task.sleep(nanoseconds: 200_000_000)
                }
                // Only subscribe to codes actually offered by the device.
                let codes: [UInt8] = [1, 3].filter { self.offered?.contains($0) ?? true }
                if self.supports(bit: 22) { self.send(0x0205, [UInt8(codes.count)] + codes) }
                else {
                    for code in codes {
                        self.send(0x0201, [code])
                        try await Task.sleep(nanoseconds: 200_000_000)
                    }
                }
                try await Task.sleep(nanoseconds: 200_000_000)
                self.send(0x0106, sequence: 0xF0)
                if self.supports(bit: 8) {
                    try await Task.sleep(nanoseconds: 200_000_000)
                    self.send(0x010C, [1, 1])
                }
                if self.supports(bit: 0) {
                    try await Task.sleep(nanoseconds: 200_000_000)
                    self.send(0x0105)
                }
            } catch { /* Cancellation belongs to disconnect. */ }
        }
    }

    func setNoise(_ mode: NoiseMode) {
        guard canControlNoise, !applying, let slot = mode.slot, (0..<32).contains(slot) else { return }
        var payload: [UInt8] = [1, 1] + Array(repeating: 0, count: slot / 8 + 1)
        payload[2 + slot / 8] = 1 << (slot % 8)
        let seq = send(0x0404, payload)
        pending = NoiseOperation(sequence: seq, wanted: mode.mode)
        applying = true
        error = nil
        startFailureTimer()
    }

    private func startFailureTimer() {
        failureTimer?.cancel()
        failureTimer = Task { @MainActor [weak self] in
            do { try await Task.sleep(nanoseconds: 3_000_000_000) } catch { return }
            guard let self, self.pending != nil else { return }
            self.error = featureError("feature_no_answer")
            self.finishOperation()
        }
    }

    private func finishOperation() {
        pending = nil
        applying = false
        failureTimer?.cancel()
        failureTimer = nil
    }

    private func settle(_ result: NoiseOperation.Result?) {
        guard let result else { return }
        switch result {
        case .applied: break
        case .notChanged: error = featureError("feature_not_changed")
        case .refused(let status):
            error = status == 14 ? text("anc_need_ear") : featureError(status == 1 ? "feature_refused_missing" : "feature_refused_now")
        }
        finishOperation()
    }

    func refreshBattery() {
        guard connection == .connected else { return }
        send(0x0106, sequence: 0xF0)
    }

    private func mergeBattery(_ cells: [BatteryCell]) {
        // Missing parts and zero levels mean unknown, not an empty battery.
        for cell in cells {
            battery.removeAll { $0.part == cell.part }
            battery.append(cell)
        }
    }

    private func receive(_ bytes: [UInt8]) {
        for message in framer.feed(bytes) { handle(message) }
    }

    private func handle(_ message: Message) {
        let p = message.payload
        switch message.command {
        case 0x8100 where p.count > 1 && p[0] == 0:
            bitmap = Array(p.dropFirst())
        case 0x8103:
            if let id = Decode.product(p) { model = Catalog.models.first { $0.id == id && $0.name == deviceName } ?? Catalog.models.first { $0.id == id } }
        case 0x8200 where p.count >= 2 && p[0] == 0 && p.count >= 2 + Int(p[1]):
            offered = Array(p.dropFirst(2).prefix(Int(p[1])))
        case 0x8106 where p.first == 0:
            if let cells = Decode.battery(Array(p.dropFirst())) { mergeBattery(cells) }
        case 0x810C where p.first == 0:
            reportNoise(Array(p.dropFirst()), isPush: false)
        case 0x8105 where p.count > 2 && p[0] == 0:
            let parts = String(decoding: p.dropFirst(2), as: UTF8.self).split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
            let versions = stride(from: 0, to: parts.count - parts.count % 3, by: 3).compactMap { i in parts[i + 1] == "2" ? parts[i + 2] : nil }
            if !versions.isEmpty { firmware = versions.joined(separator: ".") }
        case 0x8404:
            if let status = p.first, var operation = pending, operation.sequence == message.sequence {
                let result = operation.ack(sequence: message.sequence, status: status)
                pending = operation
                if status == 0 { startFailureTimer() }
                settle(result)
            }
        case 0x0204 where !p.isEmpty:
            switch p[0] {
            case 1: if let cells = Decode.battery(Array(p.dropFirst())) { mergeBattery(cells) }
            case 3: reportNoise(Array(p.dropFirst()), isPush: true)
            case 0xF2: reply(message, [0, 0xF2])
            case 0xF4:
                if let json = try? JSONSerialization.jsonObject(with: Data(p.dropFirst())), let object = json as? [String: Any] {
                    let echo = object["cmd"] as? String
                    let data = (try? JSONSerialization.data(withJSONObject: echo.map { ["cmd": $0] } ?? [:], options: [.sortedKeys])) ?? Data("{}".utf8)
                    reply(message, [0, 0xF4] + data)
                } else { reply(message, [1, 0xF4]) }
            default: break
            }
        case 0x0500:
            let time = UInt32(clamping: Int64(Date().timeIntervalSince1970))
            reply(message, [0] + (0..<4).map { UInt8((time >> ($0 * 8)) & 255) })
        case 0x050C: reply(message, [0, 1, 0])
        case 0x051C: reply(message, [0, 0, 0])
        case 0x051D: reply(message, [1])
        default: break
        }
    }

    private func reply(_ request: Message, _ payload: [UInt8]) {
        transport.write(Message(request.command | 0x8000, sequence: request.sequence, payload: payload).encoded)
    }

    private func reportNoise(_ data: [UInt8], isPush: Bool) {
        guard let mask = Decode.anc(data), let mode = model?.reportedMode(mask) else { return }
        noiseMode = mode
        if isPush, var operation = pending {
            let result = operation.push(mode: mode)
            pending = operation
            settle(result)
        }
    }

    func openBluetoothSettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.BluetoothSettings") { NSWorkspace.shared.open(url) }
    }
}
