import SwiftUI

/// Discovery, mastery, and creation — never a school grade (spec §2's
/// PROFILE section). No streak-shaming, no percentage-to-pass framing.
struct ProfileView: View {
    @EnvironmentObject var engine: Engine
    let onNavigateToExplore: (String?) -> Void

    @State private var concepts: [FfiConceptSummary] = []
    @State private var masteryById: [String: FfiMastery] = [:]
    @State private var domainNames: [String: String] = [:]
    @State private var domainOrder: [String] = []
    @State private var dailyStreak: UInt32 = 0
    @State private var longestDailyStreak: UInt32 = 0
    @State private var activityByDay: [String: UInt32] = [:]
    @State private var bookmarks: [FfiConceptSummary] = []
    @State private var celebratedIds: Set<String> = []
    @State private var remindersEnabled = StreakReminders.isEnabled

    var body: some View {
        NavigationStack {
            List {
                // No custom `.listRowInsets` on either row here — that
                // was the actual cause of the "not aligned left/right"
                // report: overriding to zero made these two rows span
                // edge-to-edge while every other section's rows (below)
                // kept the List's own default margins, so the cards
                // visibly stuck out past everything else. Inheriting the
                // same default insets as those rows is the fix.
                Section {
                    HStack(spacing: 12) {
                        StatCard(icon: "eye.fill", value: discoveredCount, of: concepts.count, label: "Concepts discovered", color: .blue)
                        StatCard(icon: "star.fill", value: masteredCount, of: nil, label: "Concepts mastered", color: .green)
                    }
                    .listRowSeparator(.hidden)
                    .listRowBackground(Color.clear)

                    HStack(spacing: 6) {
                        Image(systemName: "calendar")
                            .foregroundStyle(.orange)
                        Text(dailyStreak == 1 ? "1 day streak" : "\(dailyStreak) day streak")
                            .font(.subheadline.weight(.semibold))
                        Spacer()
                        if longestDailyStreak > dailyStreak {
                            Text("best: \(longestDailyStreak)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 10)
                    .background(RoundedRectangle(cornerRadius: 12).fill(Color.orange.opacity(0.12)))
                    .listRowSeparator(.hidden)
                    .listRowBackground(Color.clear)
                    .contentTransition(.numericText())
                    .animation(.spring(response: 0.4, dampingFraction: 0.7), value: dailyStreak)
                } header: {
                    // Same fix as Build's tier headers (§21): a headerless
                    // first Section still reserves the grouped-style top
                    // inset, which just read as unexplained blank space
                    // under the nav bar. A real header gives it a job.
                    Text("Overview")
                }

                Section {
                    ActivityHeatmapView(activityByDay: activityByDay)
                        .listRowSeparator(.hidden)
                        .listRowBackground(Color.clear)
                } header: {
                    Text("Activity")
                }

                Section {
                    Toggle(isOn: remindersToggleBinding) {
                        Label("Streak reminders", systemImage: "bell.fill")
                    }
                    .tint(Theme.brand)
                    .accessibilityIdentifier("streak-reminders-toggle")
                } header: {
                    Text("Reminders")
                } footer: {
                    Text("A local notification around 8pm, only on days you haven't played yet — never if your streak is already safe.")
                }

                // Bookmarks before Achievements — the player's own
                // curated list is more immediately actionable than a
                // (growing) locked-achievement checklist, and keeping it
                // higher means it doesn't drift further down the list
                // every time a new achievement gets added.
                if !bookmarks.isEmpty {
                    Section {
                        ForEach(bookmarks, id: \.id) { concept in
                            Button {
                                onNavigateToExplore(concept.id)
                            } label: {
                                HStack(spacing: 10) {
                                    Image(systemName: "bookmark.fill")
                                        .foregroundStyle(Theme.color(forDomain: concept.domain))
                                    Text(concept.title)
                                        .foregroundStyle(.primary)
                                    Spacer()
                                    Image(systemName: "chevron.right")
                                        .font(.caption)
                                        .foregroundStyle(.tertiary)
                                }
                            }
                            .accessibilityLabel(concept.title)
                        }
                    } header: {
                        Text("Bookmarks")
                    }
                    .listRowBackground(RoundedRectangle(cornerRadius: 12).fill(.thinMaterial).padding(.vertical, 2))
                }

                Section {
                    ForEach(AchievementDefinition.all) { achievement in
                        let unlocked = celebratedIds.contains(achievement.id)
                        HStack(spacing: 12) {
                            Image(systemName: unlocked ? achievement.icon : "lock.fill")
                                .font(.title3)
                                .foregroundStyle(unlocked ? .yellow : .secondary)
                                .frame(width: 28)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(achievement.title)
                                    .foregroundStyle(unlocked ? .primary : .secondary)
                                Text(achievement.hint)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        .opacity(unlocked ? 1 : 0.6)
                    }
                } header: {
                    Text("Achievements")
                }
                .listRowBackground(RoundedRectangle(cornerRadius: 12).fill(.thinMaterial).padding(.vertical, 2))

                ForEach(domainOrder, id: \.self) { rootId in
                    if let items = conceptsByDomain[rootId], !items.isEmpty {
                        Section {
                            ForEach(items, id: \.id) { concept in
                                HStack(spacing: 10) {
                                    Circle().fill(Theme.color(forDomain: concept.domain)).frame(width: 8, height: 8)
                                    Text(concept.title)
                                    Spacer()
                                    MasteryBadge(
                                        stage: masteryById[concept.id]?.stage ?? .discovered,
                                        discovered: masteryById[concept.id]?.discovered ?? false
                                    )
                                }
                                .padding(.vertical, 2)
                            }
                        } header: {
                            Label(domainNames[rootId] ?? rootId, systemImage: "circle.fill")
                                .labelStyle(.titleOnly)
                                .foregroundStyle(Theme.color(forDomain: rootId))
                        }
                        .listRowBackground(RoundedRectangle(cornerRadius: 12).fill(.thinMaterial).padding(.vertical, 2))
                    }
                }

                Section {
                    Link(destination: URL(string: "https://intscription.github.io/bit/privacy-policy.html")!) {
                        HStack {
                            Label("Privacy Policy", systemImage: "hand.raised.fill")
                                .foregroundStyle(.primary)
                            Spacer()
                            Image(systemName: "arrow.up.right")
                                .font(.caption)
                                .foregroundStyle(.tertiary)
                        }
                    }
                    .accessibilityIdentifier("privacy-policy-link")

                    if let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String {
                        HStack {
                            Text("Version")
                                .foregroundStyle(.secondary)
                            Spacer()
                            Text(version)
                                .foregroundStyle(.secondary)
                        }
                    }
                } header: {
                    Text("About")
                }
                .listRowBackground(RoundedRectangle(cornerRadius: 12).fill(.thinMaterial).padding(.vertical, 2))
            }
            .scrollContentBackground(.hidden)
            .gameBackground()
            .navigationTitle("Profile")
            .navigationBarTitleDisplayMode(.inline)
            .onAppear(perform: load)
        }
    }

    private var discoveredCount: Int {
        masteryById.values.filter(\.discovered).count
    }

    private var masteredCount: Int {
        masteryById.values.filter { $0.stage == .mastered }.count
    }

    /// Grouped by top-level domain rather than dumped in one flat list —
    /// a real fix, not just polish: the old version labeled every
    /// concept's row "Foundations" regardless of which domain it
    /// actually belonged to.
    private var conceptsByDomain: [String: [FfiConceptSummary]] {
        Dictionary(grouping: concepts) { Theme.rootDomainId(of: $0.domain) }
            .mapValues { $0.sorted { $0.title < $1.title } }
    }

    /// A plain `Binding<Bool>` (not just `$remindersEnabled`) because
    /// turning it on has to go through the async system permission
    /// prompt first — the visible toggle state should reflect what the OS
    /// actually granted, not optimistically flip before that's known.
    private var remindersToggleBinding: Binding<Bool> {
        Binding(
            get: { remindersEnabled },
            set: { newValue in
                if newValue {
                    StreakReminders.enable { granted in
                        remindersEnabled = granted
                        engine.refreshStreakReminder()
                    }
                } else {
                    StreakReminders.disable()
                    remindersEnabled = false
                }
            }
        )
    }

    private func load() {
        concepts = engine.ffi.allConcepts()
        var map: [String: FfiMastery] = [:]
        for concept in concepts {
            map[concept.id] = engine.ffi.mastery(conceptId: concept.id)
        }
        masteryById = map

        let roots = engine.ffi.rootDomains()
        domainNames = Dictionary(uniqueKeysWithValues: roots.map { ($0.id, $0.name) })
        domainOrder = roots.map(\.id)

        dailyStreak = engine.ffi.dailyStreak()
        longestDailyStreak = engine.ffi.longestDailyStreak()

        activityByDay = Dictionary(uniqueKeysWithValues: engine.ffi.activityHistory().map { ($0.day, $0.count) })

        bookmarks = engine.ffi.bookmarkedConcepts()
        celebratedIds = Set(AchievementDefinition.all.map(\.id).filter { engine.ffi.hasCelebrated(id: $0) })
    }
}

/// Mirrors the milestone ids Play and Build already mark via
/// `BitEngine::mark_celebrated` — a plain list here rather than a new
/// Rust type, since Rust only ever needs to store/check an opaque id
/// string (see that method's doc comment) and never needs to know the
/// achievement's copy.
struct AchievementDefinition: Identifiable {
    let id: String
    let icon: String
    let title: String
    let hint: String

    static let all: [AchievementDefinition] = [
        AchievementDefinition(id: "first_mastery", icon: "star.circle.fill", title: "First Concept Mastered", hint: "Master any concept in Play."),
        AchievementDefinition(id: "100_correct", icon: "100.circle.fill", title: "100 Correct Answers", hint: "Answer 100 puzzles correctly in Play."),
        AchievementDefinition(id: "first_build", icon: "hammer.circle.fill", title: "First Build Complete", hint: "Finish a real Build objective."),
        AchievementDefinition(id: "10_discovered", icon: "eye.circle.fill", title: "10 Concepts Discovered", hint: "Discover ten concepts in Explore."),
        AchievementDefinition(id: "7_day_streak", icon: "calendar.circle.fill", title: "7 Day Streak", hint: "Play on 7 consecutive calendar days."),
    ]
}

/// A small colored summary tile — big animated number, an icon, and a
/// fixed label (kept as a distinct `Text`, not folded into a `Label`,
/// so it stays independently queryable — see `BitAppUITests`).
private struct StatCard: View {
    let icon: String
    let value: Int
    var of: Int? = nil
    let label: String
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Image(systemName: icon)
                .foregroundStyle(color)
            HStack(alignment: .firstTextBaseline, spacing: 2) {
                Text("\(value)")
                    .font(.system(size: 26, weight: .bold, design: .rounded))
                    .contentTransition(.numericText())
                if let of {
                    Text("/ \(of)")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
            }
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 14).fill(color.opacity(0.12)))
        .animation(.spring(response: 0.4, dampingFraction: 0.7), value: value)
    }
}

/// Discovery/practicing/proficient/mastered, each with its own color and
/// icon — same four-stage vocabulary as `bit_progression::MasteryStage`,
/// just given a visual identity instead of plain text.
private struct MasteryBadge: View {
    let stage: FfiMasteryStage
    let discovered: Bool

    private var label: String {
        guard discovered else { return "Not yet discovered" }
        switch stage {
        case .discovered: return "Discovered"
        case .practicing: return "Practicing"
        case .proficient: return "Proficient"
        case .mastered: return "Mastered"
        }
    }

    private var color: Color {
        guard discovered else { return .secondary }
        switch stage {
        case .discovered: return .gray
        case .practicing: return .blue
        case .proficient: return .orange
        case .mastered: return .green
        }
    }

    private var icon: String {
        guard discovered else { return "circle.dotted" }
        switch stage {
        case .discovered: return "eye.fill"
        case .practicing: return "flame.fill"
        case .proficient: return "star.leadinghalf.filled"
        case .mastered: return "star.fill"
        }
    }

    var body: some View {
        Label(label, systemImage: icon)
            .font(.caption2.weight(.semibold))
            .foregroundStyle(color)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Capsule().fill(color.opacity(0.15)))
    }
}

/// A GitHub-contributions-style grid: one column per calendar week
/// (Sunday on top, Saturday on the bottom), darker orange for more
/// engagement events that day. Purely a "did you show up" picture —
/// deliberately not scored or comparable to anyone else's, matching this
/// screen's no-school-grade philosophy.
private struct ActivityHeatmapView: View {
    let activityByDay: [String: UInt32]
    var weeks: Int = 12

