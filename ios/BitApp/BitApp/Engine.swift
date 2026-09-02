import Foundation

/// Owns the single `FfiEngine` handle for the app's lifetime. Everything
/// mutable (mastery, live build sessions) lives behind that one Rust-side
/// Mutex — see bit-ffi's module doc — so this wrapper stays thin: it
/// resolves where the bundled `/content` directory lives, and where the
/// player's save file lives, and makes sure progress mutations get
/// written to disk.
@MainActor
final class Engine: ObservableObject {
    let ffi: FfiEngine
    private let saveURL: URL

    init() {
        guard let resourcePath = Bundle.main.resourcePath else {
            fatalError("BIT: app bundle has no resource path")
        }
        let contentDir = resourcePath + "/content"
        do {
            ffi = try FfiEngine(contentDir: contentDir)
        } catch {
            fatalError("BIT: failed to load content from \(contentDir): \(error)")
        }

        let documentsDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        // UI tests set this to a per-test-invocation UUID so each test
        // method gets an isolated save file — without it, persistence
        // (this file existing at all) would leak state between test
        // methods that otherwise assume a fresh install.
        if let token = ProcessInfo.processInfo.environment["UITEST_SAVE_TOKEN"] {
            saveURL = documentsDir.appendingPathComponent("bit-save-uitest-\(token).json")
        } else {
            saveURL = documentsDir.appendingPathComponent("bit-save.json")
        }

        if let json = try? String(contentsOf: saveURL, encoding: .utf8) {
            try? ffi.restoreJson(json: json)
        }
    }

    /// Every mutating call site (Play, Explore's discovery-on-view) routes
    /// through here instead of calling `ffi.recordDiscovery`/`recordAttempt`
    /// directly, so persistence can never be forgotten at a new call site.
    func recordDiscovery(conceptId: String) {
        ffi.recordDiscovery(conceptId: conceptId)
        ffi.recordDailyActivity(today: Engine.todayString())
        persist()
    }

    func recordAttempt(conceptId: String, correct: Bool) {
        ffi.recordAttempt(conceptId: conceptId, correct: correct, today: Engine.todayString())
        persist()
    }

    /// Play's endless game mode calls this in addition to
    /// `recordAttempt` (which still tracks per-concept mastery) — this
    /// only bumps the global "level number" shown at the top of the game
    /// screen, and only ever on a correct answer. Also the other trigger
    /// (alongside discovery) for the daily calendar streak — deliberately
    /// not every attempt, so the streak reflects real engagement
    /// (learned something new, or actually got a puzzle right) rather
    /// than just having the app open.
    func recordGameCorrectAnswer() {
        ffi.recordGameCorrectAnswer()
        ffi.recordDailyActivity(today: Engine.todayString())
        persist()
    }

    /// "YYYY-MM-DD" in the device's local calendar day — the one place in
    /// the app that asks the system clock what day it is; Rust's daily
    /// streak logic stays pure and deterministic given this string (see
    /// `BitEngine::record_daily_activity`'s doc comment).
    static func todayString() -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        formatter.timeZone = .current
        formatter.locale = Locale(identifier: "en_US_POSIX")
        return formatter.string(from: Date())
    }

    /// Resets the "correct since last suggestion" counter — call once the
    /// player has seen the new-topic prompt, whether accepted or dismissed.
    func acknowledgeNewTopicSuggestion() {
        ffi.acknowledgeNewTopicSuggestion()
        persist()
    }

    /// Marks a one-time celebration (see `MilestoneInfo`) as shown so it
    /// never fires again. Reading whether one has already fired doesn't
    /// need a wrapper — call `ffi.hasCelebrated(id:)` directly, same as
    /// other read-only queries elsewhere in the app.
    func markCelebrated(id: String) {
        ffi.markCelebrated(id: id)
        persist()
    }

    /// Also feeds the regular daily streak (see `record_daily_challenge_
    /// result`'s doc comment on the Rust side) — solving the Puzzle of the
    /// Day alone is enough to keep it alive, same as any other engagement.
    func recordDailyChallengeResult(correct: Bool) {
        ffi.recordDailyChallengeResult(today: Engine.todayString(), correct: correct)
        persist()
    }

    func toggleBookmark(conceptId: String) {
        ffi.toggleBookmark(conceptId: conceptId)
        persist()
    }

    /// An empty `text` clears the note — see `BitEngine::set_note`'s doc
    /// comment. Callers save on blur, not per keystroke (see
    /// `ConceptDetailView`), so this isn't on a hot path.
    func setNote(conceptId: String, text: String) {
        ffi.setNote(conceptId: conceptId, text: text)
        persist()
    }

    /// Also called on scene-phase transitions to background (see
    /// BitApp.swift) as a safety net — the per-mutation persist() above
    /// should already cover it, but iOS can suspend/terminate an app
    /// without warning, so belt and suspenders.
    func persist() {
        let json = ffi.saveJson()
        try? json.write(to: saveURL, atomically: true, encoding: .utf8)
        refreshStreakReminder()
        writeSharedWidgetSnapshot()
    }

    /// Mirrors just enough state into the App Group container for the
    /// home screen widget to read without linking against `FfiEngine`
    /// (widget extensions are a separate process/module). A no-op until
    /// the widget target and its matching App Group entitlement exist —
    /// `UserDefaults(suiteName:)` degrades to a harmless, merely-
    /// unshared store rather than crashing when the group isn't
    /// provisioned yet, so this is safe to ship ahead of that target.
    /// Keys here are duplicated (not imported) on the widget side — see
    /// that target's own copy of `SharedWidgetData` — since an extension
    /// can't import the main app's Swift files directly.
    private func writeSharedWidgetSnapshot() {
        guard let shared = UserDefaults(suiteName: SharedWidgetData.suiteName) else { return }
        let today = Engine.todayString()
        shared.set(ffi.dailyStreak(), forKey: SharedWidgetData.dailyStreakKey)
        if let daily = ffi.dailyChallenge(today: today) {
            shared.set(daily.dayNumber, forKey: SharedWidgetData.dayNumberKey)
        }
        shared.set(ffi.dailyChallengeResult(today: today) != nil, forKey: SharedWidgetData.solvedTodayKey)
    }

    /// Every mutation already routes through `persist()`, so this is the
    /// one place a streak-reminder reschedule needs to happen — no need
    /// to sprinkle calls across every `record*` method above. Also called
    /// directly on app foreground (see BitApp.swift) to catch the case
    /// where the calendar day rolled over while the app was suspended.
    func refreshStreakReminder() {
        let today = Engine.todayString()
        let alreadyActiveToday = ffi.activityHistory().contains { $0.day == today && $0.count > 0 }
        StreakReminders.reschedule(alreadyActiveToday: alreadyActiveToday, currentStreak: ffi.dailyStreak())
    }
}
