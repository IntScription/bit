import SwiftUI

/// No XP, no timers, no lives, no objective chrome — ever (see spec §2's
/// LAB section). This reuses the exact same build sessions as the Build
/// tab (`binary_counter`, `pixel_builder`); the only difference is that
/// this screen simply never reads or displays `objectiveStatus`.
struct LabView: View {
    @State private var mode: Mode = .binary

    enum Mode: CaseIterable {
        case binary, pixel, array, gate

        var title: String {
            switch self {
            case .binary: return "Binary"
            case .pixel: return "Pixels"
            case .array: return "Array"
            case .gate: return "Gates"
            }
        }

        var icon: String {
            switch self {
            case .binary: return "circle.grid.2x2.fill"
            case .pixel: return "square.grid.3x3.fill"
            case .array: return "square.stack.3d.up.fill"
            case .gate: return "cpu.fill"
            }
        }

        // Fixed, hand-picked colors rather than `Theme.color(forDomain:)`
        // — Lab isn't tied to any one concept, so each playground gets
        // its own stable identity instead of borrowing a domain's.
        var color: Color {
            switch self {
            case .binary: return .blue
            case .pixel: return .purple
            case .array: return .teal
            case .gate: return .orange
            }
        }

        /// What this playground is for — Lab has no objective banner or
        /// instructions anywhere else, so without this each mode is just
        /// a bare widget with no explanation of what you're looking at.
        var caption: String {
            switch self {
            case .binary: return "Toggle switches freely and watch the decimal value update live. No target, no objective — just a register to play with."
            case .pixel: return "Toggle any pixel on a free-form grid. Same mechanic as an array of switches, arranged in two dimensions instead of one row."
            case .array: return "Tap a slot, type a value, set it. A byte-wide memory cell you can write to and read back at any index."
            case .gate: return "Pick a gate, flip its inputs, watch the output respond instantly. The same AND/OR/XOR/NOT you met in Foundations, live."
            }
        }
    }

    var body: some View {
        NavigationStack {
            VStack {
                HStack(alignment: .top, spacing: 10) {
                    Image(systemName: "flask.fill")
                        .foregroundStyle(Theme.brand)
                    Text("No score, no objective, nothing to get wrong — just the raw mechanics to poke at freely.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                .padding(12)
                .background(RoundedRectangle(cornerRadius: 14).fill(.thinMaterial))
                .padding(.horizontal)
                .padding(.top, 8)

                Picker("Playground", selection: $mode) {
                    ForEach(Mode.allCases, id: \.self) { m in
                        Label(m.title, systemImage: m.icon).tag(m)
                    }
                }
                .pickerStyle(.segmented)
                .padding()

                Text(mode.caption)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 24)
                    .id(mode)
                    .transition(.opacity)
                    .animation(.easeInOut(duration: 0.2), value: mode)

                Group {
                    switch mode {
                    case .binary: LabCircuitView()
                    case .pixel: LabPixelView()
                    case .array: LabArrayView()
                    case .gate: GateDemoView(initialGate: .and)
                    }
                }
                .padding(.vertical, 20)
                .frame(maxWidth: .infinity)
                .background(
                    RoundedRectangle(cornerRadius: 20)
                        .fill(mode.color.opacity(0.08))
                )
                .padding(.horizontal)
                .padding(.top, 12)
                .tint(mode.color)
                .id(mode)
                .transition(.opacity.combined(with: .scale(scale: 0.97)))
                .animation(.spring(response: 0.35, dampingFraction: 0.75), value: mode)

                Spacer()
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .gameBackground()
            .navigationTitle("Lab")
            .navigationBarTitleDisplayMode(.inline)
        }
    }
}

private struct LabCircuitView: View {
    @EnvironmentObject var engine: Engine
    @State private var sessionId: UInt64?
    @State private var state: FfiCircuitState?

    var body: some View {
        Group {
            if let state {
                SwitchesView(state: state) { index in
                    guard let sessionId else { return }
                    _ = try? engine.ffi.applyCircuitAction(sessionId: sessionId, action: .toggle(index: index))
                    self.state = try? engine.ffi.circuitState(sessionId: sessionId)
                }
            } else {
                ProgressView()
            }
        }
        .onAppear {
            guard sessionId == nil, let id = try? engine.ffi.startBuild(buildId: "binary_counter") else { return }
            sessionId = id
            state = try? engine.ffi.circuitState(sessionId: id)
        }
        .onDisappear {
            if let sessionId { engine.ffi.endBuild(sessionId: sessionId) }
        }
    }
}

private struct LabPixelView: View {
    @EnvironmentObject var engine: Engine
    @State private var sessionId: UInt64?
    @State private var state: FfiPixelState?

    var body: some View {
        Group {
            if let state {
                PixelGridView(state: state) { x, y in
                    guard let sessionId else { return }
                    _ = try? engine.ffi.applyPixelAction(sessionId: sessionId, action: .toggle(x: x, y: y))
                    self.state = try? engine.ffi.pixelState(sessionId: sessionId)
                }
            } else {
                ProgressView()
            }
        }
        .onAppear {
            guard sessionId == nil, let id = try? engine.ffi.startBuild(buildId: "pixel_builder") else { return }
            sessionId = id
            state = try? engine.ffi.pixelState(sessionId: id)
        }
        .onDisappear {
            if let sessionId { engine.ffi.endBuild(sessionId: sessionId) }
        }
    }
}

private struct LabArrayView: View {
    @EnvironmentObject var engine: Engine
    @State private var sessionId: UInt64?
    @State private var state: FfiArrayState?

    var body: some View {
        Group {
            if let state {
                ArraySlotsView(state: state) { index, value in
                    guard let sessionId else { return }
                    _ = try? engine.ffi.applyArrayAction(sessionId: sessionId, action: .set(index: index, value: value))
                    self.state = try? engine.ffi.arrayState(sessionId: sessionId)
                }
            } else {
                ProgressView()
            }
        }
        .onAppear {
            guard sessionId == nil, let id = try? engine.ffi.startBuild(buildId: "array_builder") else { return }
            sessionId = id
            state = try? engine.ffi.arrayState(sessionId: id)
        }
        .onDisappear {
            if let sessionId { engine.ffi.endBuild(sessionId: sessionId) }
        }
    }
}
