import SwiftUI

/// A build's tier carries a color and icon identity the same way a
/// domain carries a `Theme` color — cosmetic only, but it turns a flat
/// list of names into something that reads as "these four are
/// different weights" at a glance, before ever reading the tier label.
extension FfiBuildTier {
    var accentColor: Color {
        switch self {
        case .beginner: return .green
        case .core: return .blue
        case .advanced: return .purple
        case .extreme: return .red
        }
    }

    var icon: String {
        switch self {
        case .beginner: return "leaf.fill"
        case .core: return "cube.fill"
        case .advanced: return "bolt.fill"
        case .extreme: return "flame.fill"
        }
    }

    var label: String {
        switch self {
        case .beginner: return "BEGINNER"
        case .core: return "CORE"
        case .advanced: return "ADVANCED"
        case .extreme: return "EXTREME"
        }
    }
}

struct BuildView: View {
    @EnvironmentObject var engine: Engine
    @State private var builds: [FfiBuildSummary] = []
    @Namespace private var zoomNamespace

    /// Grouped by tier in a fixed Beginner -> Extreme order (only tiers
    /// that actually have a build appear) — a tier header immediately
    /// under the nav bar gives that space a job, instead of it reading
    /// as unexplained empty padding above a flat list.
    private var buildsByTier: [(tier: FfiBuildTier, builds: [FfiBuildSummary])] {
        let order: [FfiBuildTier] = [.beginner, .core, .advanced, .extreme]
        return order.compactMap { tier in
            let matching = builds.filter { $0.tier == tier }
            return matching.isEmpty ? nil : (tier, matching)
        }
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    HStack(alignment: .top, spacing: 10) {
                        Image(systemName: "hammer.fill")
                            .foregroundStyle(Theme.brand)
                        Text("Turn what you learned in Play into something real — each project has an actual target to hit, using the exact mechanic you already practiced.")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                    .padding(12)
                    .background(RoundedRectangle(cornerRadius: 14).fill(.thinMaterial))
                } header: {
                    // Same fix as Profile's headerless-first-Section gap
                    // (§23/§25): without a real header, a grouped-style
                    // List still reserves its top inset, which read as
                    // unexplained blank space under the nav bar.
                    Text("Projects")
                }
                .listRowSeparator(.hidden)
                .listRowBackground(Color.clear)

                ForEach(buildsByTier, id: \.tier.label) { group in
                    Section {
                        ForEach(group.builds, id: \.id) { build in
                            NavigationLink {
                                BuildSessionView(build: build)
                                    .zoomDestination(id: build.id, in: zoomNamespace)
                            } label: {
                                HStack(spacing: 14) {
                                    ZStack {
                                        Circle()
                                            .fill(build.tier.accentColor.opacity(0.16))
                                            .frame(width: 44, height: 44)
                                        Image(systemName: build.tier.icon)
                                            .font(.system(size: 17, weight: .semibold))
                                            .foregroundStyle(build.tier.accentColor)
                                    }

                                    VStack(alignment: .leading, spacing: 4) {
                                        Text(build.name).font(.headline)
                                        Text(build.description)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                            .lineLimit(2)
                                    }

                                    Spacer(minLength: 8)
                                }
                                .padding(.vertical, 4)
                            }
                            .zoomSource(id: build.id, in: zoomNamespace)
                        }
                    } header: {
                        Label(group.tier.label.capitalized, systemImage: group.tier.icon)
                            .foregroundStyle(group.tier.accentColor)
                    }
                }
            }
            .scrollContentBackground(.hidden)
            .gameBackground()
            .navigationTitle("Build")
            .navigationBarTitleDisplayMode(.inline)
            .onAppear { builds = engine.ffi.listBuilds() }
        }
    }
}

/// Drives one build session to completion. Works for every simulation
/// kind the app ships (`BinaryCircuitSim`, `PixelGridSim`, `ArraySim`) by
/// branching once on `build.id` — see bit-build's `catalog` doc comment
/// for why that branch lives in exactly one place per layer instead of
/// being made fully generic.
struct BuildSessionView: View {
    @EnvironmentObject var engine: Engine
    let build: FfiBuildSummary