    private static let dayFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        formatter.timeZone = .current
        formatter.locale = Locale(identifier: "en_US_POSIX")
        return formatter
    }()

    /// Sparse on purpose (Mon/Wed/Fri only) — labeling all 7 rows next
    /// to 13pt cells is just noise; GitHub's own contribution graph
    /// uses the same every-other-row convention.
    private static let weekdayLabels: [Int: String] = [1: "M", 3: "W", 5: "F"]

    /// `weeks` columns, oldest first, each holding 7 Sunday->Saturday
    /// entries. Days beyond today (padding out the current, still-
    /// incomplete week) come back `nil` and render as empty space.
    private var weeksGrid: [[Date?]] {
        let calendar = Calendar.current
        let today = calendar.startOfDay(for: Date())
        let weekday = calendar.component(.weekday, from: today) // 1 = Sunday ... 7 = Saturday
        guard let startOfThisWeek = calendar.date(byAdding: .day, value: -(weekday - 1), to: today),
              let firstDay = calendar.date(byAdding: .day, value: -(weeks - 1) * 7, to: startOfThisWeek)
        else { return [] }

        let allDays: [Date?] = (0..<(weeks * 7)).map { offset in
            guard let day = calendar.date(byAdding: .day, value: offset, to: firstDay) else { return nil }
            return day > today ? nil : day
        }
        return stride(from: 0, to: allDays.count, by: 7).map { Array(allDays[$0..<min($0 + 7, allDays.count)]) }
    }

