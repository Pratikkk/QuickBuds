import SwiftUI
import BudsProtocol

@main
struct QuickBudsApp: App {
    @StateObject private var store = BudsStore()

    init() {
        if CommandLine.arguments.contains("--check-resources") {
            guard Catalog.models.count >= 100, text("conn_off") != "conn_off" else {
                fputs("QuickBuds resources are missing or invalid.\n", stderr)
                exit(1)
            }
            print("QuickBuds: loaded \(Catalog.models.count) models and shared translations.")
            exit(0)
        }
    }

    var body: some Scene {
        WindowGroup("QuickBuds", id: "main") {
            OverviewView(store: store)
                .frame(minWidth: 680, minHeight: 500)
        }
        .defaultSize(width: 820, height: 600)
        .commands {
            CommandGroup(replacing: .newItem) {}
        }
        MenuBarExtra("QuickBuds", systemImage: "earbuds") {
            MenuBarView(store: store)
        }
        .menuBarExtraStyle(.window)
    }
}

private struct OverviewView: View {
    @ObservedObject var store: BudsStore
    @State private var selectedDevice: String?

    var body: some View {
        NavigationSplitView {
            List(selection: $selectedDevice) {
                Section(text("model_title")) {
                    ForEach(store.devices) { device in
                        Label(device.name, systemImage: device.audioConnected ? "earbuds" : "bluetooth")
                            .tag(device.id)
                    }
                }
            }
            .overlay {
                if store.devices.isEmpty {
                    Text(text("setup_earbuds_none"))
                        .foregroundStyle(.secondary).multilineTextAlignment(.center).padding()
                }
            }
            .safeAreaInset(edge: .bottom) {
                VStack(spacing: 10) {
                    Button(text("pnc_retry"), systemImage: "arrow.clockwise") { store.refreshDevices() }
                    Button(text("setup_earbuds_pair")) { store.openBluetoothSettings() }
                }
                .buttonStyle(.borderless).padding()
            }
            .navigationTitle("QuickBuds")
            .navigationSplitViewColumnWidth(min: 210, ideal: 240)
        } detail: {
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    HStack(spacing: 16) {
                        Image(systemName: "earbuds")
                            .font(.system(size: 38)).foregroundStyle(.tint)
                            .frame(width: 72, height: 72)
                            .background(.tint.opacity(0.1), in: RoundedRectangle(cornerRadius: 18))
                        VStack(alignment: .leading, spacing: 6) {
                            Text(store.model?.name ?? store.deviceName).font(.title2.bold())
                            if store.model?.id == "06D810" { Text("Your Enco Air5").font(.caption).foregroundStyle(.secondary) }
                            Label(store.status, systemImage: store.connection == .connected ? "checkmark.circle.fill" : "circle")
                                .foregroundStyle(store.connection == .connected ? .green : .secondary)
                                .font(.callout)
                        }
                        Spacer()
                    }
                    if let error = store.error {
                        HStack(alignment: .top) {
                            Label(error, systemImage: "exclamationmark.triangle.fill")
                            Spacer()
                            Button { store.error = nil } label: { Image(systemName: "xmark") }
                                .buttonStyle(.plain).accessibilityLabel(text("dialog_close"))
                        }
                        .font(.callout).padding().background(.orange.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
                        .accessibilityElement(children: .combine)
                    }
                    if store.connection == .connected {
                        batterySection
                        if store.canControlNoise, let model = store.model, !model.selectableModes.isEmpty {
                            NoiseControlPanel(store: store)
                        }
                        GroupBox {
                            LabeledContent(text("row_firmware_title"), value: store.firmware).padding(8)
                        }
                    } else {
                        Text(text("no_buds_paired")).foregroundStyle(.secondary)
                    }
                    HStack {
                        if store.connection != .disconnected {
                            Button(text("conn_action_disconnect")) { store.disconnect() }
                        } else if let device = store.devices.first(where: { $0.id == selectedDevice }) {
                            Button(text("conn_action_connect")) { store.connect(device) }
                                .buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
                        }
                    }
                    Spacer(minLength: 0)
                }
                .padding(28)
                .frame(maxWidth: 800, alignment: .leading)
                .frame(maxWidth: .infinity)
            }
            .navigationTitle(text("desktop_nav_overview"))
            .toolbar {
                ToolbarItem {
                    Button { store.openBluetoothSettings() } label: { Image(systemName: "gearshape") }
                        .help(text("setup_earbuds_pair"))
                }
            }
        }
        .onAppear { if selectedDevice == nil { selectedDevice = store.devices.first?.id } }
        .onChange(of: store.devices.map(\.id)) { _, ids in
            if selectedDevice == nil || !ids.contains(selectedDevice!) { selectedDevice = ids.first }
        }
    }

    private var batterySection: some View {
        GroupBox {
            HStack(spacing: 16) {
                batteryCell(1, "status_left", "l.circle")
                batteryCell(2, "status_right", "r.circle")
                batteryCell(3, "status_case", "earbuds.case")
            }.padding(12)
            VStack(alignment: .leading, spacing: 8) {
                Text(store.battery.contains(where: { $0.part == 3 })
                     ? "Case: last reported level. Open the lid and refresh for a new reading."
                     : "Open the case lid near your earbuds, then refresh to read the case battery.")
                    .font(.caption).foregroundStyle(.secondary)
                Button("Refresh battery", systemImage: "arrow.clockwise") { store.refreshBattery() }
                DisclosureGroup("Battery diagnostics") {
                    Text(store.batteryDiagnostic).font(.caption.monospaced()).textSelection(.enabled)
                }.font(.caption)
            }.frame(maxWidth: .infinity, alignment: .leading).padding(8)
        } label: { Text(text("problem_cat_battery")) }
    }

    private func batteryCell(_ part: UInt8, _ key: String, _ symbol: String) -> some View {
        let cell = store.battery.first { $0.part == part }
        return VStack(spacing: 9) {
            Image(systemName: symbol).font(.title2).foregroundStyle(.secondary)
            Text(cell.map { "\($0.level)%" } ?? "—").font(.title2.monospacedDigit().weight(.semibold))
            HStack(spacing: 5) {
                BatteryIcon(cell: cell)
                Text(text(key)).foregroundStyle(.secondary)
            }.font(.caption)
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .combine)
    }
}

