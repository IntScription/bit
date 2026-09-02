import SwiftUI

/// The compact entry point embedded at the top of Play — shows today's
/// day number and whether it's already been solved, without requiring a
/// tap to find out. Tapping opens `DailyChallengeView` as a sheet.
///
/// Swipeable, not just tappable: dragging it left collapses it into a
/// small bubble pinned to the left edge (a chat-heads-style minimize,
/// not a dismiss) — the puzzle is still one tap away, it just stops
/// taking up banner space for the rest of this Play session. Purely
/// local UI state, not persisted: a fresh launch always shows the full
/// card again, so the daily-habit hook stays at full visibility on the
/// day it matters most (a new session), while still letting it get out
/// of the way once the player's already seen it today.
struct DailyChallengeCard: View {
    let dayNumber: UInt32
    let alreadyAttempted: Bool
    let wasCorrect: Bool
    let action: () -> Void

    @State private var collapsed = false
    @State private var dragOffset: CGFloat = 0

    /// How far left a drag has to travel before release commits to
    /// collapsing, rather than springing back — big enough that a
    /// glancing swipe (or a tap that wobbles slightly) doesn't
    /// accidentally collapse it.
    private let collapseThreshold: CGFloat = -70

    private var icon: String {
        alreadyAttempted ? (wasCorrect ? "checkmark.seal.fill" : "seal") : "calendar.badge.clock"
    }

    var body: some View {
        HStack {
            if collapsed {
                bubble.transition(.scale.combined(with: .opacity))
            } else {
                card
                    .offset(x: dragOffset)
                    .transition(.scale.combined(with: .opacity))
                Spacer(minLength: 0)
            }
        }
        .animation(.spring(response: 0.35, dampingFraction: 0.75), value: collapsed)
    }

    private var card: some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .font(.title3)
                    .foregroundStyle(Theme.brand)

                VStack(alignment: .leading, spacing: 2) {
                    Text("Puzzle of the Day #\(dayNumber)")
                        .font(.subheadline.weight(.semibold))
                    Text(alreadyAttempted ? "Solved today — tap to see your result" : "One puzzle, same for everyone today")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Spacer()
                Image(systemName: "chevron.right")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.tertiary)
            }
            .padding(12)
            .background(Theme.brand.opacity(0.12), in: RoundedRectangle(cornerRadius: 14))
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("daily-challenge-card")
        // `minimumDistance` keeps a plain tap from ever being mistaken
        // for a drag — the gesture doesn't even start recognizing until
        // the touch has moved, so `action` above still fires normally.
        .simultaneousGesture(
            DragGesture(minimumDistance: 20)
                .onChanged { value in
                    dragOffset = min(0, value.translation.width)
                }
                .onEnded { value in
                    if value.translation.width < collapseThreshold {
                        Haptics.toggle()
                        collapsed = true
                    }
                    dragOffset = 0
                }
        )
    }

    private var bubble: some View {
        Button(action: action) {
            ZStack {
                Circle().fill(Theme.brand).frame(width: 46, height: 46)
                Image(systemName: icon)
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(.white)
            }
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("daily-challenge-bubble")
        .accessibilityLabel("Puzzle of the Day")
    }
}

/// Answering flow + Wordle-style shareable result, all in one sheet.
/// Deliberately reuses `validateChallenge`/`recordDailyChallengeResult`
/// rather than any bespoke daily-only validation path — a daily puzzle is
/// just a regular challenge whose `(conceptId, difficulty, seed)` happen
/// to be picked by the date instead of by Play's random roll.
struct DailyChallengeView: View {
    @EnvironmentObject var engine: Engine
    @Environment(\.dismiss) private var dismiss

    @State private var daily: FfiDailyChallenge?
    @State private var answer = ""
    @State private var result: Bool?
    @FocusState private var answerFieldFocused: Bool

