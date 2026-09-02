import SwiftUI

/// Mirrors `bit_knowledge::Concept`'s JSON shape (see `concept_detail_json`
/// in bit-ffi). Only fields the UI actually renders are declared —
/// `Decodable` ignores the rest, so this can lag the full Rust struct
/// without breaking.
struct ConceptDetail: Decodable {
    let title: String
    let domain: String
    let difficulty: String
    let learningObjectives: [String]
    let realWorldUses: [String]
    let historicalRelevance: String?
    let interactiveDemo: InteractiveDemoInfo?

    enum CodingKeys: String, CodingKey {
        case title, domain, difficulty
        case learningObjectives = "learning_objectives"
        case realWorldUses = "real_world_uses"
        case historicalRelevance = "historical_relevance"
        case interactiveDemo = "interactive_demo"
    }
}

/// The row-morphs-into-detail-page transition (iOS 18's `.zoom`
/// navigation transition) — degrades to the plain push transition on
/// anything older, since `.navigationTransition(.zoom)` doesn't exist
/// before iOS 18.
extension View {
    @ViewBuilder
    func zoomSource(id: String, in namespace: Namespace.ID) -> some View {
        if #available(iOS 18.0, *) {
            matchedTransitionSource(id: id, in: namespace)
        } else {
            self
        }
    }

    @ViewBuilder
    func zoomDestination(id: String, in namespace: Namespace.ID) -> some View {
        if #available(iOS 18.0, *) {
            navigationTransition(.zoom(sourceID: id, in: namespace))
        } else {
            self
        }
    }
}

/// Unwraps the optional zoom id/namespace pair — `ConceptDetailView` is
/// also reached from places (Related/Prerequisites links, search
/// results) that don't carry a matching source, so this degrades to a
/// plain push when either is nil rather than requiring every call site
/// to branch.
private struct ZoomDestinationModifier: ViewModifier {
    let zoomId: String?
    let zoomNamespace: Namespace.ID?

    func body(content: Content) -> some View {
        if let zoomId, let zoomNamespace {
            content.zoomDestination(id: zoomId, in: zoomNamespace)
        } else {
            content
        }
    }
}

struct ExploreView: View {
    /// Owned by `RootView` — lets Play's "ready for something new?"
    /// prompt push straight to a concept's page (`RootView.
    /// navigateToConceptInExplore`) instead of just switching tabs and
    /// leaving the player to find it themselves.
    @Binding var path: NavigationPath
    @State private var query = ""

    var body: some View {
        NavigationStack(path: $path) {
            Group {
                if query.isEmpty {
                    DomainListView(
                        domainId: nil,
                        title: "Computer Science",
                        description: "Every concept in BIT, organized by topic — tap a folder to start exploring."
                    )
                } else {
                    SearchResultsView(query: query)
                }
            }
            .navigationDestination(for: String.self) { conceptId in
                ConceptDetailView(conceptId: conceptId)
            }
        }
        .searchable(text: $query, prompt: "Search concepts")
    }
}

/// A brief full-screen color flash in the domain's theme color, fading
/// out on appear — the "you just stepped into a new area" moment,
/// deliberately reserved for top-level domain entry only (see call site).
struct DomainWashOverlay: View {
    let color: Color
    @State private var opacity: Double = 0.35

    var body: some View {
        color
            .opacity(opacity)
            .ignoresSafeArea()
            .allowsHitTesting(false)
            .onAppear {
                withAnimation(.easeOut(duration: 0.45)) {
                    opacity = 0
                }
            }
    }
}

struct SearchResultsView: View {
    @EnvironmentObject var engine: Engine
    let query: String
    @State private var hits: [FfiSearchHit] = []

    var body: some View {
        List(hits, id: \.conceptId) { hit in
            NavigationLink(hit.conceptId) {
                ConceptDetailView(conceptId: hit.conceptId)
            }
        }
        .scrollContentBackground(.hidden)
        .gameBackground()
        .navigationTitle("Search")
        .navigationBarTitleDisplayMode(.inline)
        .onChange(of: query, initial: true) { _, newValue in
            hits = engine.ffi.search(query: newValue)
        }
    }
}

/// Recursive: pushing a folder just presents another `DomainListView`,
/// which is what gives Explore its filesystem feel without an artificial
/// depth limit (see /ARCHITECTURE.md §6).
struct DomainListView: View {
    @EnvironmentObject var engine: Engine
    let domainId: String?
    let title: String
    var description: String = ""