private struct MenuBarView: View {
    @ObservedObject var store: BudsStore
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(store.model?.name ?? "QuickBuds").font(.headline)
                Spacer(minLength: 6)
                Circle().fill(store.connection == .connected ? Color.green : Color.secondary)
                    .frame(width: 7, height: 7).accessibilityLabel(store.status)
            }
            if store.connection == .connected {
                HStack(spacing: 8) {
                ForEach([UInt8(1), 2, 3], id: \.self) { part in
                    let key = part == 1 ? "status_left" : part == 2 ? "status_right" : "status_case"
                    VStack(spacing: 2) {
                        Text(text(key)).font(.caption2).foregroundStyle(.secondary)
                        let cell = store.battery.first(where: { $0.part == part })
                        HStack(spacing: 4) {
                            BatteryIcon(cell: cell)
                            Text(cell.map { "\($0.level)%" } ?? "—")
                                .font(.callout.monospacedDigit())
                        }

                    }.frame(maxWidth: .infinity)
                }
                }
                if store.canControlNoise {
                    Divider()
                    NoiseControlPanel(store: store, compact: true)
                }
            }
            if let error = store.error { Text(error).font(.caption).foregroundStyle(.orange) }
            Divider()
            Button(text("desktop_nav_overview")) {
                openWindow(id: "main")
                NSApp.activate(ignoringOtherApps: true)
            }
            Button("Quit QuickBuds") { NSApp.terminate(nil) }.keyboardShortcut("q")
        }
        .padding(10).frame(width: 280).background(.ultraThinMaterial)
    }
}


/// Shared Control Center style rows keep the window and menu bar behavior identical.
private struct NoiseControlPanel: View {
    @ObservedObject var store: BudsStore
    var compact = false

    private func symbol(_ mode: Int) -> String {
        switch mode {
        case 1: return "speaker.slash"
        case 2: return "person.wave.2"
        case 7: return "sparkles"
        default: return "headphones"
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: compact ? 2 : 6) {
            HStack {
                Text(text("anc_section")).font(.headline).foregroundStyle(.secondary)
                Spacer()
                if store.applying { ProgressView().controlSize(.small).accessibilityLabel(text("feature_applying")) }
            }.padding(.horizontal, compact ? 6 : 10).padding(.bottom, compact ? 3 : 8)
            ForEach(store.model?.selectableModes ?? []) { mode in
                Button { store.setNoise(mode) } label: {
                    HStack(spacing: compact ? 8 : 12) {
                        Image(systemName: "checkmark")
                            .font(.body.weight(.semibold))
                            .opacity(store.noiseMode == mode.mode ? 1 : 0)
                            .frame(width: 18)
                        Image(systemName: symbol(mode.mode))
                            .font(.title3).foregroundStyle(.secondary).frame(width: 26)
                        Text(text(mode.titleKey)).font(.body)
                        Spacer(minLength: 0)
                    }
                    .padding(.horizontal, compact ? 6 : 10).padding(.vertical, compact ? 5 : 10)
                    .contentShape(RoundedRectangle(cornerRadius: 9))
                    .background(store.noiseMode == mode.mode ? Color.accentColor.opacity(0.12) : .clear,
                                in: RoundedRectangle(cornerRadius: 9))
                }
                .buttonStyle(.plain)
                .disabled(store.applying)
                .accessibilityAddTraits(store.noiseMode == mode.mode ? .isSelected : [])
            }
        }
        .padding(compact ? 6 : 12)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16))
        .overlay(RoundedRectangle(cornerRadius: 16).strokeBorder(.white.opacity(0.12)))
    }
}


private struct BatteryIcon: View {
    let cell: BatteryCell?

    private var symbol: String {
        guard let cell else { return "battery.0percent" }
        if cell.charging { return "battery.100percent.bolt" }
        switch cell.level {
        case 0...10: return "battery.0percent"
        case 11...37: return "battery.25percent"
        case 38...62: return "battery.50percent"
        case 63...87: return "battery.75percent"
        default: return "battery.100percent"
        }
    }

    private var color: Color {
        guard let cell else { return .secondary }
        if cell.charging { return .green }
        return cell.level <= 20 ? .red : .secondary
    }

    var body: some View {
        Image(systemName: symbol)
            .foregroundStyle(color)
            .accessibilityHidden(true) // The adjacent percentage supplies the value.
    }
}
