import SwiftUI

/// Play is pure puzzle mode — no concept picker, no interactive demos
/// (those live in Explore now). Opening the tab drops straight into an
/// endless stream of challenges drawn only from concepts the player has
/// actually discovered (`gamePool`), with a plain running total at the
/// top. Difficulty ramps gently with that total; every ~15 correct
/// answers (see bit-core's `NEW_TOPIC_SUGGESTION_THRESHOLD`), if
/// something new is unlocked, a sheet offers to jump straight to it in
/// Explore.
struct PlayView: View {
    @EnvironmentObject var engine: Engine
    let onNavigateToExplore: (String?) -> Void

    @State private var pool: [FfiConceptSummary] = []
    @State private var currentConcept: FfiConceptSummary?
    @State private var challenge: FfiChallenge?
    @State private var answer = ""
    @State private var feedback: String?
    @State private var feedbackIsCorrect = false
    @State private var seed: UInt64 = 0
    @State private var levelNumber: UInt32 = 0
    @State private var streak: Int = 0
    @State private var comboPulse = false
    @State private var levelPulse = false
    @State private var isReviewPuzzle = false
    @State private var firstConcept: FfiConceptSummary?
    @State private var activeSheet: PlaySheetKind?
    @State private var recentPrompts: [String] = []
    @State private var showDailyChallenge = false
    @State private var dailyChallenge: FfiDailyChallenge?
    @State private var dailyResult: Bool?
    @FocusState private var answerFieldFocused: Bool

    /// How many past prompts `pickNewPuzzle` refuses to repeat. Small on
    /// purpose — big enough that the same question can't come back
    /// immediately, small enough that a short pool (2-3 discovered
    /// concepts) doesn't run out of fresh combinations to offer.
    private let recentPromptMemory = 6

