import SwiftUI

/// The app's opening beat, shown once per launch: eight bits flip on in
/// sequence — the exact demonstration Foundations opens with (spec §3
/// Round 0) — then the whole thing steps aside for the real app. ~1.3s
/// total, never longer; it's a flourish, not a loading screen (there's
/// nothing to actually wait for — content loads synchronously in
/// `Engine.init`).
struct SplashView: View {
    let onFinished: () -> Void

    @State private var flipped: [Bool] = Array(repeating: false, count: bitWidth)
    @State private var titleOpacity: Double = 0
    @State private var isExiting = false

    private static let bitWidth = 8
    private let staggerInterval: Double = 0.07

    var body: some View {
        VStack(spacing: 22) {
            HStack(spacing: 7) {
                ForEach(0..<Self.bitWidth, id: \.self) { index in
                    SplashBitCell(isOn: flipped[index])
                }
            }

            Text("BIT")
                .font(.system(size: 32, weight: .heavy, design: .monospaced))
                .tracking(6)
                .foregroundStyle(Theme.brand)
                .opacity(titleOpacity)
        }
        .opacity(isExiting ? 0 : 1)
        .scaleEffect(isExiting ? 0.94 : 1)
        .animation(.easeOut(duration: 0.25), value: isExiting)
        .onAppear(perform: runSequence)
    }

    private func runSequence() {
        for index in 0..<Self.bitWidth {
            DispatchQueue.main.asyncAfter(deadline: .now() + Double(index) * staggerInterval) {
                withAnimation(.spring(response: 0.35, dampingFraction: 0.65)) {
                    flipped[index] = true
                }
            }
        }
        Haptics.toggle()

        let lastFlipStart = Double(Self.bitWidth - 1) * staggerInterval
        let titleDelay = lastFlipStart + 0.35
        DispatchQueue.main.asyncAfter(deadline: .now() + titleDelay) {
            withAnimation(.easeOut(duration: 0.25)) {
                titleOpacity = 1
            }
            Haptics.success()
        }

        let exitDelay = titleDelay + 0.35
        DispatchQueue.main.asyncAfter(deadline: .now() + exitDelay) {
            isExiting = true
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + exitDelay + 0.25) {
            onFinished()
        }
    }
}

/// A single bit, flipping from a mirrored "0" to a normal "1" as it
/// turns on — same 3D-flip visual language as `FlipToggleCell`, just
/// driven by a timed sequence instead of a tap.
private struct SplashBitCell: View {
    let isOn: Bool

    var body: some View {
        RoundedRectangle(cornerRadius: 8)
            .fill(isOn ? Color.accentColor : Color.gray.opacity(0.25))
            .frame(width: 32, height: 46)
            .overlay(
                Text(isOn ? "1" : "0")
                    .font(.system(.headline, design: .rounded).bold())
                    .foregroundStyle(isOn ? .white : .secondary)
            )
            .rotation3DEffect(.degrees(isOn ? 0 : 180), axis: (x: 0, y: 1, z: 0), perspective: 0.35)
    }
}

/// Gates `RootView` behind `SplashView` on cold launch — skipped
/// entirely under `UITEST_SAVE_TOKEN` (see `Engine.swift`), since it
/// would only add ~1.3s of dead time to every UI test without proving
/// anything a screenshot couldn't already show.
struct LaunchView: View {
    @State private var showSplash: Bool

    init() {
        _showSplash = State(initialValue: ProcessInfo.processInfo.environment["UITEST_SAVE_TOKEN"] == nil)
    }

    var body: some View {
        Group {
            if showSplash {
                SplashView {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        showSplash = false
                    }
                }
                .transition(.opacity)
            } else {
                RootView()
                    .transition(.opacity)
            }
        }
    }
}
