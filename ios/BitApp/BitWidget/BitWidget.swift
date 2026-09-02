//
//  BitWidget.swift
//  BitWidget
//

import WidgetKit
import SwiftUI

/// Duplicated from the main app's own `SharedWidgetData.swift` — a widget
/// extension is a separate module and can't import the app target's
/// Swift files directly. Keep these four lines in sync by hand if either
/// side ever changes; there's nothing more here than key names.
enum SharedWidgetData {
    static let suiteName = "group.dev.kartiksanil.bit"
    static let dailyStreakKey = "dailyStreak"
    static let dayNumberKey = "dayNumber"
    static let solvedTodayKey = "solvedToday"
}

struct StreakEntry: TimelineEntry {
    let date: Date
    let dailyStreak: Int
    let dayNumber: Int
    let solvedToday: Bool
}

struct StreakProvider: TimelineProvider {
    func placeholder(in context: Context) -> StreakEntry {
        StreakEntry(date: Date(), dailyStreak: 3, dayNumber: 42, solvedToday: false)
    }

    func getSnapshot(in context: Context, completion: @escaping (StreakEntry) -> Void) {
        completion(currentEntry())
    }

    /// A single entry, refreshed on next app launch/foreground (which
    /// rewrites the App Group snapshot — see `Engine.persist()`) rather
    /// than a self-driven timeline: nothing about the streak changes on
    /// its own between player actions, so there's nothing to schedule
    /// ahead of time.
    func getTimeline(in context: Context, completion: @escaping (Timeline<StreakEntry>) -> Void) {
        completion(Timeline(entries: [currentEntry()], policy: .never))
    }

    private func currentEntry() -> StreakEntry {
        let shared = UserDefaults(suiteName: SharedWidgetData.suiteName)
        return StreakEntry(
            date: Date(),
            dailyStreak: shared?.integer(forKey: SharedWidgetData.dailyStreakKey) ?? 0,
            dayNumber: shared?.integer(forKey: SharedWidgetData.dayNumberKey) ?? 0,
            solvedToday: shared?.bool(forKey: SharedWidgetData.solvedTodayKey) ?? false
        )
    }
}

struct BitWidgetEntryView: View {
    var entry: StreakProvider.Entry

    /// Matches `Theme.brand` in the main app (kept as a literal here,
    /// same reasoning as `SharedWidgetData` above — the widget target
    /// can't import the app's `Theme.swift`).
    private static let brand = Color(red: 0.55, green: 0.25, blue: 0.98)

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 4) {
                Image(systemName: entry.solvedToday ? "checkmark.seal.fill" : "flame.fill")
                    .foregroundStyle(entry.solvedToday ? .green : .orange)
                Text("\(entry.dailyStreak)")
                    .font(.system(size: 28, weight: .heavy, design: .rounded))
            }
            Text(entry.dailyStreak == 1 ? "day streak" : "day streak")
                .font(.caption)
                .foregroundStyle(.secondary)

            Spacer()

            Text(entry.solvedToday ? "Solved today" : "Puzzle #\(entry.dayNumber) awaits")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(Self.brand)
                .lineLimit(1)
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
        .containerBackground(for: .widget) {
            Self.brand.opacity(0.1)
        }
    }
}

struct BitWidget: Widget {
    let kind = "dev.kartiksanil.bit.streakwidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: StreakProvider()) { entry in
            BitWidgetEntryView(entry: entry)
        }
        .configurationDisplayName("BIT Streak")
        .description("Your current daily streak and whether today's Puzzle of the Day is solved.")
        .supportedFamilies([.systemSmall])
    }
}

#Preview(as: .systemSmall) {
    BitWidget()
} timeline: {
    StreakEntry(date: .now, dailyStreak: 3, dayNumber: 42, solvedToday: false)
    StreakEntry(date: .now, dailyStreak: 7, dayNumber: 43, solvedToday: true)
}
