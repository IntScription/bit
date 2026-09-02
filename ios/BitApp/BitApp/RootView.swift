import SwiftUI

/// Owns cross-tab navigation state so Play's "ready for something new?"
/// prompt can jump straight to that concept's page in Explore, not just
/// switch tabs and leave the player to hunt for it.
struct RootView: View {
    @State private var selectedTab: Tab = .play
    @State private var explorePath = NavigationPath()

    enum Tab {
        case play, build, explore, lab, profile
    }

    var body: some View {
        TabView(selection: $selectedTab) {
            PlayView(onNavigateToExplore: navigateToExplore)
                .tabItem { Label("Play", systemImage: "play.fill") }
                .tag(Tab.play)
            BuildView()
                .tabItem { Label("Build", systemImage: "hammer.fill") }
                .tag(Tab.build)
            ExploreView(path: $explorePath)
                .tabItem { Label("Explore", systemImage: "square.stack.3d.up.fill") }
                .tag(Tab.explore)
            LabView()
                .tabItem { Label("Lab", systemImage: "flask.fill") }
                .tag(Tab.lab)
            ProfileView(onNavigateToExplore: navigateToExplore)
                .tabItem { Label("Profile", systemImage: "person.crop.circle.fill") }
                .tag(Tab.profile)
        }
        // The app's one signature color, cascading everywhere nothing
        // more specific (a domain, a build tier) overrides it locally.
        .tint(Theme.brand)
    }

    /// `conceptId` nil means "just switch to Explore" (the empty-pool
    /// state's "Go to Explore" button); non-nil pushes straight to that
    /// concept's page (the new-topic-suggestion sheet's "Learn" button).
    private func navigateToExplore(conceptId: String?) {
        if let conceptId {
            explorePath = NavigationPath([conceptId])
        }
        selectedTab = .explore
    }
}
