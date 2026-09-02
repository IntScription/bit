import SwiftUI

@main
struct BitApp: App {
    @StateObject private var engine = Engine()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            LaunchView()
                .environmentObject(engine)
        }
        .onChange(of: scenePhase) { _, newPhase in
            if newPhase == .background {
                engine.persist()
            } else if newPhase == .active {
                // Catches the calendar day rolling over while the app was
                // suspended — persist() already reschedules on every
                // mutation, but nothing mutates on a plain foreground.
                engine.refreshStreakReminder()
            }
        }
    }
}
