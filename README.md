# BIT

[![CI](https://github.com/IntScription/bit/actions/workflows/ci.yml/badge.svg)](https://github.com/IntScription/bit/actions/workflows/ci.yml)

An offline-first, endless computer science learning game for iOS. A
Rust engine drives everything — content, spaced-repetition-style
review, challenge generation, progress — behind a native SwiftUI
shell, bridged through [UniFFI](https://mozilla.github.io/uniffi-rs/).

No ads, no accounts, no analytics, no network requests. Everything
runs and stays entirely on-device.

## What it is

- **Play** — an endless stream of puzzles drawn from what you've
  learned, weighted toward what you're starting to forget
- **Explore** — every concept, organized with prerequisites, learning
  objectives, worked examples, and real-world uses
- **Build** — hands-on projects (a binary counter, an array builder,
  a digital lock) where you build the real thing, not just read about it
- **Lab** — interactive playgrounds for experimenting freely
- **Puzzle of the Day** — one challenge, date-seeded, the same for
  every install, with a shareable result
- **Profile** — discovery/mastery tracking, streaks, bookmarks, notes,
  achievements — never a school grade

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full, narratively
documented design history — every phase of this project, in order,
with the reasoning behind each decision.

## Project layout

```
crates/           Rust workspace — the actual engine
  bit-knowledge/   Concept graph, domains, prerequisites
  bit-content/     Loads RON + Markdown content into the graph
  bit-challenge/   Deterministic, seeded puzzle generators
  bit-simulation/  Interactive demo state machines (registers, gates, arrays...)
  bit-build/       Hands-on Build project definitions
  bit-progression/ Mastery stages, review timing, gating rules
  bit-persistence/ Versioned local save state
  bit-core/        Ties every crate together into one engine
  bit-ffi/         UniFFI bridge exposed to Swift
tools/
  content-lint/    Validates all content loads and every build instantiates
content/           All concepts, domains, and builds — RON + Markdown, data-driven
ios/
  BitApp/          The SwiftUI app (and BitWidgetExtension, the home screen widget)
  Generated/       UniFFI-generated Swift bindings (regenerated, not hand-edited)
scripts/           Build/binding-generation/test-running scripts
docs/              Support + privacy policy pages (served via GitHub Pages)
app-store/         App Store Connect metadata and screenshots
```

## Building

Requires Xcode and a Rust toolchain.

```bash
# Regenerate the Swift FFI bindings after any Rust change
./scripts/generate-swift-bindings.sh

# Build the iOS app for the simulator
./scripts/build-ios-app.sh

# Run the full UI test suite
./scripts/run-ios-ui-tests.sh
```

Rust-side tests:

```bash
cargo test --workspace
cargo run -p content-lint --quiet -- content
```

## Content

Every concept, domain, and build is data — RON for structure,
Markdown for prose — under `/content`. Adding a new concept never
requires touching Rust unless its puzzle mechanic is genuinely new;
`GeneratorKind::default_for_concept` maps a concept id straight onto
an existing generator wherever the mechanic already exists.

## License

MIT — see [LICENSE](LICENSE).