    @State private var sessionId: UInt64?
    @State private var circuitState: FfiCircuitState?
    @State private var pixelState: FfiPixelState?
    @State private var arrayState: FfiArrayState?
    @State private var status: FfiObjectiveStatus = .inProgress
    @State private var celebratingFirstBuild: MilestoneInfo?
    @State private var relatedConcepts: [FfiConceptSummary] = []
    private var isPixel: Bool { build.id == "pixel_builder" }
    private var isArray: Bool { build.id == "array_builder" }

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // The full project brief — tier, what it's for, and
                // exactly which concepts it's putting into practice —
                // not just a bare interactive widget with no context
                // once you're here.
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        Label(build.tier.label.capitalized, systemImage: build.tier.icon)
                            .font(.caption.weight(.bold))
                            .foregroundStyle(build.tier.accentColor)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 4)
                            .background(Capsule().fill(build.tier.accentColor.opacity(0.15)))
                        Spacer()
                    }

                    Text(build.description)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)

                    if !relatedConcepts.isEmpty {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("PRACTICES")
                                .font(.caption2.weight(.bold))
                                .foregroundStyle(.tertiary)
                            ForEach(relatedConcepts, id: \.id) { concept in
                                NavigationLink {
                                    ConceptDetailView(conceptId: concept.id)
                                } label: {
                                    HStack(spacing: 8) {
                                        Circle().fill(Theme.color(forDomain: concept.domain)).frame(width: 7, height: 7)
                                        Text(concept.title)
                                            .foregroundStyle(.primary)
                                        Spacer()
                                        Image(systemName: "chevron.right")
                                            .font(.caption2)
                                            .foregroundStyle(.tertiary)
                                    }
                                }
                            }
                        }
                        .padding(.top, 4)
                    }
                }
                .padding()
                .background(RoundedRectangle(cornerRadius: 16).fill(.thinMaterial))

                ObjectiveBanner(status: status)

                if isPixel {
                    if let pixelState {
                        PixelGridView(state: pixelState) { x, y in
                            guard let sessionId else { return }
                            _ = try? engine.ffi.applyPixelAction(sessionId: sessionId, action: .toggle(x: x, y: y))
                            refresh()
                        }
                    } else {
                        ProgressView()
                    }
                } else if isArray {
                    if let arrayState {
                        ArraySlotsView(state: arrayState) { index, value in
                            guard let sessionId else { return }
                            _ = try? engine.ffi.applyArrayAction(sessionId: sessionId, action: .set(index: index, value: value))
                            refresh()
                        }
                    } else {
                        ProgressView()
                    }
                } else if let circuitState {
                    SwitchesView(state: circuitState) { index in
                        guard let sessionId else { return }
                        _ = try? engine.ffi.applyCircuitAction(sessionId: sessionId, action: .toggle(index: index))
                        refresh()
                    }
                } else {
                    ProgressView()
                }
            }
            .padding()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .gameBackground()
        .tint(build.tier.accentColor)
        .navigationTitle(build.name)
        .navigationBarTitleDisplayMode(.inline)
        .onAppear(perform: start)
        .onDisappear {
            if let sessionId { engine.ffi.endBuild(sessionId: sessionId) }
        }
        .sheet(item: $celebratingFirstBuild) { milestone in
            MilestoneCelebrationSheet(milestone: milestone) {
                celebratingFirstBuild = nil
            }
        }
    }

    private func start() {
        relatedConcepts = engine.ffi.buildRelatedConcepts(buildId: build.id)
        guard let id = try? engine.ffi.startBuild(buildId: build.id) else { return }
        sessionId = id
        refresh()
    }

    private func refresh() {
        guard let sessionId else { return }
        let previousStatus = status
        status = (try? engine.ffi.objectiveStatus(sessionId: sessionId)) ?? .inProgress
        if isPixel {
            pixelState = try? engine.ffi.pixelState(sessionId: sessionId)
        } else if isArray {
            arrayState = try? engine.ffi.arrayState(sessionId: sessionId)
        } else {
            circuitState = try? engine.ffi.circuitState(sessionId: sessionId)
        }

        // Freeform builds report `.met` unconditionally (there's no real
        // objective to complete), so only a genuinely-scored build's
        // fresh transition into `.met` counts as "finished a build."
        if !build.freeform, previousStatus != .met, status == .met, !engine.ffi.hasCelebrated(id: "first_build") {
            engine.markCelebrated(id: "first_build")
            celebratingFirstBuild = MilestoneInfo(
                id: "first_build",
                icon: "hammer.circle.fill",
                title: "First Build Complete!",
                message: "\(build.name) matches the target. That's your first finished build.",
                color: build.tier.accentColor
            )
        }
    }
}
