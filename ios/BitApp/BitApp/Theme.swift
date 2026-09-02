import SwiftUI

/// One accent color per top-level domain — Foundations looks and feels
/// different from Data Representation, which looks different from
/// whatever domain comes after it, the way Infinity Loop shifts its
/// palette per level. Kept strictly cosmetic and separate from semantic
/// color (green for "objective met," the switch on/off fill) — a theme
/// color never carries meaning on its own, so there's nothing to
/// confuse.
///
/// Content grows without a rebuild (see /ARCHITECTURE.md's "endless
/// exploration" premise), so this can't be a hardcoded switch over known
/// domain ids alone: the first several domains get a hand-picked color
/// from `palette`, and any domain beyond that falls back to a stable
/// hash into the same palette — stable across launches (unlike Swift's
/// `Hashable`, whose hash seed is randomized per process), so a given
/// domain always looks the same color every time you visit it.
enum Theme {
    private static let palette: [Color] = [
        Color(red: 0.20, green: 0.50, blue: 0.95), // blue
        Color(red: 0.86, green: 0.30, blue: 0.55), // magenta
        Color(red: 0.20, green: 0.72, blue: 0.55), // green
        Color(red: 0.95, green: 0.58, blue: 0.15), // orange
        Color(red: 0.56, green: 0.40, blue: 0.92), // purple
        Color(red: 0.15, green: 0.72, blue: 0.85), // cyan
        Color(red: 0.90, green: 0.40, blue: 0.30), // red-orange
        Color(red: 0.45, green: 0.65, blue: 0.20), // olive
    ]

    private static let knownRoots: [String: Int] = [
        "foundations": 0,
        "data_representation": 1,
    ]

    /// A domain id like "foundations.bits_binary" belongs to the
    /// top-level domain "foundations" — themes are assigned per
    /// top-level domain, not per subdomain, so drilling deeper within
    /// Data Representation doesn't shift the palette again.
    static func rootDomainId(of domainId: String) -> String {
        domainId.split(separator: ".").first.map(String.init) ?? domainId
    }

    static func color(forDomain domainId: String) -> Color {
        let root = rootDomainId(of: domainId)
        if let index = knownRoots[root] {
            return palette[index]
        }
        return palette[stableHash(root) % palette.count]
    }

    /// FNV-1a: deterministic across launches, unlike `Hasher`.
    private static func stableHash(_ s: String) -> Int {
        var hash: UInt64 = 0xcbf29ce484222325
        for byte in s.utf8 {
            hash ^= UInt64(byte)
            hash = hash &* 0x100000001b3
        }
        return Int(hash % UInt64(Int.max))
    }

    // MARK: - App-wide identity

    /// The app's one signature color — a saturated electric violet with
    /// a retro-arcade-neon pitch to it, not the default system blue
    /// every stock-List app already uses and not quite the softer
    /// purple/magenta already spoken for in `palette` below (this sits
    /// between them, more saturated than either, so it reads as its own
    /// identity rather than "one more domain color"). Set as the
    /// app-wide `.tint()` at `RootView`; any screen that also tints
    /// itself per-domain or per-tier overrides this locally, so this is
    /// only ever visible where nothing more specific applies (the tab
    /// bar, Build's plain chrome, Profile, the splash screen's title).
    static let brand = Color(
        uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(red: 0.72, green: 0.36, blue: 1.0, alpha: 1)
                : UIColor(red: 0.52, green: 0.12, blue: 0.88, alpha: 1)
        }
    )

    private static let backgroundTop = Color(
        uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(red: 0.09, green: 0.09, blue: 0.15, alpha: 1)
                : UIColor(red: 0.96, green: 0.96, blue: 1.0, alpha: 1)
        }
    )

    private static let backgroundBottom = Color(
        uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(red: 0.05, green: 0.06, blue: 0.10, alpha: 1)
                : UIColor(red: 0.92, green: 0.94, blue: 1.0, alpha: 1)
        }
    )

    /// The gradient every `gameBackground()` paints — a top-to-bottom
    /// soft tint instead of a flat system white/black, so the whole app
    /// reads as one designed surface rather than a stock list app.
    static var backgroundGradient: LinearGradient {
        LinearGradient(colors: [backgroundTop, backgroundBottom], startPoint: .top, endPoint: .bottom)
    }
}

extension View {
    /// Applied once per tab root (and per pushed screen) in place of
    /// the default flat white/black — see `Theme.backgroundGradient`.
    func gameBackground() -> some View {
        background(Theme.backgroundGradient.ignoresSafeArea())
    }
}
