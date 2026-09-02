import UIKit

/// Small tactile cues — a toggle should feel like flipping a physical
/// switch, a correct answer should feel like a win. No-ops harmlessly on
/// the Simulator (no Taptic Engine to drive) and on devices without one.
enum Haptics {
    static func toggle() {
        UIImpactFeedbackGenerator(style: .light).impactOccurred()
    }

    static func success() {
        UINotificationFeedbackGenerator().notificationOccurred(.success)
    }

    static func failure() {
        UINotificationFeedbackGenerator().notificationOccurred(.error)
    }

    /// A distinct, heavier thump for a Play combo multiplier tier-up —
    /// deliberately not the same feel as a plain correct answer
    /// (`.success()`), since this is meant to register as "that mattered
    /// more," not just "correct again."
    static func comboUp() {
        UIImpactFeedbackGenerator(style: .heavy).impactOccurred()
    }
}
