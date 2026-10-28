import SwiftUI
import BudsProtocol

struct AudioControlsView: View {
    @ObservedObject var store: BudsStore
    @State private var editor: EQEditorItem?
    @State private var deleteCandidate: CustomEQ?
    @State private var deviceCandidate: LinkedDevice?

    var body: some View {
        VStack(spacing: 20) {
            GroupBox {
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        Label(text("dual_title"), systemImage: "laptopcomputer.and.iphone").font(.headline)
                        Spacer()
                        if store.canDualToggle {
                            Toggle(text("dual_title"), isOn: Binding(get: { store.dualEnabled == true }, set: { store.setDualEnabled($0) }))
                                .labelsHidden().toggleStyle(.switch).disabled(store.audioBusy)
                        }
                    }
                    if store.linkedDevices.isEmpty {
                        Text("No devices reported yet.").foregroundStyle(.secondary)
                    }
                    Text("Connect requires an existing pairing. If the earbuds are no longer paired with your phone, pair them again in the phone’s Bluetooth settings first.")
                        .font(.caption).foregroundStyle(.secondary)
                    ForEach(store.linkedDevices) { device in
                        HStack(spacing: 12) {
                            Image(systemName: device.isThisDevice ? "desktopcomputer" : "iphone").font(.title2).foregroundStyle(.secondary)
                            VStack(alignment: .leading, spacing: 3) {
                                Text(device.name.isEmpty ? "Bluetooth device" : device.name)
                                Text(text(device.isThisDevice ? "dual_connected_this" : device.connected ? "dual_connected" : "dual_not_connected"))
                                    .font(.caption).foregroundStyle(.secondary)
                            }
                            Spacer()
                            if store.canManageDevices && !device.isThisDevice {
                                Button(text(device.connected ? "conn_action_disconnect" : "conn_action_connect")) { deviceCandidate = device }
                                    .disabled(store.audioBusy)
                            }
                        }.padding(.vertical, 4)
                    }
                    HStack {
                        Button("Refresh devices", systemImage: "arrow.clockwise") { store.refreshAudioControls() }.disabled(store.audioBusy)
                        if store.audioBusy { ProgressView().controlSize(.small) }
                    }
                    DisclosureGroup("Device connection diagnostics") {
                        Text(store.deviceDiagnostic).font(.caption.monospaced()).textSelection(.enabled)
                    }.font(.caption)
                }.padding(8)
            }
            if store.canEQ || store.canCustomEQ {
                GroupBox {
                    VStack(alignment: .leading, spacing: 10) {
                        HStack {
                            Label("Sound Master EQ", systemImage: "slider.horizontal.3").font(.headline)
                            Spacer()
                            if store.audioBusy { ProgressView().controlSize(.small) }
                        }
                        if store.canEQ {
                            ForEach(store.model?.eqPresets ?? []) { preset in
                                Button { store.selectEQ(preset.slot) } label: {
                                    HStack {
                                        Text(text(preset.titleKey))
                                        Spacer()
                                        Image(systemName: "checkmark").opacity(store.eqSelection == preset.slot ? 1 : 0)
                                    }.padding(.vertical, 8).contentShape(Rectangle())
                                }.buttonStyle(.plain).disabled(store.audioBusy)
                                    .accessibilityAddTraits(store.eqSelection == preset.slot ? .isSelected : [])
                                Divider()
                            }
                        }
                        if store.canCustomEQ {
                            Text("Custom").font(.subheadline).foregroundStyle(.secondary)
                            ForEach(store.customPresets) { preset in
                                HStack {
                                    Button {
                                        editor = EQEditorItem(preset: preset, isNew: false)
                                    } label: {
                                        HStack {
                                            Text(preset.name)
                                            if preset.selected { Image(systemName: "checkmark") }
                                        }
                                    }.buttonStyle(.plain)
                                    Spacer()
                                    Button("Apply") { store.saveCustomEQ(preset, create: false) }
                                    Button(role: .destructive) { deleteCandidate = preset } label: { Image(systemName: "trash") }
                                        .accessibilityLabel("Delete " + preset.name)
                                }.disabled(store.audioBusy).padding(.vertical, 4)
                            }
                            Button("Add equalizer", systemImage: "plus") {
                                let frequencies = store.model?.customEqBands ?? [62, 250, 1000, 4000, 8000, 16000]
                                editor = EQEditorItem(preset: CustomEQ(name: "My EQ", frequencies: frequencies), isNew: true)
                            }.disabled(store.audioBusy || store.customPresets.count >= (store.model?.customEqLimit ?? 3))
                        }
                    }.padding(8)
                }
            }
        }
        .sheet(item: $editor) { item in
            EQEditor(preset: item.preset) { preset in store.saveCustomEQ(preset, create: item.isNew) }
        }
        .confirmationDialog("Delete custom equalizer?", isPresented: Binding(get: { deleteCandidate != nil }, set: { if !$0 { deleteCandidate = nil } })) {
            Button("Delete", role: .destructive) {
                if let preset = deleteCandidate { store.deleteCustomEQ(preset) }
                deleteCandidate = nil
            }
        }
        .confirmationDialog(deviceCandidate.map { ($0.connected ? "Disconnect " : "Connect ") + $0.name + "?" } ?? "Device management", isPresented: Binding(get: { deviceCandidate != nil }, set: { if !$0 { deviceCandidate = nil } })) {
            Button(deviceCandidate?.connected == true ? "Disconnect" : "Connect") {
                if let device = deviceCandidate { store.setLinkedDevice(device) }
                deviceCandidate = nil
            }
        }
    }
}

private struct EQEditorItem: Identifiable {
    let id = UUID()
    let preset: CustomEQ
    let isNew: Bool
}

private struct EQEditor: View {
    @Environment(\.dismiss) private var dismiss
    @State var preset: CustomEQ
    let save: (CustomEQ) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Custom equalizer").font(.title2.bold())
            TextField("Name", text: $preset.name)
            ScrollView {
                VStack(spacing: 12) {
                    ForEach(preset.frequencies.indices, id: \.self) { i in
                        HStack {
                            Text(preset.frequencies[i] >= 1000 ? "\(Double(preset.frequencies[i]) / 1000, specifier: "%g") kHz" : "\(preset.frequencies[i]) Hz")
                                .frame(width: 75, alignment: .leading)
                            Slider(value: Binding(get: { Double(preset.gains[i]) }, set: { preset.gains[i] = Int8(clamping: Int($0.rounded())) }),
                                   in: Double(preset.minimum)...max(Double(preset.minimum) + 1, Double(preset.maximum)), step: 1)
                                .disabled(preset.minimum == preset.maximum)
                                .accessibilityLabel("\(preset.frequencies[i]) Hz")
                            Text("\(preset.gains[i]) dB").monospacedDigit().frame(width: 50, alignment: .trailing)
                        }
                    }
                }.padding(.vertical, 4)
            }
            HStack {
                Button("Reset") { preset.gains = Array(repeating: 0, count: preset.gains.count) }
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Button("Save") { save(preset); dismiss() }.keyboardShortcut(.defaultAction)
                    .disabled(preset.payload(action: 2) == nil)
            }
        }.padding(24).frame(width: 500, height: 520)
    }
}
