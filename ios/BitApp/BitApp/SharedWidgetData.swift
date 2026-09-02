import Foundation

/// The App Group contract between BitApp and the BitWidget extension.
/// Deliberately just string/key constants, not a shared framework — for
/// a surface this small, duplicating these four lines in the widget
/// target (see its own `SharedWidgetData.swift`) is simpler than the
/// build-system overhead of a shared framework target, and the doc
/// comments on each side point at the other so a rename can't drift
/// silently.
///
/// `suiteName` must exactly match the App Group ID enabled in Signing &
/// Capabilities on *both* the BitApp and BitWidget targets — see
/// ARCHITECTURE.md's Puzzle of the Day / widget section for the setup
/// steps. Until that capability exists, `UserDefaults(suiteName:)` still
/// returns a valid (just unshared) store, so writes here are always safe.
enum SharedWidgetData {
    static let suiteName = "group.dev.kartiksanil.bit"
    static let dailyStreakKey = "dailyStreak"
    static let dayNumberKey = "dayNumber"
    static let solvedTodayKey = "solvedToday"
}
