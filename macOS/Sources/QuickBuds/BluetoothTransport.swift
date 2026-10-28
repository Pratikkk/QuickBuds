import Foundation
import IOBluetooth
import BudsProtocol

struct PairedDevice: Identifiable {
    let device: IOBluetoothDevice
    var id: String { device.addressString ?? "" }
    var name: String { device.name ?? id }
    var audioConnected: Bool { device.isConnected() }
}

/// All IOBluetooth channel lifecycle operations run on the main run loop. Writes use a
/// serial worker so a slow hardware write cannot freeze SwiftUI or interleave frames.
final class BluetoothTransport: NSObject, IOBluetoothRFCOMMChannelDelegate {
    var onData: (([UInt8]) -> Void)?
    var onOpen: (() -> Void)?
    var onClose: ((String?) -> Void)?
    private var device: IOBluetoothDevice?
    private var channel: IOBluetoothRFCOMMChannel?
    private var services: [BluetoothRFCOMMChannelID] = []
    private var timeout: DispatchWorkItem?
    private var writeGate = ConnectionGate()
    private var queuedWrites = 0
    private let writer = DispatchQueue(label: "com.quickbuds.rfcomm-writes")
    private let uuids = ["0000079A-D102-11E1-9B23-00025B00A5A5", "00001107-D102-11E1-9B23-00025B00A5A5"]

    static func paired() -> [PairedDevice] {
        (IOBluetoothDevice.pairedDevices() as? [IOBluetoothDevice] ?? [])
            .map { PairedDevice(device: $0) }
            .sorted { $0.audioConnected == $1.audioConnected ? $0.name < $1.name : $0.audioConnected }
    }

    func connect(_ device: IOBluetoothDevice) {
        disconnect()
        self.device = device
        writeGate = ConnectionGate()
        let gate = writeGate
        let timer = DispatchWorkItem { [weak self] in
            guard gate.isActive else { return }
            self?.fail("Bluetooth connection timed out.")
        }
        timeout = timer
        DispatchQueue.main.asyncAfter(deadline: .now() + 12, execute: timer)
        let result = device.performSDPQuery(self)
        if result != kIOReturnSuccess { fail("Bluetooth service discovery failed (\(result)).") }
    }

    @objc func sdpQueryComplete(_ device: IOBluetoothDevice!, status: IOReturn) {
        guard let device, device === self.device else { return }
        guard status == kIOReturnSuccess else { fail("Bluetooth service discovery failed (\(status))."); return }
        services = uuids.compactMap { value in
            guard var raw = UUID(uuidString: value)?.uuid else { return nil }
            let uuid = withUnsafeBytes(of: &raw) { IOBluetoothSDPUUID(bytes: $0.baseAddress, length: 16) }
            guard let record = device.getServiceRecord(for: uuid) else { return nil }
            var id: BluetoothRFCOMMChannelID = 0
            return record.getRFCOMMChannelID(&id) == kIOReturnSuccess && id > 0 ? id : nil
        }
        openNextService()
    }

    private func openNextService() {
        guard let device, !services.isEmpty else {
            fail("No compatible earbud control service is available. Connect your earbuds in Bluetooth settings and try again.")
            return
        }
        let id = services.removeFirst()
        var next: IOBluetoothRFCOMMChannel?
        let result = device.openRFCOMMChannelAsync(&next, withChannelID: id, delegate: self)
        channel = next
        if result != kIOReturnSuccess { channel?.setDelegate(nil); channel?.close(); channel = nil; openNextService() }
    }

    func rfcommChannelOpenComplete(_ rfcommChannel: IOBluetoothRFCOMMChannel!, status error: IOReturn) {
        guard let rfcommChannel, rfcommChannel === channel else { return }
        if error != kIOReturnSuccess {
            rfcommChannel.setDelegate(nil)
            rfcommChannel.close()
            channel = nil
            openNextService()
            return
        }
        timeout?.cancel()
        timeout = nil
        onOpen?()
    }

    func rfcommChannelData(_ rfcommChannel: IOBluetoothRFCOMMChannel!, data dataPointer: UnsafeMutableRawPointer!, length dataLength: Int) {
        guard rfcommChannel === channel, let dataPointer, dataLength > 0 else { return }
        onData?(Array(UnsafeBufferPointer(start: dataPointer.assumingMemoryBound(to: UInt8.self), count: dataLength)))
    }

    func rfcommChannelClosed(_ rfcommChannel: IOBluetoothRFCOMMChannel!) {
        guard rfcommChannel === channel else { return }
        disconnect()
        onClose?(nil)
    }

    func write(_ bytes: [UInt8]) {
        guard let active = channel else { return }
        guard bytes.count <= Int(UInt16.max), queuedWrites < 64 else {
            DispatchQueue.main.async { [weak self] in
                guard let self, self.channel === active else { return }
                self.fail("The Bluetooth write queue stalled. Reconnecting.")
            }
            return
        }
        let gate = writeGate
        queuedWrites += 1
        writer.async { [weak self] in
            defer {
                DispatchQueue.main.async {
                    guard let self, self.writeGate === gate else { return }
                    self.queuedWrites = max(0, self.queuedWrites - 1)
                }
            }
            guard gate.isActive else { return }
            var data = bytes
            let result = data.withUnsafeMutableBytes { active.writeSync($0.baseAddress, length: UInt16($0.count)) }
            if result != kIOReturnSuccess {
                DispatchQueue.main.async {
                    guard let self, gate.isActive, self.channel === active else { return }
                    self.fail("Bluetooth write failed (\(result)).")
                }
            }
        }
    }

    func disconnect() {
        writeGate.invalidate()
        queuedWrites = 0
        timeout?.cancel()
        timeout = nil
        channel?.setDelegate(nil)
        channel?.close()
        channel = nil
        device = nil
        services = []
    }

    private func fail(_ message: String) {
        disconnect()
        onClose?(message)
    }
}
