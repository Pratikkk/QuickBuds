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
    @Published var batteryUpdated: [UInt8: Date] = [:]
    @Published var cachedBattery: Set<UInt8> = []
    private var preferredDeviceID: String? = UserDefaults.standard.string(forKey: "preferredDeviceID")
    private var reconnectTask: Task<Void, Never>?
    private var reconnectEnabled = true
    private var reconnectPolicy = ReconnectPolicy()
    private var sleeping = false
    private var workspaceObservers: [NSObjectProtocol] = []
    private var handshakeWatchdog: Task<Void, Never>?
    @Published var nextReconnect: Date?
    @Published var noiseMode: Int?
    @Published var error: String?
    @Published var applying = false
    @Published var firmware = "—"
    @Published var batteryDiagnostic = "No battery response received yet."
    @Published var linkedDevices: [LinkedDevice] = []
    @Published var eqSelection: UInt8?
    @Published var customPresets: [CustomEQ] = []
    @Published var dualEnabled: Bool?
    @Published var audioBusy = false
    @Published var deviceDiagnostic = "No device action requested yet."
    private var devicePoll: Task<Void, Never>?
    private var deviceReadSequences: Set<UInt8> = []
    private var audioTransaction: (command: UInt16, sequence: UInt8, read: UInt16, payload: [UInt8], verify: () -> Bool)?
    private var audioRefresh: Task<Void, Never>?
    private var requestedDual: Bool?
    private var createdEQID: UInt8?
    private var audioReadSequence: UInt8?
    private var audioTimeout: Task<Void, Never>?
    private var audioConfirmationRetry: Task<Void, Never>?
    private var audioConfirmationWindow: ConfirmationWindow?
    private let transport = BluetoothTransport()
    private var framer = Framer()
    private var sequence: UInt8 = 0
    private var bitmap: [UInt8] = []
    private var offered: [UInt8]?
    private var batteryReadThrottle = ReadThrottle()
    private var deferredBatteryRead: Task<Void, Never>?
    private var initialization: Task<Void, Never>?
    private var pending: NoiseOperation?
    private var failureTimer: Task<Void, Never>?

    init() {
        transport.onOpen = { [weak self] in self?.opened() }
        transport.onData = { [weak self] in self?.receive($0) }
        transport.onClose = { [weak self] reason in
            guard let self else { return }
            self.closeConnection()
            self.error = reason ?? self.error
            self.reconnectPolicy.failed()
            self.scheduleReconnect()
        }
        let center = NSWorkspace.shared.notificationCenter
        workspaceObservers.append(center.addObserver(forName: NSWorkspace.willSleepNotification, object: nil, queue: .main) { [weak self] _ in
            guard let self else { return }
            self.sleeping = true
            self.reconnectTask?.cancel()
            self.reconnectTask = nil
            self.nextReconnect = nil
            self.closeConnection()
        })
        workspaceObservers.append(center.addObserver(forName: NSWorkspace.didWakeNotification, object: nil, queue: .main) { [weak self] _ in
            guard let self else { return }
            self.sleeping = false
            self.reconnectPolicy.reset()
            self.refreshDevices()
            self.scheduleReconnect()
        })
        refreshDevices()
        scheduleReconnect()
    }

    deinit {
        for observer in workspaceObservers { NSWorkspace.shared.notificationCenter.removeObserver(observer) }
        reconnectTask?.cancel()
        handshakeWatchdog?.cancel()
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
        reconnectPolicy.reset()
        beginConnection(device)
    }

    func connectPreferred() {
        refreshDevices()
        if let device = devices.first(where: { $0.id == preferredDeviceID })
            ?? devices.first(where: { $0.name == "OPPO Enco Air5" }) {
            connect(device)
        } else {
            error = text("no_buds_paired")
        }
    }

    private func beginConnection(_ device: PairedDevice) {
        guard !sleeping else { return }
        nextReconnect = nil
        reconnectTask?.cancel()
        reconnectTask = nil
        reconnectEnabled = true
        if preferredDeviceID != device.id {
            battery = []
            batteryUpdated = [:]
            cachedBattery = []
        }
        preferredDeviceID = device.id
        UserDefaults.standard.set(device.id, forKey: "preferredDeviceID")
        closeConnection()
        error = nil
        deviceName = device.name
        model = Catalog.models.first { $0.name == device.name }
        linkedDevices = []
        eqSelection = nil
        customPresets = []
        dualEnabled = nil
        noiseMode = nil
        firmware = "—"
        bitmap = []
        offered = nil
        sequence = 0
        batteryReadThrottle = ReadThrottle()
        batteryDiagnostic = "No battery response received yet."
        connection = .connecting
        transport.connect(device.device)
    }

    func disconnect() {
        reconnectEnabled = false
        nextReconnect = nil
        reconnectTask?.cancel()
        reconnectTask = nil
        closeConnection()
    }

    private func scheduleReconnect() {
        guard reconnectEnabled, !sleeping, connection == .disconnected, preferredDeviceID != nil, reconnectTask == nil else { return }
        let delay = reconnectPolicy.delay
        nextReconnect = Date().addingTimeInterval(TimeInterval(delay))
        reconnectTask = Task { @MainActor [weak self] in
            do {
                try await Task.sleep(nanoseconds: UInt64(delay) * 1_000_000_000)
                guard let self else { return }
                self.reconnectTask = nil
                self.nextReconnect = nil
                guard self.reconnectEnabled, !self.sleeping, self.connection == .disconnected else { return }
                self.refreshDevices()
                if let device = self.devices.first(where: { $0.id == self.preferredDeviceID && $0.audioConnected }) {
                    self.beginConnection(device)
                } else {
                    self.reconnectPolicy.failed()
                    self.scheduleReconnect()
                }
            } catch { }
        }
    }

    private func closeConnection() {
        devicePoll?.cancel()
        devicePoll = nil
        deviceReadSequences = []
        deferredBatteryRead?.cancel()
        deferredBatteryRead = nil
        handshakeWatchdog?.cancel()
        handshakeWatchdog = nil
        audioRefresh?.cancel()
        audioRefresh = nil
        audioTimeout?.cancel()
        audioConfirmationRetry?.cancel()
        audioConfirmationRetry = nil
        audioConfirmationWindow = nil
        if audioBusy { error = "The connection ended before the setting was confirmed." }
        audioTransaction = nil
        audioReadSequence = nil
        audioBusy = false
        cachedBattery = Set(battery.map(\.part))
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
        handshakeWatchdog = Task { @MainActor [weak self] in
            do { try await Task.sleep(nanoseconds: 8_000_000_000) } catch { return }
            guard let self, self.bitmap.isEmpty, self.connection == .connected else { return }
            self.closeConnection()
            self.error = "The earbuds opened a connection but did not finish setup. Retrying automatically."
            self.reconnectPolicy.failed()
            self.scheduleReconnect()
        }
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
                let codes: [UInt8] = [1, 2, 3, 6].filter { self.offered?.contains($0) ?? true }
                if self.supports(bit: 22) { self.send(0x0205, [UInt8(codes.count)] + codes) }
                else {
                    for code in codes {
                        self.send(0x0201, [code])
                        try await Task.sleep(nanoseconds: 200_000_000)
                    }
                }
                try await Task.sleep(nanoseconds: 200_000_000)
                self.refreshBattery()
                if self.supports(bit: 8) {
                    try await Task.sleep(nanoseconds: 200_000_000)
                    self.send(0x010C, [1, 1])
                }
                if self.supports(bit: 0) {
                    try await Task.sleep(nanoseconds: 200_000_000)
                    self.send(0x0105)
                }
                try await Task.sleep(nanoseconds: 200_000_000)
                self.refreshAudioControls()
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
        guard batteryReadThrottle.allow(at: ProcessInfo.processInfo.systemUptime) else {
            if deferredBatteryRead == nil {
                deferredBatteryRead = Task { @MainActor [weak self] in
                    do { try await Task.sleep(nanoseconds: 1_000_000_000) } catch { return }
                    guard let self else { return }
                    self.deferredBatteryRead = nil
                    self.refreshBattery()
                }
            }
            return
        }
        deferredBatteryRead?.cancel()
        deferredBatteryRead = nil
        send(0x0106, sequence: 0xF0)
    }

    private func mergeBattery(_ cells: [BatteryCell]) {
        // Missing parts and zero levels mean unknown, not an empty battery.
        let reported = Set(cells.map(\.part))
        cachedBattery = Set(battery.map(\.part)).union(cachedBattery).subtracting(reported)
        var updated = batteryUpdated
        let now = Date()
        for cell in cells { updated[cell.part] = now }
        batteryUpdated = updated
        let merged = (battery.filter { !reported.contains($0.part) } + cells).sorted { $0.part < $1.part }
        if battery != merged { battery = merged }
    }

    private func receive(_ bytes: [UInt8]) {
        for message in framer.feed(bytes) { handle(message) }
    }

    private func handle(_ message: Message) {
        let p = message.payload
        var validAudioReport = false
        if let transaction = audioTransaction, message.command == transaction.command | 0x8000,
           message.sequence == transaction.sequence, let status = p.first {
            if transaction.command == 0x0429 { recordDeviceDiagnostic("Acknowledgment: status \(status), sequence \(message.sequence).") }
            guard status == 0 else { finishAudio("The earbuds refused the change (status \(status))."); return }
            if transaction.command == 0x0403, supports(bit: 27) {
                send(0x0413, [8, 0, requestedDual == true ? 0 : 1])
            }
            if transaction.command == 0x0418 && p.count >= 2 { createdEQID = p[1] }
            if transaction.command != 0x0429 { audioReadSequence = send(transaction.read, transaction.payload) }
        }
        switch message.command {
        case 0x810F where p.count >= 2 && p[0] == 0: eqSelection = p[1]; validAudioReport = true
        case 0x0504 where !p.isEmpty: eqSelection = p[0]
        case 0x0503 where !p.isEmpty && p.count >= 1 + Int(p[0]) * 2:
            for i in stride(from: 1, to: 1 + Int(p[0]) * 2, by: 2) where p[i] == 0x11 { dualEnabled = p[i + 1] != 0 }
        case 0x8122 where p.first == 0:
            if let presets = CustomEQ.decode(Array(p.dropFirst())) { customPresets = presets; validAudioReport = true }
        case 0x8112 where p.first == 0:
            if let devices = LinkedDevice.decode(Array(p.dropFirst())) {
                linkedDevices = devices; validAudioReport = true
                if audioTransaction?.command == 0x0429 {
                    recordDeviceDiagnostic("Status reply: sequence \(message.sequence), \(devices.filter(\.connected).count) connected devices.")
                }
            }
        case 0x810D where p.count >= 2 && p[0] == 0:
            if p.count >= 2 + Int(p[1]) * 2 {
                for i in stride(from: 2, to: 2 + Int(p[1]) * 2, by: 2) where p[i] == 0x11 { dualEnabled = p[i + 1] != 0; validAudioReport = true }
            }
        case 0x8100 where p.count > 1 && p[0] == 0:
            bitmap = Array(p.dropFirst())
            handshakeWatchdog?.cancel()
            handshakeWatchdog = nil
            reconnectPolicy.reset()
        case 0x8103:
            if let id = Decode.product(p) { model = Catalog.models.first { $0.id == id && $0.name == deviceName } ?? Catalog.models.first { $0.id == id } }
        case 0x8200 where p.count >= 2 && p[0] == 0 && p.count >= 2 + Int(p[1]):
            offered = Array(p.dropFirst(2).prefix(Int(p[1])))
        case 0x8106 where p.first == 0:
            batteryDiagnostic = "Battery reply: " + p.map { String(format: "%02X", $0) }.joined(separator: " ")
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
            case 1:
                batteryDiagnostic = "Battery event: " + p.map { String(format: "%02X", $0) }.joined(separator: " ")
                if let cells = Decode.battery(Array(p.dropFirst())) { mergeBattery(cells) }
            case 6:
                if let devices = LinkedDevice.decode(Array(p.dropFirst())) {
                    linkedDevices = devices
                    if let active = audioTransaction, active.command == 0x0429 {
                        recordDeviceDiagnostic("Device-state push received.")
                        if active.verify() {
                            recordDeviceDiagnostic("Requested device state confirmed by push.")
                            finishAudio()
                            refreshAudioControls()
                        }
                    }
                }
            case 2: refreshBattery()
            case 3: reportNoise(Array(p.dropFirst()), isPush: true)
            case 0xF2: reply(message, [0, 0xF2])
            case 0xF4: reply(message, ControlReply.jsonAcknowledgment(Array(p.dropFirst())))
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
        if let transaction = audioTransaction, message.command == transaction.read | 0x8000,
           (message.sequence == audioReadSequence || (transaction.command == 0x0429 && deviceReadSequences.contains(message.sequence))) {
            let decision = audioConfirmationWindow?.decide(valid: validAudioReport, matches: transaction.verify(), now: ProcessInfo.processInfo.systemUptime) ?? .failed
            switch decision {
            case .confirmed:
                if transaction.command == 0x0429 { recordDeviceDiagnostic("Requested device state confirmed by read.") }
                finishAudio()
                refreshAudioControls()
            case .failed:
                finishAudio("The earbuds did not apply the requested change.")
                refreshAudioControls()
            case .retry:
                if transaction.command == 0x0429 { return } // The bounded device poll owns these reads.
                audioConfirmationRetry?.cancel()
                audioConfirmationRetry = Task { @MainActor [weak self] in
                    do { try await Task.sleep(nanoseconds: 500_000_000) } catch { return }
                    guard let self, let active = self.audioTransaction,
                          active.sequence == transaction.sequence, self.connection == .connected else { return }
                    self.audioReadSequence = self.send(active.read, active.payload)
                }
            }
        }
    }

    var hasAir5Audio: Bool { model?.id == "06D810" && connection == .connected }
    var canEQ: Bool { hasAir5Audio && supports(bit: 10) }
    var canCustomEQ: Bool { hasAir5Audio && model?.customEq == 1 && supports(bit: 34) }
    var canManageDevices: Bool { hasAir5Audio && supports(bit: 59) && model?.dualDevice?.contains("connectDisconnectDevice") == true }
    var canDualToggle: Bool { hasAir5Audio && supports(bit: 7) && dualEnabled != nil }

    func refreshAudioControls() {
        guard connection == .connected, !audioBusy else { return }
        audioRefresh?.cancel()
        var queries: [(UInt16, [UInt8])] = []
        if canEQ { queries.append((0x010F, [])) }
        if canCustomEQ { queries.append((0x0122, [])) }
        if hasAir5Audio && supports(bit: 29) { queries.append((0x0112, [])) }
        if hasAir5Audio { queries.append((0x010D, [1, 0x11])) }
        audioRefresh = Task { @MainActor [weak self] in
            do {
                for (command, payload) in queries {
                    try Task.checkCancellation()
                    guard let self, self.connection == .connected else { return }
                    self.send(command, payload)
                    try await Task.sleep(nanoseconds: 200_000_000)
                }
            } catch { }
        }
    }

    private func changeAudio(_ command: UInt16, payload: [UInt8], read: UInt16, query: [UInt8] = [], verify: @escaping () -> Bool) {
        guard connection == .connected, !audioBusy else { return }
        audioBusy = true
        let timeout: TimeInterval = command == 0x0429 ? 15 : 6
        audioConfirmationWindow = ConfirmationWindow(now: ProcessInfo.processInfo.systemUptime, timeout: timeout)
        createdEQID = nil
        audioRefresh?.cancel()
        error = nil
        let seq = send(command, payload)
        audioTransaction = (command, seq, read, query, verify)
        audioReadSequence = nil
        if command == 0x0429 {
            deviceReadSequences = []
            recordDeviceDiagnostic("Request sent: command 0429, sequence \(seq). Waiting for reported state.")
            devicePoll = Task { @MainActor [weak self] in
                do {
                    while !Task.isCancelled {
                        try await Task.sleep(nanoseconds: 1_000_000_000)
                        guard let self, self.audioTransaction?.sequence == seq, self.connection == .connected else { return }
                        let readSequence = self.send(0x0112)
                        self.deviceReadSequences.insert(readSequence)
                    }
                } catch { }
            }
        }
        audioTimeout = Task { @MainActor [weak self] in
            do { try await Task.sleep(nanoseconds: UInt64(timeout) * 1_000_000_000) } catch { return }
            if command == 0x0429 {
                self?.recordDeviceDiagnostic("No confirmation within 15 seconds.")
                self?.finishAudio("The earbuds did not report the requested phone connection. Check that the phone’s Bluetooth is on and the earbuds are paired; try connecting from the phone’s Bluetooth settings.")
            } else {
                self?.finishAudio("The earbuds did not confirm the change. Refresh and try again.")
            }
        }
    }

    private func recordDeviceDiagnostic(_ line: String) {
        let entries = deviceDiagnostic.split(separator: "\n").map(String.init)
        deviceDiagnostic = (Array(entries.suffix(11)) + [line]).joined(separator: "\n")
    }

    private func finishAudio(_ message: String? = nil) {
        devicePoll?.cancel()
        devicePoll = nil
        deviceReadSequences = []
        audioConfirmationRetry?.cancel()
        audioConfirmationRetry = nil
        audioConfirmationWindow = nil
        if let message { error = message }
        audioTimeout?.cancel()
        audioTimeout = nil
        audioTransaction = nil
        audioReadSequence = nil
        audioBusy = false
    }

    func selectEQ(_ slot: UInt8) {
        guard canEQ, model?.eqPresets?.contains(where: { $0.slot == slot }) == true else { return }
        changeAudio(0x0406, payload: [slot], read: 0x010F) { [weak self] in self?.eqSelection == slot }
    }

    func setLinkedDevice(_ device: LinkedDevice) {
        guard canManageDevices, !device.isThisDevice, !audioBusy else { return }
        deviceDiagnostic = "Request: " + (device.connected ? "disconnect" : "connect") + " " + device.name
        changeAudio(0x0429, payload: device.command.payload, read: 0x0112) { [weak self] in
            self?.linkedDevices.first(where: { $0.id == device.id })?.connected == !device.connected
        }
    }

    func setDualEnabled(_ enabled: Bool) {
        guard canDualToggle else { return }
        requestedDual = enabled
        changeAudio(0x0403, payload: [0x11, enabled ? 1 : 0], read: 0x010D, query: [1, 0x11]) { [weak self] in self?.dualEnabled == enabled }
    }

    func saveCustomEQ(_ preset: CustomEQ, create: Bool) {
        guard canCustomEQ, let payload = preset.payload(action: create ? 1 : 2) else { return }
        if create && customPresets.count >= (model?.customEqLimit ?? 3) {
            error = "The earbuds have reached their custom EQ limit."; return
        }
        changeAudio(0x0418, payload: payload, read: 0x0122) { [weak self] in
            self?.customPresets.contains(where: { (create ? $0.id == self?.createdEQID : ($0.id == preset.id && $0.selected)) && $0.name == preset.name && $0.frequencies == preset.frequencies && $0.gains == preset.gains }) == true
        }
    }

    func deleteCustomEQ(_ preset: CustomEQ) {
        guard canCustomEQ, let payload = preset.payload(action: 3) else { return }
        let count = customPresets.count
        changeAudio(0x0418, payload: payload, read: 0x0122) { [weak self] in self?.customPresets.count == count - 1 }
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
