import SwiftUI

/// Formats a byte as 8-bit binary with leading zeros — shared by every
/// demo that shows raw bytes (UTF-8 breakdown, XOR checksum).
func bin8(_ b: UInt8) -> String {
    var s = String(b, radix: 2)
    while s.count < 8 { s = "0" + s }
    return s
}

/// Decoded from `concept_detail_json`'s `interactive_demo` field — see
/// bit-knowledge's `InteractiveDemo`. `DemoContainerView` below maps
/// `id` to one of the widgets in this file.
struct InteractiveDemoInfo: Decodable {
    let id: String
    let params: [String: String]
}

/// Dispatches a concept's `interactive_demo` id to a real, live widget —
/// every id referenced anywhere in `/content` resolves to something
/// actually interactive here, not a placeholder. See spec's core
/// philosophy: INTERACT before EXPLAIN, never the reverse.
struct DemoContainerView: View {
    let demo: InteractiveDemoInfo

    var body: some View {
        VStack {
            switch demo.id {
            case "eight_switches", "single_switch", "shift_register":
                CircuitDemoView(bitWidth: bitWidth)
            case "signed_switches":
                CircuitDemoView(bitWidth: bitWidth, showSignedValue: true)
            case "nibble_grouping":
                CircuitDemoView(bitWidth: 8, showHexBreakdown: true)
            case "place_value_comparison":
                CircuitDemoView(bitWidth: bitWidth)
            case "logic_gate":
                GateDemoView(initialGate: gateFromParams)
            case "truth_table":
                GateDemoView(initialGate: .and)
            case "ascii_table":
                AsciiTableView()
            case "utf8_byte_breakdown":
                Utf8BreakdownView()
            case "xor_checksum_demo":
                XorChecksumDemoView()
            case "array_slots":
                ArrayDemoView(capacity: arrayCapacity)
            default:
                EmptyView()
            }
        }
        .padding(.vertical, 8)
    }

    private var bitWidth: UInt8 {
        UInt8(demo.params["bit_width"] ?? "") ?? 8
    }

    private var arrayCapacity: UInt8 {
        UInt8(demo.params["capacity"] ?? "") ?? 6
    }

    private var gateFromParams: FfiGateKind {
        switch demo.params["gate"] {
        case "or": return .or
        case "xor": return .xor
        case "not": return .not
        default: return .and
        }
    }
}

/// Wraps `SwitchesView` with a live FFI session — used for every
/// circuit-flavored demo (Bits, Binary, Bytes, Bit Shifting, Two's
/// Complement, Hexadecimal, Decimal). Not tied to any content Build; see
/// bit-ffi's `start_circuit_demo` doc comment for why this bypasses the
/// Build catalog entirely.
struct CircuitDemoView: View {
    @EnvironmentObject var engine: Engine
    let bitWidth: UInt8
    var showSignedValue: Bool = false
    var showHexBreakdown: Bool = false

    @State private var sessionId: UInt64?
    @State private var state: FfiCircuitState?

    var body: some View {
        Group {
            if let state {
                SwitchesView(state: state, showSignedValue: showSignedValue, showHexBreakdown: showHexBreakdown) { index in
                    guard let sessionId else { return }
                    _ = try? engine.ffi.applyCircuitAction(sessionId: sessionId, action: .toggle(index: index))
                    self.state = try? engine.ffi.circuitState(sessionId: sessionId)
                }
            } else {
                ProgressView()
            }
        }
        .onAppear {
            guard sessionId == nil else { return }
            let id = engine.ffi.startCircuitDemo(bitWidth: bitWidth)
            sessionId = id
            state = try? engine.ffi.circuitState(sessionId: id)
        }
        .onDisappear {
            if let sessionId { engine.ffi.endBuild(sessionId: sessionId) }
        }
    }
}

/// A single configurable logic gate — AND/OR/XOR/NOT — with a picker so
/// one widget serves every gate concept's demo, plus Boolean Logic's
/// "truth_table" demo (which just starts on AND and lets the player pick
/// any of the four).
struct GateDemoView: View {
    @EnvironmentObject var engine: Engine
    let initialGate: FfiGateKind

