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
    @State private var section = OverviewSection.sound
    @State private var sidebarVisibility = NavigationSplitViewVisibility.automatic
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var animatedSidebarVisibility: Binding<NavigationSplitViewVisibility> {
        Binding(get: { sidebarVisibility }, set: { visibility in
            withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.25)) {
                sidebarVisibility = visibility
            }
        })
    }

    var body: some View {
        NavigationSplitView(columnVisibility: animatedSidebarVisibility) {
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
            .frame(width: 220)
            .clipped()
            .navigationSplitViewColumnWidth(min: 220, ideal: 220, max: 220)
        } detail: {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    HStack(spacing: 16) {
                        Image(systemName: "earbuds")
                            .font(.system(size: 28)).foregroundStyle(.tint)
                            .frame(width: 48, height: 48)
                            .background(.tint.opacity(0.1), in: RoundedRectangle(cornerRadius: 18))
                        VStack(alignment: .leading, spacing: 6) {
                            Text(store.model?.name ?? store.deviceName).font(.title2.bold())
                            Label(store.status, systemImage: store.connection == .connected ? "checkmark.circle.fill" : "circle")
                                .foregroundStyle(store.connection == .connected ? .green : .secondary)
                                .font(.callout)
                        }
                        Spacer(minLength: 8)
                        if store.connection != .disconnected {
                            Button(text("conn_action_disconnect")) { store.disconnect() }.controlSize(.small)
                        }
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
                        if store.hasAir5Audio {
                            Picker("Overview section", selection: $section) {
                                ForEach(OverviewSection.allCases) { item in Text(item.rawValue).tag(item) }
                            }.pickerStyle(.segmented).labelsHidden()
                        }
                        if section == .sound, store.canControlNoise, let model = store.model, !model.selectableModes.isEmpty {
                            OrganizedNoiseControls(store: store)
                        }
                        if store.hasAir5Audio { AudioControlsView(store: store, section: section) }
                        DisclosureGroup("Details") {
                            VStack(alignment: .leading, spacing: 10) {
                                LabeledContent(text("row_firmware_title"), value: store.firmware)
                                DisclosureGroup("Battery diagnostics") {
                                    Text(store.batteryDiagnostic).font(.caption.monospaced()).textSelection(.enabled)
                                }
                            }.font(.caption).padding(.top, 8)
                        }.font(.callout).foregroundStyle(.secondary)
                    } else {
                        Text(store.devices.isEmpty ? text("no_buds_paired") : store.nextReconnect != nil
                             ? "Waiting to reconnect to your earbuds…" : "Select your earbuds and choose Connect.")
                            .foregroundStyle(.secondary)
                    }
                    if store.connection == .disconnected, let device = store.devices.first(where: { $0.id == selectedDevice }) {
                        Button(text("conn_action_connect")) { store.connect(device) }
                            .buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
                    }
                }
                .padding(20)
                .frame(maxWidth: 800, alignment: .leading)
                .frame(maxWidth: .infinity, alignment: .leading)
                .animation(reduceMotion ? nil : .easeInOut(duration: 0.25), value: sidebarVisibility)
            }
            .frame(minWidth: 420)
            .navigationTitle(text("desktop_nav_overview"))
        }
        .navigationSplitViewStyle(.balanced)
        .onAppear { if selectedDevice == nil { selectedDevice = store.devices.first?.id } }
        .onChange(of: store.devices.map(\.id)) { _, ids in
            if selectedDevice == nil || !ids.contains(selectedDevice!) { selectedDevice = ids.first }
        }
    }

    private var batterySection: some View {
        GroupBox {
            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    Text(text("problem_cat_battery")).font(.subheadline.weight(.semibold))
                    Spacer()
                    Button("Refresh battery", systemImage: "arrow.clockwise") { store.refreshBattery() }
                        .labelStyle(.iconOnly).buttonStyle(.borderless).help("Refresh battery")
                }
                HStack(spacing: 12) {
                    batteryCell(1, "status_left", "earbuds")
                    batteryCell(3, "status_case", "earbuds.case")
                    batteryCell(2, "status_right", "earbuds")
                }
                Text(store.battery.contains(where: { $0.part == 3 })
                     ? "Case battery shows its last reported level."
                     : "Open the case lid and refresh to request its battery level.")
                    .font(.caption).foregroundStyle(.secondary)
            }.padding(8)
        }
    }

    private func batteryCell(_ part: UInt8, _ key: String, _ symbol: String) -> some View {
        let cell = store.battery.first { $0.part == part }
        return HStack(spacing: 8) {
            ZStack {
                Circle().stroke(Color.secondary.opacity(0.18), lineWidth: 4)
                if let cell {
                    Circle().trim(from: 0, to: CGFloat(cell.level) / 100)
                        .stroke(Color.accentColor, style: StrokeStyle(lineWidth: 4, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                }
                Image(systemName: symbol).font(.system(size: 21)).foregroundStyle(.primary)
            }.frame(width: 46, height: 46)
            VStack(alignment: .leading, spacing: 3) {
                Text(text(key)).font(.caption).foregroundStyle(.secondary)
                HStack(spacing: 4) {
                    Text(cell.map { "\($0.level)%" } ?? "—").font(.body.monospacedDigit().weight(.semibold))
                    BatteryIcon(cell: cell).font(.caption2)
                }
                if store.cachedBattery.contains(part) {
                    Text("Last known").font(.caption2).foregroundStyle(.secondary)
                }
                if let updated = store.batteryUpdated[part] {
                    Text(updated.formatted(date: .omitted, time: .shortened))
                        .font(.caption2).foregroundStyle(.secondary)
                        .help("Last received: " + updated.formatted(date: .abbreviated, time: .standard))
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .center)
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
            if store.connection != .connected {
                HStack(spacing: 6) {
                    if store.connection == .connecting { ProgressView().controlSize(.small) }
                    Text(store.status).font(.caption).foregroundStyle(.secondary)
                    if let date = store.nextReconnect {
                        Text("Retry in").font(.caption).foregroundStyle(.secondary)
                        Text(date, style: .timer).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                    }
                }
                if store.connection == .disconnected {
                    Button(text("conn_action_connect")) { store.connectPreferred() }
                } else {
                    Button("Cancel connection") { store.disconnect() }
                }
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

                        if store.cachedBattery.contains(part) {
                            Text("Cached").font(.caption2).foregroundStyle(.secondary)
                        }
                    }.frame(maxWidth: .infinity)
                    .help(store.batteryUpdated[part].map { "Last received: " + $0.formatted(date: .omitted, time: .standard) } ?? "Not reported")
                }
                }
                if store.canControlNoise {
                    Divider()
                    NoiseControlPanel(store: store, compact: true)
                }
            }
            if store.connection == .connected {
                if store.canEQ {
                    Menu {
                        ForEach(store.model?.eqPresets ?? []) { preset in
                            Button { store.selectEQ(preset.slot) } label: {
                                if store.eqSelection == preset.slot { Label(text(preset.titleKey), systemImage: "checkmark") }
                                else { Text(text(preset.titleKey)) }
                            }
                        }
                    } label: { Label("Sound Master EQ", systemImage: "slider.horizontal.3") }
                    .disabled(store.audioBusy)
                }
                HStack {
                    Button { store.refreshBattery() } label: { Label("Refresh", systemImage: "arrow.clockwise") }
                    Spacer()
                    Button(text("conn_action_disconnect")) { store.disconnect() }
                }.font(.callout)
            }
            if let error = store.error {
                HStack(alignment: .top) {
                    Text(error).font(.caption).foregroundStyle(.orange)
                    Spacer(minLength: 2)
                    Button { store.error = nil } label: { Image(systemName: "xmark") }
                        .buttonStyle(.plain).accessibilityLabel(text("dialog_close"))
                }
            }
            Divider()
            VStack(spacing: 2) {
                Button {
                    openWindow(id: "main")
                    NSApp.activate(ignoringOtherApps: true)
                } label: {
                    trayFooterLabel(text("desktop_nav_overview"), symbol: "square.grid.2x2")
                }
                Button { NSApp.terminate(nil) } label: {
                    trayFooterLabel("Quit QuickBuds", symbol: "power", shortcut: "⌘Q")
                }.keyboardShortcut("q")
            }
            .buttonStyle(.plain)
        }
        .padding(10).frame(width: 280, alignment: .leading)
        .fixedSize(horizontal: false, vertical: true)
        .background(.ultraThinMaterial)
    }

    private func trayFooterLabel(_ title: String, symbol: String, shortcut: String? = nil) -> some View {
        HStack(spacing: 8) {
            Image(systemName: symbol).frame(width: 18).foregroundStyle(.secondary)
            Text(title)
            Spacer(minLength: 8)
            if let shortcut { Text(shortcut).font(.caption).foregroundStyle(.secondary) }
        }
        .font(.callout)
        .frame(maxWidth: .infinity, minHeight: 28, alignment: .leading)
        .padding(.horizontal, 8)
        .contentShape(Rectangle())
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


private struct OrganizedNoiseControls: View {
    @ObservedObject var store: BudsStore
    private var modes: [NoiseMode] { store.model?.selectableModes ?? [] }
    private var levels: [NoiseMode] { modes.filter { [3, 4, 5, 7, 8].contains($0.mode) } }
    private var ancSelected: Bool { [3, 4, 5, 7, 8].contains(store.noiseMode ?? -1) }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(text("anc_section")).font(.headline)
                Spacer()
                if store.applying { Text(text("feature_applying")).font(.caption).foregroundStyle(.secondary) }
            }
            HStack(spacing: 4) {
                if let off = modes.first(where: { $0.mode == 1 }) {
                    modeButton(off, title: text(off.titleKey), icon: "speaker.slash", selected: store.noiseMode == 1)
                }
                if let anc = levels.first(where: { $0.mode == 7 }) ?? levels.first {
                    modeButton(anc, title: text("anc_seg_anc"), icon: "headphones", selected: ancSelected)
                }
                if let transparency = modes.first(where: { $0.mode == 2 }) {
                    modeButton(transparency, title: text(transparency.titleKey), icon: "person.wave.2", selected: store.noiseMode == 2)
                }
            }
            if !levels.isEmpty {
                Divider()
                HStack(spacing: 6) {
                    ForEach(levels) { mode in
                        Button { store.setNoise(mode) } label: {
                            HStack(spacing: 4) {
                                if store.noiseMode == mode.mode { Image(systemName: "checkmark") }
                                Text(text(mode.titleKey))
                            }.frame(maxWidth: .infinity).padding(.vertical, 7)
                        }
                        .buttonStyle(.bordered)
                        .disabled(store.applying)
                        .accessibilityAddTraits(store.noiseMode == mode.mode ? .isSelected : [])
                    }
                }
            }
        }
        .padding(16)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16))
    }

    private func modeButton(_ mode: NoiseMode, title: String, icon: String, selected: Bool) -> some View {
        Button { store.setNoise(mode) } label: {
            VStack(spacing: 6) {
                Image(systemName: icon).font(.title3)
                Text(title).font(.callout)
            }
            .frame(maxWidth: .infinity).padding(.vertical, 12)
            .background(selected ? Color.accentColor.opacity(0.16) : .clear, in: Capsule())
            .contentShape(Capsule())
        }
        .buttonStyle(.plain).disabled(store.applying)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}