    var body: some View {
        NavigationStack {
            VStack(spacing: 24) {
                if let daily {
                    Text("Day #\(daily.dayNumber)")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.secondary)

                    if let result {
                        resultView(daily: daily, correct: result)
                    } else {
                        answeringView(daily: daily)
                    }
                } else {
                    ProgressView()
                }
            }
            .padding()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .gameBackground()
            .tint(Theme.brand)
            .navigationTitle("Puzzle of the Day")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Close") { dismiss() }
                }
            }
            .onAppear(perform: refresh)
        }
    }

    private func answeringView(daily: FfiDailyChallenge) -> some View {
        VStack(spacing: 20) {
            Text(daily.conceptTitle)
                .font(.caption)
                .foregroundStyle(.secondary)

            Text(daily.challenge.prompt)
                .font(.title2.monospaced())
                .multilineTextAlignment(.center)

            TextField("Your answer", text: $answer)
                .textFieldStyle(.roundedBorder)
                .keyboardType(answerKeyboardType(conceptId: daily.conceptId))
                .autocorrectionDisabled()
                .multilineTextAlignment(.center)
                .frame(maxWidth: 200)
                .focused($answerFieldFocused)
                .submitLabel(.done)
                .onSubmit { checkAnswer(daily: daily) }
                .accessibilityIdentifier("daily-answer-field")

            Button("Check") { checkAnswer(daily: daily) }
                .buttonStyle(.squishy)
                .disabled(answer.isEmpty)
                .accessibilityIdentifier("daily-check-answer")
        }
    }

    private func resultView(daily: FfiDailyChallenge, correct: Bool) -> some View {
        VStack(spacing: 20) {
            Image(systemName: correct ? "checkmark.circle.fill" : "xmark.circle.fill")
                .font(.system(size: 56))
                .foregroundStyle(correct ? .green : .red)
                .symbolEffect(.bounce, value: correct)

            Text(correct ? "Solved!" : "Not quite")
                .font(.title2.bold())
            Text(daily.conceptTitle)
                .foregroundStyle(.secondary)

            HStack(spacing: 6) {
                Image(systemName: "flame.fill").foregroundStyle(.orange)
                Text("\(engine.ffi.dailyStreak())-day streak")
                    .font(.subheadline.weight(.semibold))
            }
            .accessibilityIdentifier("daily-streak-readout")

            ShareLink(item: shareText(daily: daily, correct: correct)) {
                Label("Share result", systemImage: "square.and.arrow.up")
            }
            .buttonStyle(.squishy)
            .tint(Theme.brand)
            .accessibilityIdentifier("daily-share-button")
        }
        .transition(.scale.combined(with: .opacity))
    }

    private func shareText(daily: FfiDailyChallenge, correct: Bool) -> String {
        "BIT Puzzle of the Day #\(daily.dayNumber)\n\(correct ? "✅" : "❌") \(daily.conceptTitle)\n🔥 \(engine.ffi.dailyStreak())-day streak"
    }

    private func refresh() {
        guard let today = engine.ffi.dailyChallenge(today: Engine.todayString()) else { return }
        daily = today
        if let previousResult = engine.ffi.dailyChallengeResult(today: Engine.todayString()) {
            result = previousResult
        }
    }

    private func answerKeyboardType(conceptId: String) -> UIKeyboardType {
        switch conceptId {
        case "ascii": return .asciiCapable
        case "twos_complement": return .numbersAndPunctuation
        default: return .numberPad
        }
    }

    private func checkAnswer(daily: FfiDailyChallenge) {
        answerFieldFocused = false
        guard let validation = engine.ffi.validateChallenge(
            conceptId: daily.conceptId, difficulty: daily.difficulty, seed: daily.challenge.seed, answer: answer
        ) else { return }

        engine.recordDailyChallengeResult(correct: validation.correct)
        if validation.correct { Haptics.success() } else { Haptics.failure() }
        withAnimation(.spring(response: 0.3, dampingFraction: 0.6)) {
            result = validation.correct
        }
    }
}