    @State private var gate: FfiGateKind
    @State private var sessionId: UInt64?
    @State private var state: FfiGateState?
    @State private var outputPop = false

    init(initialGate: FfiGateKind) {
        self.initialGate = initialGate
        _gate = State(initialValue: initialGate)
    }

    var body: some View {
        VStack(spacing: 16) {
            Picker("Gate", selection: $gate) {
                Text("AND").tag(FfiGateKind.and)
                Text("OR").tag(FfiGateKind.or)
                Text("XOR").tag(FfiGateKind.xor)
                Text("NOT").tag(FfiGateKind.not)
            }
            .pickerStyle(.segmented)

            if let state {
                HStack(spacing: 16) {
                    ForEach(Array(state.inputs.enumerated()), id: \.offset) { index, isOn in
                        FlipToggleCell(
                            isOn: isOn,
                            accessibilityId: "gate-input-\(index)",
                            accessibilityDescription: "input \(index + 1)",
                            size: CGSize(width: 44, height: 44)
                        ) {
                            guard let sessionId else { return }
                            _ = try? engine.ffi.applyGateAction(sessionId: sessionId, action: .toggle(index: UInt16(index)))
                            refresh()
                        }
                    }
                    Image(systemName: "arrow.right")
                        .foregroundStyle(.secondary)
                    RoundedRectangle(cornerRadius: 8)
                        .fill(state.output ? Color.green : Color.gray.opacity(0.25))
                        .frame(width: 44, height: 44)
                        .overlay(Text(state.output ? "1" : "0").fontWeight(.bold).foregroundStyle(state.output ? .white : .primary))
                        .scaleEffect(outputPop ? 1.2 : 1.0)
                        .animation(.spring(response: 0.25, dampingFraction: 0.5), value: outputPop)
                        // A plain Shape isn't a Button, so it needs an
                        // explicit accessibility element (unlike the
                        // input buttons above, which get one for free) —
                        // otherwise the identifier/label below tag
                        // nothing queryable.
                        .accessibilityElement(children: .ignore)
                        .accessibilityIdentifier("gate-output")
                        .accessibilityLabel("output, \(state.output ? "on" : "off")")
                        .onChange(of: state.output) { _, _ in
                            outputPop = true
                            DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) { outputPop = false }
                        }
                }
            } else {
                ProgressView()
            }
        }
        .onAppear { start(for: gate) }
        .onChange(of: gate) { _, newGate in start(for: newGate) }
    }

    private func start(for gate: FfiGateKind) {
        let inputCount: UInt8 = gate == .not ? 1 : 2
        sessionId = engine.ffi.startGateDemo(gate: gate, inputCount: inputCount)
        refresh()
    }

    private func refresh() {
        guard let sessionId else { return }
        state = try? engine.ffi.gateState(sessionId: sessionId)
    }
}

/// Wraps `ArraySlotsView` with a live FFI session — the Arrays concept's
/// demo. Not tied to any content Build; mirrors `CircuitDemoView`'s use
/// of `start_circuit_demo` for the same reason (see that type's doc
/// comment).
struct ArrayDemoView: View {
    @EnvironmentObject var engine: Engine
    let capacity: UInt8

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
            guard sessionId == nil else { return }
            let id = engine.ffi.startArrayDemo(capacity: capacity)
            sessionId = id
            state = try? engine.ffi.arrayState(sessionId: id)
        }
        .onDisappear {
            if let sessionId { engine.ffi.endBuild(sessionId: sessionId) }
        }
    }
}

/// Static reference table, code <-> character — no simulation needed, so
/// this is pure local SwiftUI state, not FFI-backed like the others.
struct AsciiTableView: View {
    @State private var selected: Int = 65 // 'A'
    private let codes = Array(33...126)
    private let columns = Array(repeating: GridItem(.fixed(36), spacing: 4), count: 10)