    var body: some View {
        NavigationStack {
            VStack(spacing: 24) {
                if let dailyChallenge {
                    DailyChallengeCard(
                        dayNumber: dailyChallenge.dayNumber,
                        alreadyAttempted: dailyResult != nil,
                        wasCorrect: dailyResult ?? false,
                        action: { showDailyChallenge = true }
                    )
                }

                ZStack {
                    // A brief expanding, fading ring behind the number on
                    // every level-up — the number's own bounce
                    // (`contentTransition`) reads fine in isolation, but
                    // needs a little more "that mattered" than a font
                    // change alone gives it.
                    Circle()
                        .fill(currentConcept.map { Theme.color(forDomain: $0.domain) } ?? .accentColor)
                        .frame(width: 64, height: 64)
                        .scaleEffect(levelPulse ? 2.2 : 0.6)
                        .opacity(levelPulse ? 0 : 0.35)
                        .animation(.easeOut(duration: 0.55), value: levelPulse)

                    // "Level number on top" — deliberately just the number,
                    // no "Level" label, no per-concept framing.
                    Text("\(levelNumber)")
                        .font(.system(size: 40, weight: .heavy, design: .rounded))
                        .contentTransition(.numericText())
                        .animation(.spring(response: 0.3, dampingFraction: 0.6), value: levelNumber)
                        .accessibilityIdentifier("game-level-number")
                }

                if streak >= 2 {
                    HStack(spacing: 4) {
                        Image(systemName: "flame.fill")
                            .foregroundStyle(.orange)
                            .symbolEffect(.bounce, value: streak)
                        Text("\(streak) in a row")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(.secondary)

                        // Combo is deliberately separate from the daily
                        // streak (bit-core's `daily_streak`) — this is
                        // pure in-session game-feel, reset the instant a
                        // miss happens, never written to disk or scored.
                        if comboMultiplier > 1 {
                            Text("×\(comboMultiplier)")
                                .font(.caption.weight(.heavy))
                                .foregroundStyle(Theme.brand)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Capsule().fill(Theme.brand.opacity(0.15)))
                                .scaleEffect(comboPulse ? 1.35 : 1.0)
                                .animation(.spring(response: 0.25, dampingFraction: 0.45), value: comboPulse)
                                .accessibilityIdentifier("play-combo-multiplier")
                        }
                    }
                    .transition(.scale.combined(with: .opacity))
                    .accessibilityIdentifier("play-streak")
                }

                Spacer()

                if pool.isEmpty {
                    emptyPoolView
                } else if let currentConcept, let challenge {
                    puzzleView(concept: currentConcept, challenge: challenge)
                        .id(challenge.seed)
                        .transition(.asymmetric(
                            insertion: .move(edge: .trailing).combined(with: .opacity),
                            removal: .move(edge: .leading).combined(with: .opacity)
                        ))
                } else {
                    ProgressView()
                }

                Spacer()
            }
            .animation(.spring(response: 0.3, dampingFraction: 0.6), value: streak)
            .animation(.spring(response: 0.35, dampingFraction: 0.75), value: challenge?.seed)
            .padding()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .gameBackground()
            .tint(currentConcept.map { Theme.color(forDomain: $0.domain) })
            .navigationTitle("Play")
            .navigationBarTitleDisplayMode(.inline)
            .onAppear(perform: refresh)
            .sheet(isPresented: $showDailyChallenge, onDismiss: refreshDailyChallengeState) {
                DailyChallengeView()
            }
            .sheet(item: $activeSheet) { sheet in
                switch sheet {
                case .suggestion(let concept):
                    NewTopicSuggestionSheet(
                        concept: concept,
                        onLearn: {
                            activeSheet = nil
                            engine.acknowledgeNewTopicSuggestion()
                            onNavigateToExplore(concept.id)
                        },
                        onDismiss: {
                            activeSheet = nil
                            engine.acknowledgeNewTopicSuggestion()
                        }
                    )
                case .milestone(let milestone):
                    MilestoneCelebrationSheet(milestone: milestone) {
                        activeSheet = nil
                    }
                }
            }
        }
    }

    private var emptyPoolView: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)
                .symbolEffect(.pulse)
            Text("Nothing to play yet")
                .font(.title2.bold())

            if let firstConcept {
                Text("\(firstConcept.title) is a great place to start.")
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                Button("Start with \(firstConcept.title)") { onNavigateToExplore(firstConcept.id) }
                    .buttonStyle(.squishy)
                    .tint(Theme.color(forDomain: firstConcept.domain))
                    .accessibilityIdentifier("play-start-first-concept")
            } else {
                Text("Learn your first concept in Explore, then come back here to practice it.")
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                Button("Go to Explore") { onNavigateToExplore(nil) }
                    .buttonStyle(.squishy)
            }
        }
        .padding()
    }

    private func puzzleView(concept: FfiConceptSummary, challenge: FfiChallenge) -> some View {
        VStack(spacing: 20) {
            HStack(spacing: 6) {
                Circle().fill(Theme.color(forDomain: concept.domain)).frame(width: 8, height: 8)
                Text(concept.title).font(.caption).foregroundStyle(.secondary)
                if isReviewPuzzle {
                    Label("Review", systemImage: "arrow.triangle.2.circlepath")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(.orange)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Capsule().fill(Color.orange.opacity(0.15)))
                        .transition(.scale.combined(with: .opacity))
                        .accessibilityIdentifier("play-review-badge")
                }
            }

            // A quiet difficulty readout — never a number, just how full
            // the bar is, so it registers as "this is ramping up" without
            // turning into a second score to track. This is the *overall*
            // ramp (tied to the lifetime level number), not a per-puzzle
            // or per-concept meter — it reads the same whether the
            // current puzzle happens to be Arrays or Binary, and only
            // fills in further as the level number climbs. The "Review"
            // badge above is the only thing that's puzzle-specific.
            VStack(spacing: 2) {
                HStack(spacing: 3) {
                    ForEach(0..<8, id: \.self) { tier in
                        Capsule()
                            .fill(tier < Int(currentDifficulty) ? Theme.color(forDomain: concept.domain) : Color.gray.opacity(0.2))
                            .frame(width: 12, height: 4)
                    }
                }
                Text("Overall difficulty")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            .animation(.spring(response: 0.4, dampingFraction: 0.8), value: currentDifficulty)

            Text(challenge.prompt)
                .font(.title2.monospaced())
                .multilineTextAlignment(.center)

            TextField("Your answer", text: $answer)
                .textFieldStyle(.roundedBorder)
                .keyboardType(answerKeyboardType)
                .autocorrectionDisabled()
                .multilineTextAlignment(.center)
                .frame(maxWidth: 200)
                .focused($answerFieldFocused)
                .submitLabel(.done)
                .onSubmit { checkAnswer() }
                .accessibilityIdentifier("play-answer-field")

            Button("Check") { checkAnswer() }
                .buttonStyle(.squishy)
                .disabled(answer.isEmpty)
                .accessibilityIdentifier("play-check-answer")

            // Stuck on this one? Swap it for a new puzzle from the same
            // pool — deliberately never calls `recordAttempt` or touches
            // the streak/level, so skipping never costs anything.
            Button("Skip this one") {
                answerFieldFocused = false
                pickNewPuzzle()
            }
            .font(.subheadline)
            .foregroundStyle(.secondary)
            .accessibilityIdentifier("play-skip-puzzle")

            if let feedback {
                VStack(spacing: 10) {
                    HStack(spacing: 6) {
                        if feedbackIsCorrect {
                            Image(systemName: "checkmark.circle.fill")
                                .symbolEffect(.bounce, value: feedback)
                        }
                        Text(feedback)
                    }
                    .foregroundStyle(feedbackIsCorrect ? .green : .red)

                    // A miss is a teaching moment, not just a scoreboard
                    // entry — one tap should get you back to the concept's
                    // own Explore page instead of just re-guessing.
                    if !feedbackIsCorrect {
                        Button {
                            onNavigateToExplore(concept.id)
                        } label: {
                            Label("Explain \(concept.title)", systemImage: "book.fill")
                        }
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(Theme.color(forDomain: concept.domain))
                        .accessibilityIdentifier("play-explain-this")
                    }
                }
                .transition(.scale.combined(with: .opacity))
                .animation(.spring(response: 0.3, dampingFraction: 0.6), value: feedback)
            }
        }
    }

    private func refresh() {
        refreshDailyChallengeState()
        pool = engine.ffi.gamePool()
        levelNumber = engine.ffi.gameTotalCorrect()
        if pool.isEmpty {
            // A brand-new install has nothing to suggest via the
            // "you've been on a roll" sheet yet (that needs a streak of
            // correct answers), so the empty state itself points at a
            // concrete first concept instead of leaving the player to
            // guess where to start.
            firstConcept = engine.ffi.nextLearnableConcept()
        } else if challenge == nil {
            pickNewPuzzle()
        }
    }

    private func refreshDailyChallengeState() {
        let today = Engine.todayString()
        dailyChallenge = engine.ffi.dailyChallenge(today: today)
        dailyResult = engine.ffi.dailyChallengeResult(today: today)
    }

    /// Tiers are deliberately coarse, not 1:1 with `streak` — the payoff
    /// is "you leveled up your multiplier," a milestone that fires a
    /// handful of times per session, not a number that ticks on every
    /// single correct answer (which would read as noise, not a moment).
    private var comboMultiplier: Int {
        switch streak {
        case 0...1: return 1
        case 2...4: return 2
        case 5...9: return 3
        default: return 4
        }
    }

    /// Difficulty ramps gently with the running total rather than staying
    /// fixed — every generator clamps its own difficulty range, so this
    /// just has to trend upward, not land on an exact valid value.
    private var currentDifficulty: UInt8 {
        UInt8(min(8, 3 + levelNumber / 10))
    }

    /// Most answers here are plain digits — a dedicated number pad means
    /// fewer taps and no accidental letters. Two exceptions: ASCII
    /// Lookup's answer can be a single printable character as often as a
    /// number (direction is randomized per puzzle), and Two's
    /// Complement's signed answers need a minus sign a bare number pad
    /// doesn't have.
    private var answerKeyboardType: UIKeyboardType {
        switch currentConcept?.id {
        case "ascii": return .asciiCapable
        case "twos_complement": return .numbersAndPunctuation
        default: return .numberPad
        }
    }

    private func pickNewPuzzle() {
        // Avoid immediately repeating the same concept when there's a
        // choice, so the puzzle mix actually feels mixed.
        let candidates = pool.count > 1 ? pool.filter { $0.id != currentConcept?.id } : pool
        let today = Engine.todayString()

        // Weighted, not flat-random: a concept overdue for review (see
        // `BitEngine::is_rusty`) is 3x as likely to come up as anything
        // else discovered — enough to resurface it regularly without
        // ever fully crowding out the rest of the pool.
        let weighted = candidates.flatMap { c in
            Array(repeating: c, count: engine.ffi.isRusty(conceptId: c.id, today: today) ? 3 : 1)
        }
        guard !weighted.isEmpty else { return }

        // Seeds are random, so a fresh seed almost always produces a
        // fresh prompt — this just re-rolls a handful of times on the
        // rare collision instead of ever asking the exact same question
        // twice within the last few puzzles. Each attempt's result is
        // kept as a fallback in case every retry collides (only possible
        // with a tiny pool and a short prompt space).
        var picked: (concept: FfiConceptSummary, challenge: FfiChallenge, seed: UInt64)?
        for _ in 0..<8 {
            guard let concept = weighted.randomElement() else { break }
            let candidateSeed = UInt64.random(in: UInt64.min...UInt64.max)
            guard let candidateChallenge = engine.ffi.generateChallenge(
                conceptId: concept.id, difficulty: currentDifficulty, seed: candidateSeed
            ) else { continue }
            picked = (concept, candidateChallenge, candidateSeed)
            if !recentPrompts.contains(candidateChallenge.prompt) {
                break
            }
        }
        guard let picked else { return }

        currentConcept = picked.concept
        isReviewPuzzle = engine.ffi.isRusty(conceptId: picked.concept.id, today: today)
        seed = picked.seed
        challenge = picked.challenge
        answer = ""
        feedback = nil

        recentPrompts.append(picked.challenge.prompt)
        if recentPrompts.count > recentPromptMemory {
            recentPrompts.removeFirst(recentPrompts.count - recentPromptMemory)
        }
    }

    private func checkAnswer() {
        answerFieldFocused = false // dismiss the keyboard — nothing behind it needs covering once submitted
        guard let currentConcept,
              let result = engine.ffi.validateChallenge(conceptId: currentConcept.id, difficulty: currentDifficulty, seed: seed, answer: answer)
        else { return }

        engine.recordAttempt(conceptId: currentConcept.id, correct: result.correct)
        feedbackIsCorrect = result.correct
        feedback = result.correct ? "Correct!" : (result.explanation ?? "Not quite — try again.")

        if result.correct {
            let previousCombo = comboMultiplier
            streak += 1
            if comboMultiplier > previousCombo {
                Haptics.comboUp()
                comboPulse = false
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.02) { comboPulse = true }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { comboPulse = false }
            } else {
                Haptics.success()
            }
            engine.recordGameCorrectAnswer()
            levelNumber = engine.ffi.gameTotalCorrect()
            levelPulse = false
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.02) { levelPulse = true }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                levelPulse = false
                // At most one sheet at a time: a milestone takes priority
                // over the new-topic nudge (it's rarer and time-bound to
                // this exact answer), which only gets checked otherwise.
                if let milestone = checkMilestones(conceptId: currentConcept.id) {
                    activeSheet = .milestone(milestone)
                } else if engine.ffi.shouldSuggestNewTopic(), let next = engine.ffi.nextLearnableConcept() {
                    activeSheet = .suggestion(next)
                }
                pool = engine.ffi.gamePool()
                pickNewPuzzle()
            }
        } else {
            Haptics.failure()
            streak = 0
        }
    }

    /// Checks (and, on a hit, immediately marks) the milestones that can
    /// fire right after a correct answer. Returns at most one — if
    /// several become true on the same answer, only the first checked
    /// celebrates now; the rest still fire on a later correct answer,
    /// since they stay unmarked until then.
    private func checkMilestones(conceptId: String) -> MilestoneInfo? {
        if levelNumber >= 100, !engine.ffi.hasCelebrated(id: "100_correct") {
            engine.markCelebrated(id: "100_correct")
            return MilestoneInfo(
                id: "100_correct",
                icon: "100.circle.fill",
                title: "100 Correct Answers!",
                message: "A hundred puzzles down. The streak continues.",
                color: .blue
            )
        }

        let mastery = engine.ffi.mastery(conceptId: conceptId)
        if mastery.stage == .mastered, !engine.ffi.hasCelebrated(id: "first_mastery") {
            engine.markCelebrated(id: "first_mastery")
            return MilestoneInfo(
                id: "first_mastery",
                icon: "star.circle.fill",
                title: "First Concept Mastered!",
                message: "You've mastered \(currentConcept?.title ?? "a concept") — proven it again and again, not just once.",
                color: .green
            )
        }

        if engine.ffi.discoveredConceptCount() >= 10, !engine.ffi.hasCelebrated(id: "10_discovered") {
            engine.markCelebrated(id: "10_discovered")
            return MilestoneInfo(
                id: "10_discovered",
                icon: "eye.circle.fill",
                title: "10 Concepts Discovered!",
                message: "Ten topics learned so far. The pool of things to practice just keeps growing.",
                color: .purple
            )
        }

        if engine.ffi.dailyStreak() >= 7, !engine.ffi.hasCelebrated(id: "7_day_streak") {
            engine.markCelebrated(id: "7_day_streak")
            return MilestoneInfo(
                id: "7_day_streak",
                icon: "calendar.circle.fill",
                title: "7 Day Streak!",
                message: "A full week of showing up. That consistency is what actually builds mastery.",
                color: .orange
            )
        }

        return nil
    }
}

enum PlaySheetKind: Identifiable {
    case suggestion(FfiConceptSummary)
    case milestone(MilestoneInfo)

    var id: String {
        switch self {
        case .suggestion(let concept): return "suggestion-\(concept.id)"
        case .milestone(let milestone): return "milestone-\(milestone.id)"
        }
    }
}

struct NewTopicSuggestionSheet: View {
    let concept: FfiConceptSummary
    let onLearn: () -> Void
    let onDismiss: () -> Void

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "star.circle.fill")
                .font(.system(size: 56))
                .foregroundStyle(Theme.color(forDomain: concept.domain))
                .symbolEffect(.bounce, value: concept.id)

            Text("Ready for something new?")
                .font(.title2.bold())
            Text("You've been on a roll — \(concept.title) is ready to learn.")
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            Button("Learn \(concept.title)", action: onLearn)
                .buttonStyle(.squishy)
                .tint(Theme.color(forDomain: concept.domain))
                .accessibilityIdentifier("suggestion-learn")

            Button("Keep practicing", action: onDismiss)
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("suggestion-dismiss")
        }
        .padding()
        .presentationDetents([.medium])
    }
}
