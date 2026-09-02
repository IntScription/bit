import Foundation
import UserNotifications

/// Local-only streak reminders — no backend, no push token, nothing
/// leaves the device. A single reusable notification identifier means
/// "reschedule" is just "remove, then maybe re-add," so there's never a
/// way to end up with duplicate pending reminders piling up.
enum StreakReminders {
    private static let identifier = "streak-reminder"
    private static let enabledKey = "streakRemindersEnabled"
    /// 8pm local — late enough that most of a day's real engagement has
    /// already happened, early enough not to be a bedtime intrusion.
    private static let reminderHour = 20

    static var isEnabled: Bool {
        get { UserDefaults.standard.bool(forKey: enabledKey) }
        set { UserDefaults.standard.set(newValue, forKey: enabledKey) }
    }

    /// Call when the player turns reminders on from Profile. Requests
    /// system permission (the OS only ever prompts once — a prior denial
    /// just resolves `granted: false` here without showing anything).
    /// `isEnabled` always ends up matching what was actually granted, so
    /// the Profile toggle can't show "on" when the OS silently said no.
    static func enable(completion: @escaping (Bool) -> Void) {
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { granted, _ in
            DispatchQueue.main.async {
                isEnabled = granted
                completion(granted)
            }
        }
    }

    static func disable() {
        isEnabled = false
        UNUserNotificationCenter.current().removePendingNotificationRequests(withIdentifiers: [identifier])
    }

    /// Cheap and safe to call often — on every save (see `Engine.persist`)
    /// and on app foreground. Always clears the previous pending reminder
    /// first, then re-adds one only if reminders are on *and* today has
    /// no recorded activity yet, so a player who already engaged today
    /// never gets nagged, and toggling off never leaves a stale reminder
    /// behind.
    static func reschedule(alreadyActiveToday: Bool, currentStreak: UInt32) {
        let center = UNUserNotificationCenter.current()
        center.removePendingNotificationRequests(withIdentifiers: [identifier])
        guard isEnabled, !alreadyActiveToday else { return }

        let content = UNMutableNotificationContent()
        content.title = "Keep your streak alive"
        content.body = currentStreak > 0
            ? "Your \(currentStreak)-day streak is waiting — one puzzle keeps it going."
            : "Today's Puzzle of the Day is ready whenever you are."
        content.sound = .default

        // Partial date components + `repeats: false` fires at the *next*
        // occurrence of that hour — today at 8pm if it hasn't passed yet,
        // otherwise tomorrow. `reschedule` gets called again well before
        // then (every save, every foreground), so it never goes stale.
        var when = DateComponents()
        when.hour = reminderHour
        let trigger = UNCalendarNotificationTrigger(dateMatching: when, repeats: false)
        center.add(UNNotificationRequest(identifier: identifier, content: content, trigger: trigger))
    }
}