    private func count(on date: Date?) -> UInt32 {
        guard let date else { return 0 }
        return activityByDay[Self.dayFormatter.string(from: date)] ?? 0
    }

    private func color(for count: UInt32) -> Color {
        switch count {
        case 0: return Color.secondary.opacity(0.12)
        case 1: return Theme.brand.opacity(0.35)
        case 2...3: return Theme.brand.opacity(0.65)
        default: return Theme.brand
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(alignment: .top, spacing: 4) {
                    VStack(spacing: 4) {
                        ForEach(0..<7, id: \.self) { row in
                            Text(Self.weekdayLabels[row] ?? "")
                                .font(.system(size: 9))
                                .foregroundStyle(.secondary)
                                .frame(width: 12, height: 13)
                        }
                    }

                    ForEach(Array(weeksGrid.enumerated()), id: \.offset) { _, column in
                        VStack(spacing: 4) {
                            ForEach(Array(column.enumerated()), id: \.offset) { _, date in
                                RoundedRectangle(cornerRadius: 3)
                                    .fill(date == nil ? Color.clear : color(for: count(on: date)))
                                    .frame(width: 13, height: 13)
                            }
                        }
                    }
                }
                .padding(.vertical, 2)
            }

            HStack(spacing: 5) {
                Spacer()
                Text("Less").font(.caption2).foregroundStyle(.secondary)
                ForEach([0, 1, 2, 4], id: \.self) { level in
                    RoundedRectangle(cornerRadius: 3)
                        .fill(color(for: UInt32(level)))
                        .frame(width: 11, height: 11)
                }
                Text("More").font(.caption2).foregroundStyle(.secondary)
            }
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 16).fill(Theme.brand.opacity(0.07)))
        .accessibilityIdentifier("profile-activity-heatmap")
    }
}
