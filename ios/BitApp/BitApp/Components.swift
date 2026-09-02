import SwiftUI

/// N switches whose active bits sum to a value — the exact demonstration
/// from spec §3 Round 0 ("128 64 32 16 8 4 2 1"). Shared by Play, every
/// circuit Build, Lab's binary playground, and Explore's circuit-flavored
/// concept demos, so the same visual vocabulary carries everywhere.
struct SwitchesView: View {
    let state: FfiCircuitState
    /// Two's Complement's demo wants the signed reading alongside the
    /// plain one; Binary's demo doesn't need it cluttering the screen.
    var showSignedValue: Bool = false
    /// Hexadecimal's demo groups the same 8 switches into two nibbles and
    /// shows the hex digit under each — same widget, different framing.
    var showHexBreakdown: Bool = false
    let onToggle: (UInt16) -> Void

    @State private var valueBounce = false

    private var bitWidth: Int { state.switches.count }

    private func placeValue(at index: Int) -> Int {
        1 << (bitWidth - 1 - index)
    }

    var body: some View {
        VStack(spacing: 12) {
            HStack(spacing: 8) {
                ForEach(Array(state.switches.enumerated()), id: \.offset) { index, isOn in
                    FlipToggleCell(
                        isOn: isOn,
                        caption: "\(placeValue(at: index))",
                        accessibilityId: "switch-\(index)",
                        accessibilityDescription: "\(placeValue(at: index))s place"
                    ) {
                        onToggle(UInt16(index))
                    }
                    // Nibble boundary marker for the hex-breakdown variant —
                    // purely a visual grouping cue between the two nibbles.
                    .padding(.trailing, (showHexBreakdown && bitWidth == 8 && index == 3) ? 10 : 0)
                }
            }

            Text("\(state.value)")
                .font(.system(size: 44, weight: .bold, design: .monospaced))
                .contentTransition(.numericText())
                .scaleEffect(valueBounce ? 1.15 : 1.0)
                .animation(.spring(response: 0.35, dampingFraction: 0.55), value: state.value)
                .onChange(of: state.value) { _, _ in bounceValue() }

            if showSignedValue {
                Text("signed: \(state.signedValue)")
                    .font(.title3.monospaced())
                    .foregroundStyle(.secondary)
                    .contentTransition(.numericText())
                    .animation(.default, value: state.signedValue)
            }

            if showHexBreakdown && bitWidth == 8 {
                let hex = String(format: "%02X", state.value)
                Text("hex: \(hex.prefix(1))  \(hex.suffix(1))")
                    .font(.title3.monospaced())
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func bounceValue() {
        withAnimation(.spring(response: 0.15, dampingFraction: 0.4)) { valueBounce = true }
        withAnimation(.spring(response: 0.25, dampingFraction: 0.6).delay(0.15)) { valueBounce = false }
    }
}

/// A grid of on/off pixels — each pixel is literally a bit. Shared by the
/// Pixel Builder build and Lab's pixel playground.
struct PixelGridView: View {
    let state: FfiPixelState
    let onToggle: (UInt16, UInt16) -> Void

    var body: some View {
        VStack(spacing: 2) {
            ForEach(0..<Int(state.height), id: \.self) { y in
                HStack(spacing: 2) {
                    ForEach(0..<Int(state.width), id: \.self) { x in
                        let isOn = state.pixels[y * Int(state.width) + x]
                        FlipPixelCell(isOn: isOn, x: x, y: y) {
                            onToggle(UInt16(x), UInt16(y))
                        }
                    }
                }
            }
        }
    }
}

/// One flippable digital switch — the physical-flip counterpart to a
/// literal on/off state. Rotation angle increments by 180° per tap rather
/// than toggling between two fixed angles, so repeated taps keep
/// spinning forward instead of snapping back; a 0/1 digit reads correctly
/// either way round (both are near-symmetric glyphs), so no separate
/// front/back content swap is needed to avoid mirrored text.
struct FlipToggleCell: View {
    let isOn: Bool
    var caption: String? = nil
    let accessibilityId: String
    let accessibilityDescription: String
    var size: CGSize = CGSize(width: 32, height: 48)
    let action: () -> Void

    @State private var flips = 0

    var body: some View {
        VStack(spacing: 4) {
            if let caption {
                Text(caption)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Button {
                Haptics.toggle()
                flips += 1
                action()
            } label: {
                RoundedRectangle(cornerRadius: 8)
                    .fill(isOn ? Color.accentColor : Color.gray.opacity(0.25))
                    .frame(width: size.width, height: size.height)
                    .overlay(
                        Text(isOn ? "1" : "0")
                            .fontWeight(.bold)
                            .foregroundStyle(isOn ? .white : .primary)
                    )
            }
            .buttonStyle(.plain)
            .rotation3DEffect(.degrees(Double(flips) * 180), axis: (x: 0, y: 1, z: 0), perspective: 0.35)
            .animation(.spring(response: 0.4, dampingFraction: 0.65), value: flips)
            .accessibilityIdentifier(accessibilityId)
            .accessibilityLabel("\(accessibilityDescription), \(isOn ? "on" : "off")")
        }
    }
}

/// A pixel doesn't carry text, so a flip never risks mirrored content —
/// same spinning-forward mechanic as `FlipToggleCell`, just on a plain
/// color swatch.
private struct FlipPixelCell: View {
    let isOn: Bool
    let x: Int
    let y: Int
    let action: () -> Void

    @State private var flips = 0

    var body: some View {
        Button {
            Haptics.toggle()
            flips += 1
            action()
        } label: {
            Rectangle()
                .fill(isOn ? Color.primary : Color.gray.opacity(0.15))
                .frame(width: 36, height: 36)
        }
        .buttonStyle(.plain)
        .rotation3DEffect(.degrees(Double(flips) * 180), axis: (x: 0, y: 1, z: 0), perspective: 0.35)
        .animation(.spring(response: 0.4, dampingFraction: 0.65), value: flips)
        .accessibilityIdentifier("pixel-\(x)-\(y)")
        .accessibilityLabel("pixel row \(y), column \(x), \(isOn ? "on" : "off")")
    }
}

/// A row of tappable, byte-valued slots — the Data Structures counterpart
/// to `SwitchesView`. Unlike a switch (two states, so a tap is enough),
/// an array slot holds an arbitrary 0-255 value, so tapping selects the
/// slot and a small numeric field underneath is how the value is set.
/// Shared by Arrays' Explore demo, the Array Builder build, and Lab's
/// array playground.
struct ArraySlotsView: View {
    let state: FfiArrayState
    let onSet: (UInt16, UInt8) -> Void

    @State private var selectedIndex: Int?
    @State private var inputText: String = ""
    @FocusState private var inputFocused: Bool

    var body: some View {
        VStack(spacing: 16) {
            HStack(spacing: 8) {
                ForEach(Array(state.slots.enumerated()), id: \.offset) { index, value in
                    let filled = state.filled[index]
                    Button {
                        Haptics.toggle()
                        selectedIndex = index
                        inputText = filled ? "\(value)" : ""
                        inputFocused = true
                    } label: {
                        VStack(spacing: 2) {
                            Text("\(index)").font(.caption2).foregroundStyle(.secondary)
                            Text(filled ? "\(value)" : "–")
                                .font(.system(.body, design: .monospaced))
                                .fontWeight(.bold)
                        }
                        .frame(width: 40, height: 52)
                        .background(
                            RoundedRectangle(cornerRadius: 8)
                                .fill(filled ? Color.accentColor.opacity(0.2) : Color.gray.opacity(0.15))
                        )
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(selectedIndex == index ? Color.accentColor : .clear, lineWidth: 2)
                        )
                    }
                    .buttonStyle(.plain)
                    .accessibilityIdentifier("array-slot-\(index)")
                    .accessibilityLabel("slot \(index), \(filled ? "value \(value)" : "empty")")
                }
            }

            if let selectedIndex {
                HStack(spacing: 8) {
                    Text("index \(selectedIndex):").font(.subheadline).foregroundStyle(.secondary)
                    TextField("0-255", text: $inputText)
                        .keyboardType(.numberPad)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 90)
                        .focused($inputFocused)
                        .accessibilityIdentifier("array-value-input")
                        .onSubmit { commit(selectedIndex) }
                    Button("Set") { commit(selectedIndex) }
                        .buttonStyle(.squishy)
                        .disabled(UInt8(inputText) == nil)
                        .accessibilityIdentifier("array-set-button")
                }
                .transition(.opacity)
            } else {
                Text("Tap a slot to set its value")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .animation(.default, value: selectedIndex)
    }

    private func commit(_ index: Int) {
        guard let value = UInt8(inputText) else { return }
        inputFocused = false
        onSet(UInt16(index), value)
    }
}

/// The app's one shared "primary action" button style — a tinted capsule
/// that squishes down on press. Used anywhere a `.borderedProminent`
/// button used to sit alone (Play's Check button, empty-state CTAs, the
/// new-topic suggestion) so every main tap in the app shares the same
/// tactile feel. Reads `Color.accentColor`, which resolves from whatever
/// `.tint()` is active in the environment — domain/tier theming still
/// flows through exactly like `.borderedProminent` did.
struct SquishyButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.headline)
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .foregroundStyle(.white)
            .background(Color.accentColor.opacity(isEnabled ? (configuration.isPressed ? 0.8 : 1) : 0.35))
            .clipShape(Capsule())
            .scaleEffect(configuration.isPressed ? 0.95 : 1)
            .animation(.spring(response: 0.25, dampingFraction: 0.6), value: configuration.isPressed)
    }
}

extension ButtonStyle where Self == SquishyButtonStyle {
    static var squishy: SquishyButtonStyle { SquishyButtonStyle() }
}

/// A one-time achievement — `id` matches whatever string was passed to
/// `BitEngine::mark_celebrated`, so a caller both shows and permanently
/// silences the same milestone. No score or comparison to anyone else,
/// same as everything else in Profile — just "this happened."
struct MilestoneInfo: Identifiable {
    let id: String
    let icon: String
    let title: String
    let message: String
    let color: Color
}

/// Shared celebration sheet for every milestone in the app (first
/// concept mastered, 100 correct answers, first build completed, ...) —
/// one widget, driven entirely by `MilestoneInfo`, rather than a bespoke
/// sheet per achievement.
struct MilestoneCelebrationSheet: View {
    let milestone: MilestoneInfo
    let onDismiss: () -> Void

    @State private var didCelebrate = false

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: milestone.icon)
                .font(.system(size: 64))
                .foregroundStyle(milestone.color)
                .symbolEffect(.bounce, value: didCelebrate)

            Text(milestone.title)
                .font(.title2.bold())
                .multilineTextAlignment(.center)
            Text(milestone.message)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            Button("Nice!", action: onDismiss)
                .buttonStyle(.squishy)
                .tint(milestone.color)
                .accessibilityIdentifier("milestone-dismiss")
        }
        .padding()
        .presentationDetents([.medium])
        .onAppear {
            Haptics.success()
            didCelebrate = true
        }
    }
}

struct ObjectiveBanner: View {
    let status: FfiObjectiveStatus
    @State private var didCelebrate = false

    var body: some View {
        Group {
            if status == .met {
                Label("Objective met!", systemImage: "checkmark.seal.fill")
                    .font(.title3.bold())
                    .foregroundStyle(.green)
                    .symbolEffect(.bounce, value: didCelebrate)
                    .transition(.scale.combined(with: .opacity))
            }
        }
        .onChange(of: status, initial: true) { _, newStatus in
            guard newStatus == .met, !didCelebrate else { return }
            didCelebrate = true
            Haptics.success()
        }
    }
}