    var body: some View {
        VStack(spacing: 12) {
            let scalar = UnicodeScalar(UInt8(selected))
            Text("\(selected)")
                .font(.system(size: 40, weight: .bold, design: .monospaced))
                .contentTransition(.numericText())
                .animation(.default, value: selected)
            Text("'\(Character(scalar))'").font(.title2).foregroundStyle(.secondary)

            LazyVGrid(columns: columns, spacing: 4) {
                ForEach(codes, id: \.self) { code in
                    Button {
                        Haptics.toggle()
                        selected = code
                    } label: {
                        Text(String(UnicodeScalar(UInt8(code))))
                            .font(.caption)
                            .frame(width: 32, height: 32)
                            .background(selected == code ? Color.accentColor : Color.gray.opacity(0.15))
                            .foregroundStyle(selected == code ? .white : .primary)
                            .clipShape(RoundedRectangle(cornerRadius: 6))
                    }
                    .buttonStyle(.plain)
                    .accessibilityIdentifier("ascii-\(code)")
                    .accessibilityLabel("ASCII \(code), character \(String(UnicodeScalar(UInt8(code))))")
                }
            }
        }
    }
}

/// Type any single character, see its real UTF-8 byte breakdown —
/// computed by Swift's own UTF-8 encoder (`String.utf8`), not
/// reimplemented by hand, so it's exactly as correct as the platform's.
struct Utf8BreakdownView: View {
    @State private var text: String = "A"

    var body: some View {
        VStack(spacing: 12) {
            TextField("Type a character", text: $text)
                .textFieldStyle(.roundedBorder)
                .frame(maxWidth: 160)
                .multilineTextAlignment(.center)
                .accessibilityIdentifier("utf8-input")
                .onChange(of: text) { _, newValue in
                    if let last = newValue.unicodeScalars.last, newValue.unicodeScalars.count > 1 {
                        text = String(last)
                    }
                }

            if let scalar = text.unicodeScalars.first {
                let bytes = Array(String(scalar).utf8)
                Text("U+\(String(format: "%04X", scalar.value))  →  \(bytes.count) byte\(bytes.count == 1 ? "" : "s")")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                HStack(spacing: 8) {
                    ForEach(Array(bytes.enumerated()), id: \.offset) { _, byte in
                        Text(bin8(byte))
                            .font(.system(.body, design: .monospaced))
                            .padding(8)
                            .background(Color.gray.opacity(0.15))
                            .clipShape(RoundedRectangle(cornerRadius: 6))
                    }
                }
            } else {
                Text("Type a character above").foregroundStyle(.secondary)
            }
        }
    }
}

/// Toggle bits across a few bytes, watch the running XOR checksum update
/// live — local-only state (no live simulation needed for a running XOR,
/// it's cheap to compute directly in Swift, matching how the concept
/// itself is explained: XOR every byte together).
struct XorChecksumDemoView: View {
    @State private var bytes: [[Bool]] = Array(repeating: Array(repeating: false, count: 8), count: 3)

    private func value(_ row: [Bool]) -> UInt8 {
        var v: UInt8 = 0
        for (i, on) in row.enumerated() where on { v |= 1 << (7 - i) }
        return v
    }

    private var checksum: UInt8 {
        bytes.map(value).reduce(0, ^)
    }

    var body: some View {
        VStack(spacing: 12) {
            ForEach(0..<bytes.count, id: \.self) { row in
                HStack(spacing: 6) {
                    ForEach(0..<8, id: \.self) { col in
                        let isOn = bytes[row][col]
                        Button {
                            Haptics.toggle()
                            bytes[row][col].toggle()
                        } label: {
                            RoundedRectangle(cornerRadius: 6)
                                .fill(isOn ? Color.accentColor : Color.gray.opacity(0.25))
                                .frame(width: 24, height: 32)
                                .overlay(Text(isOn ? "1" : "0").font(.caption2).fontWeight(.bold).foregroundStyle(isOn ? .white : .primary))
                        }
                        .buttonStyle(.plain)
                        .accessibilityIdentifier("checksum-byte-\(row)-bit-\(col)")
                    }
                    Text("= \(value(bytes[row]))")
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                        .frame(width: 40, alignment: .leading)
                }
            }
            Divider()
            HStack(spacing: 8) {
                Text("XOR:")
                Text(bin8(checksum))
                    .font(.title3.monospaced())
                    .fontWeight(.bold)
                    .contentTransition(.numericText())
                Text("(\(checksum))")
                    .foregroundStyle(.secondary)
            }
            .animation(.default, value: checksum)
        }
    }
}