    @State private var children: [FfiDomain] = []
    @State private var concepts: [FfiConceptSummary] = []
    @Namespace private var zoomNamespace

    var body: some View {
        List {
            if !description.isEmpty {
                Text(description)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .listRowSeparator(.hidden)
            }
            if !children.isEmpty {
                Section("Folders") {
                    ForEach(children, id: \.id) { domain in
                        NavigationLink {
                            DomainListView(domainId: domain.id, title: domain.name, description: domain.description)
                        } label: {
                            HStack(spacing: 10) {
                                // A swatch preview of the domain's theme
                                // color, visible before drilling in — see
                                // Theme.swift for why it's domain-scoped.
                                Circle().fill(Theme.color(forDomain: domain.id)).frame(width: 12, height: 12)
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(domain.name)
                                    if !domain.description.isEmpty {
                                        Text(domain.description)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                            .lineLimit(2)
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !concepts.isEmpty {
                Section("Concepts") {
                    ForEach(concepts, id: \.id) { concept in
                        NavigationLink {
                            ConceptDetailView(conceptId: concept.id, zoomId: concept.id, zoomNamespace: zoomNamespace)
                        } label: {
                            Text(concept.title)
                        }
                        .zoomSource(id: concept.id, in: zoomNamespace)
                    }
                }
            }
        }
        .scrollContentBackground(.hidden)
        .gameBackground()
        .navigationTitle(title)
        .navigationBarTitleDisplayMode(.inline)
        .tint(domainId.map { Theme.color(forDomain: $0) })
        // The Infinity-Loop-style wash: only on *entering* a top-level
        // domain (no "." in the id), not on every subdomain drill-down —
        // this is the moment the palette actually changes, so it's the
        // only moment that earns a flourish.
        .overlay {
            if let domainId, !domainId.contains(".") {
                DomainWashOverlay(color: Theme.color(forDomain: domainId))
            }
        }
        .onAppear(perform: load)
    }

    private func load() {
        if let domainId {
            children = engine.ffi.domainChildren(domainId: domainId)
            concepts = engine.ffi.conceptsInDomain(domainId: domainId)
        } else {
            children = engine.ffi.rootDomains()
        }
    }
}

struct ConceptDetailView: View {
    @EnvironmentObject var engine: Engine
    let conceptId: String
    /// Set only when reached from a row that declared a matching
    /// `.zoomSource` — links from Related/Prerequisites sections (which
    /// construct a fresh `ConceptDetailView` recursively, not from a
    /// namespaced list row) leave this nil and just get a plain push.
    var zoomId: String? = nil
    var zoomNamespace: Namespace.ID? = nil

    @State private var detail: ConceptDetail?
    @State private var mastery: FfiMastery?
    @State private var related: [FfiConceptSummary] = []
    @State private var prerequisites: [FfiConceptSummary] = []
    @State private var isBookmarked = false
    @State private var noteText = ""
    @FocusState private var noteFieldFocused: Bool

    var body: some View {
        ScrollView {
            if let detail {
                // No repeated heading here — the nav bar's inline title
                // (set below) already shows the concept name centered
                // beside the back button, so the content starts right at
                // the difficulty badge instead of a second, redundant
                // large title pushing everything down.
                VStack(alignment: .leading, spacing: 20) {
                    Text(detail.difficulty.uppercased())
                        .font(.caption.bold())
                        .foregroundStyle(Theme.color(forDomain: detail.domain))
                        .padding(.horizontal, 10)
                        .padding(.vertical, 4)
                        .background(Theme.color(forDomain: detail.domain).opacity(0.15), in: Capsule())

                    // INTERACT before EXPLAIN (spec §4) — the demo comes
                    // before any prose section, not after.
                    if let demo = detail.interactiveDemo {
                        DemoContainerView(demo: demo)
                    }

                    if !prerequisites.isEmpty {
                        section("Prerequisites", icon: "arrow.triangle.branch", color: .blue) {
                            ForEach(prerequisites, id: \.id) { c in
                                NavigationLink(c.title) { ConceptDetailView(conceptId: c.id) }
                            }
                        }
                    }

                    if !detail.learningObjectives.isEmpty {
                        section("You'll learn to", icon: "graduationcap.fill", color: .indigo) {
                            ForEach(detail.learningObjectives, id: \.self) { Text("• \($0)") }
                        }
                    }

                    if !detail.realWorldUses.isEmpty {
                        section("Real world", icon: "globe", color: .orange) {
                            ForEach(detail.realWorldUses, id: \.self) { Text("• \($0)") }
                        }
                    }

                    if !related.isEmpty {
                        section("Related", icon: "link", color: .purple) {
                            ForEach(related, id: \.id) { c in
                                NavigationLink(c.title) { ConceptDetailView(conceptId: c.id) }
                            }
                        }
                    }

                    if let mastery {
                        section("Your progress", icon: "chart.bar.fill", color: Theme.color(forDomain: detail.domain)) {
                            Text("Stage: \(describe(mastery.stage))")
                            Text("Practiced \(mastery.practicedCount) times, \(Int(mastery.accuracy * 100))% correct")
                        }
                    }

                    // The "creation" pillar (spec §2's PROFILE section) —
                    // a place to write your own explanation, not just
                    // read the author's. Saves on losing focus, not per
                    // keystroke, so typing never triggers a disk write.
                    section("Your notes", icon: "note.text", color: .brown) {
                        TextEditor(text: $noteText)
                            .frame(minHeight: 90)
                            .padding(6)
                            .background(RoundedRectangle(cornerRadius: 10).fill(Color.gray.opacity(0.08)))
                            .overlay(alignment: .topLeading) {
                                if noteText.isEmpty {
                                    Text("Write something to remember about this concept...")
                                        .foregroundStyle(.secondary)
                                        .padding(.top, 14)
                                        .padding(.leading, 11)
                                        .allowsHitTesting(false)
                                }
                            }
                            .focused($noteFieldFocused)
                            .accessibilityIdentifier("concept-notes-editor")
                            .onChange(of: noteFieldFocused) { _, focused in
                                if !focused {
                                    engine.setNote(conceptId: conceptId, text: noteText)
                                }
                            }
                    }
                }
                .padding()
            } else {
                ProgressView().padding()
            }
        }
        .gameBackground()
        .navigationTitle(detail?.title ?? "")
        .navigationBarTitleDisplayMode(.inline)
        .tint(detail.map { Theme.color(forDomain: $0.domain) })
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    Haptics.toggle()
                    engine.toggleBookmark(conceptId: conceptId)
                    isBookmarked.toggle()
                } label: {
                    Image(systemName: isBookmarked ? "bookmark.fill" : "bookmark")
                }
                .symbolEffect(.bounce, value: isBookmarked)
                .accessibilityIdentifier("bookmark-toggle")
                .accessibilityLabel(isBookmarked ? "Remove bookmark" : "Add bookmark")
            }
        }
        .modifier(ZoomDestinationModifier(zoomId: zoomId, zoomNamespace: zoomNamespace))
        .onAppear(perform: load)
        .onDisappear {
            // Belt and suspenders alongside the focus-lost save above —
            // navigating away can dismiss the editor without ever
            // toggling focus false first.
            engine.setNote(conceptId: conceptId, text: noteText)
        }
    }

    @ViewBuilder
    /// Every prose section is its own floating card with a colored icon
    /// header, instead of plain stacked text — this is the single
    /// biggest lever on how "plain" a concept page reads, short of
    /// hand-illustrating each one.
    private func section<Content: View>(
        _ title: String, icon: String, color: Color = Theme.brand, @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(title, systemImage: icon)
                .font(.subheadline.weight(.bold))
                .foregroundStyle(color)
            content()
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 16).fill(.thinMaterial))
    }

    private func describe(_ stage: FfiMasteryStage) -> String {
        switch stage {
        case .discovered: return "Discovered"
        case .practicing: return "Practicing"
        case .proficient: return "Proficient"
        case .mastered: return "Mastered"
        }
    }

    private func load() {
        engine.recordDiscovery(conceptId: conceptId)
        if let json = engine.ffi.conceptDetailJson(conceptId: conceptId), let data = json.data(using: .utf8) {
            detail = try? JSONDecoder().decode(ConceptDetail.self, from: data)
        }
        mastery = engine.ffi.mastery(conceptId: conceptId)
        related = engine.ffi.related(conceptId: conceptId)
        prerequisites = engine.ffi.prerequisites(conceptId: conceptId)
        isBookmarked = engine.ffi.isBookmarked(conceptId: conceptId)
        noteText = engine.ffi.noteFor(conceptId: conceptId) ?? ""
    }
}
