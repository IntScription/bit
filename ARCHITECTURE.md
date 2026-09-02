# BIT — Architecture Proposal

Status: proposal, pre-implementation. This document is the output of the
inspection/design pass requested before writing game code. Directory was
empty at inspection time — no existing code, conventions, or constraints
to reconcile.

## 0. Client architecture decision

Bevy was the spec's stated default, "unless there is a strong technical
reason to use another framework." BIT is UI-heavy, text-heavy, and
navigation-heavy (a knowledge library, forms of practice, settings,
accessibility needs) with animation concentrated in specific moments
(bits toggling, packets traveling, a CPU pipeline stepping) rather than
continuous real-time simulation. A full ECS game engine optimizes for
the latter and is weak on the former on iOS specifically: text input,
VoiceOver/Dynamic Type accessibility, native navigation idioms, and App
Store polish are all DIY in Bevy today.

**Decision (confirmed with product owner): Rust core + native SwiftUI
shell.**

- All logic — knowledge graph, challenge engine, simulations, build
  engine, progression, persistence — lives in a Rust workspace with zero
  UI/rendering dependencies.
- The Rust core compiles to a static library and is exposed to Swift via
  **UniFFI**.
- SwiftUI owns every screen: navigation, the five tabs, forms, lists,
  the Explore file-tree, settings.
- Animated moments (switches toggling, gates firing, packets moving,
  pipeline stages advancing) are built with SwiftUI `Canvas` /
  `TimelineView` first, escalating to SpriteKit only if a specific
  animation needs it. No second game-engine dependency is introduced
  for v1. If a future simulation genuinely needs a custom renderer
  (e.g. a dense CPU pipeline or 3D visualization), evaluate then —
  don't pre-adopt Bevy/wgpu speculatively.
- This still satisfies the spec's hard requirement ("core simulation
  and educational logic should not depend directly on rendering") more
  strictly than a Bevy app would, since the FFI boundary makes the
  separation physical, not just architectural discipline.

Risk and mitigation for this choice are in §11.

## 1. Workspace layout

```
/bit
  Cargo.toml                 # workspace root
  /crates
    bit-knowledge/           # domain tree, concept graph, search, related/used_by
    bit-content/              # loads & validates RON+Markdown content into typed structs
    bit-challenge/            # procedural challenge engine, per-domain generators
    bit-simulation/           # pure, deterministic simulations (no UI deps)
    bit-build/                 # BuildDefinition + BuildSession, wraps bit-simulation
    bit-progression/           # mastery model, adaptive-difficulty signals, round pacing
    bit-persistence/           # SaveState schema, versioned local storage
    bit-core/                  # facade crate: single BitEngine API composing the above
    bit-ffi/                   # UniFFI bindings exposing bit-core to Swift
  /content
    /domains/**/*.ron          # domain tree nodes (recursive)
    /concepts/**/*.ron          # concept metadata, mirrors domain folders
    /concepts/**/*.md           # concept prose bodies (one per concept, same id)
    /challenges/*.ron           # challenge templates
    /builds/*.ron                # build definitions
    /missions/*.ron               # missions
  /tools
    content-lint/                # validates /content against bit-content schema, checks graph integrity
  /ios
    BitApp/                       # SwiftUI app (added when FFI surface stabilizes)
  ARCHITECTURE.md
```

Rationale: `/content` sits outside `/crates` because it's data, not code
— authored, reviewed, and linted independently of the Rust build, and
bundled into the iOS app as resources rather than compiled in. Crate
boundaries mirror the separation the spec asks for in §15 (Knowledge /
Challenge / Simulation / Build / Progression / Persistence engines as
distinct, independently testable units) plus a facade (`bit-core`) so
the FFI layer has one stable surface instead of six.

## 2. Knowledge graph data model

Two content kinds: **domains** (the recursive folder tree Explore
navigates) and **concepts** (leaves attached to a domain).

```ron
// content/domains/foundations/bits_binary.ron
Domain(
    id: "foundations.bits_binary",
    name: "Bits & Binary",
    parent: Some("foundations"),
    order: 10,
)
```

```ron
// content/concepts/foundations/bits_binary/binary.ron
Concept(
    id: "binary",
    title: "Binary",
    domain: "foundations.bits_binary",
    difficulty: Core,
    applicability: [Core, Practical],
    prerequisites: ["bits"],
    related: ["bytes", "hexadecimal", "bitwise_operations"],
    learning_objectives: [
        "Convert between binary and decimal",
        "Understand positional value of bits",
    ],
    explanation: "binary.md",
    interactive_demo: Some(InteractiveDemo(
        id: "eight_switches",
        params: {"bit_width": "8"},
    )),
    practice_types: ["binary_to_decimal", "decimal_to_binary"],
    build_refs: ["binary_counter", "digital_lock"],
    lab_capabilities: ["binary_playground"],
    real_world_uses: [
        "CPU data representation",
        "Memory addressing",
        "Digital electronics",
    ],
    historical_relevance: None,
    mastery: MasteryRule(
        min_practice_correct: 5,
        min_accuracy: 0.8,
        decay_days: Some(30),
    ),
)
```

Notes:
- `interactive_demo` names a component id + parameters; it does **not**
  encode how to render it. Swift maintains a registry
  (`["eight_switches": EightSwitchesDemoView, ...]`). This keeps content
  data-driven per §16 without forcing every demo through a generic,
  weaker abstraction — bespoke interactions stay bespoke, just
  addressable by id.
- `bit-knowledge` builds the domain tree and reverse indices
  (`used_by`, i.e. concepts that list X as a prerequisite/related) once
  at load time. Recursion depth is unbounded — nodes are `{id, parent}`
  pairs, not nested structs, so there's no artificial 2–3 level limit.
- Prose lives in a sibling `.md` file, not inline in RON, so long-form
  writing doesn't fight a data-serialization format.

## 3. Content format choice

RON for structured metadata (enums, optional fields, and nested structs
map cleanly with serde; comments are allowed, unlike JSON), Markdown for
prose. `bit-content` deserializes RON via `serde` + `ron`, resolves
`explanation` paths, and returns fully typed `Concept`/`Domain` structs
— nothing downstream touches raw files. A `ContentSource` trait
abstracts *where* files come from (`FilesystemSource` for dev/tests,
`BundleSource` reading the iOS app bundle at runtime), so the loading
logic is identical on both sides.

## 4. Challenge generation architecture

```rust
pub trait ChallengeGenerator: Send + Sync {
    fn kind(&self) -> GeneratorKind;
    fn generate(&self, ctx: &GenerationContext) -> Challenge;
    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult;
}

pub struct GenerationContext {
    pub seed: u64,
    pub difficulty: DifficultyLevel,
    pub concept_ids: Vec<ConceptId>,
    pub mastery: MasterySnapshot,
    pub template: ChallengeTemplate,
}
```

- `ChallengeTemplate` (content data) names a `GeneratorKind` + concept
  ids + difficulty-curve params. New challenges from existing mechanics
  need zero Rust changes — just a new template. A genuinely new
  mechanic needs one new `ChallengeGenerator` impl.
- Each domain gets its own generator family in `bit-challenge`
  (`binary::ConversionGenerator`, `logic::GateEvalGenerator`,
  `graph::PathfindingGenerator`, …), matching §8's requirement that
  networking/CPU/algorithm challenges each use domain-appropriate
  generation rather than one generic template engine.
- Every `Challenge` carries `(seed, template_id, params)`, making it
  fully reproducible — this is what lets Daily Challenge hand every
  player the same seed for the day and lets tests assert exact output
  for a given seed.
- `GenerationContext.mastery` is how adaptive difficulty (§9) enters
  generation: weak concepts get sampled more often, difficulty curves
  shift, but nothing is ever *blocked* — matches "do not punish
  learning."

## 5. Simulation & Build architecture

`bit-simulation` holds pure, deterministic, step-driven simulations with
no timing/rendering dependency — `apply(state, action) -> StepResult`,
never a tick-by-wallclock loop. This is what makes them unit-testable
and reusable across Build, Lab, and Play.

```rust
pub trait Simulation {
    type Action;
    type State: Clone + Serialize;
    fn initial_state(&self, def: &BuildDefinition) -> Self::State;
    fn apply(&mut self, state: &mut Self::State, action: Self::Action) -> StepResult;
    fn check_objectives(&self, state: &Self::State, objectives: &[Objective]) -> ObjectiveStatus;
}
```

Concrete simulations, roughly one per Build tier from §2: `BinaryCircuitSim`,
`LogicGateSim`, `MemorySim`, `RegisterMachineSim`, `CpuSim` (registers,
ALU, later pipeline stages), `NetworkSim`, `FilesystemSim`.

`bit-build` wraps `{BuildDefinition + a Simulation}` into a
`BuildSession` that Swift drives via `apply_action` / `snapshot_state` /
`check_objectives` through FFI. Objectives are declarative predicates
over state ("byte value == 0b1011", "all requested addresses
readable"), not a single expected answer — this is what makes multiple
valid solutions possible per §11 of the spec without special-casing
each build.

## 6. Explore navigation architecture

`bit-knowledge` exposes:

```rust
fn children(domain: DomainId) -> Vec<DomainNode>;
fn concepts_in(domain: DomainId) -> Vec<ConceptSummary>;
fn concept(id: ConceptId) -> ConceptDetail;
fn prerequisites(id: ConceptId) -> Vec<ConceptSummary>;
fn related(id: ConceptId) -> Vec<ConceptSummary>;
fn used_by(id: ConceptId) -> Vec<ConceptSummary>;   // reverse edge, built at load time
fn search(query: &str) -> Vec<SearchHit>;
```

Search is a simple in-memory inverted index (token → concept ids) built
once at startup from title/aliases/tags — content is small and fully
local, so no embedded search engine dependency is justified. SwiftUI
renders `children()`/`concepts_in()` as a `List` with disclosure groups,
giving the filesystem feel the spec asks for without hand-rolling tree
UI.

## 7. Progression & mastery model

Per concept: `MasteryState { discovered, practiced_count,
correct_count, stage: Discovered|Practicing|Proficient|Mastered,
last_seen, streak }`. Domain-level breadth/depth are computed on demand
from the set of concept states in that subtree, not stored redundantly
— one source of truth.

Round pacing (Demonstration → Fiddle → Guided Practice → timed/ranked
challenges, §3) is modeled as a `RoundStage` enum owned by
`bit-progression`, not scattered across screens: a concept's current
stage is queried once (`round_stage_for(concept_id, mastery)`), and
that's what gates whether the UI is allowed to show timers, lives, or
streak pressure. This keeps "never introduce pressure before
understanding" testable as a pure function instead of a UI convention
someone can forget.

## 8. Persistence

`SaveState` is a single versioned, serde-serialized struct (`{
schema_version, mastery: HashMap<ConceptId, MasteryState>, builds: [...],
bookmarks, notes, ... }`) written to local disk (JSON for v1 —
human-inspectable during development; revisit only if profiling shows a
need for something incremental like `redb`). `schema_version` plus
explicit `migrate_vN_to_vN+1` functions are mandatory from the first
commit, since "no fixed ending" means saves must survive years of
content growth across app updates. Autosave triggers on
state-changing events (challenge completed, mastery changed, build
saved), not on a timer.

## 9. Content validation

`tools/content-lint` walks `/content`, deserializes every file through
`bit-content`'s real types (so the validator can never drift from the
runtime parser), and checks: every `prerequisites`/`related`/`domain`
reference resolves, no prerequisite cycles, every
`practice_types`/`build_refs`/`lab_capabilities` id resolves, every
domain has a valid parent up to a single root. Run in CI once CI
exists; run locally via `cargo run -p content-lint` in the meantime.
This is the guardrail that makes hand-authoring hundreds of concepts
safe.

## 10. FFI surface (bit-ffi) — implemented

`FfiEngine`, a `uniffi::Object` wrapping `bit-core::BitEngine` behind a
single `Mutex`, exposes the five tabs' needs as proc-macro (`#[uniffi::
export]`) methods — no UDL file. Two payload strategies, chosen per
field: structured UniFFI records/enums for small, stable, hot-path
values (`FfiCircuitAction`/`FfiCircuitState`, `FfiPixelAction`/
`FfiPixelState`, mastery, round stage, build/objective status — one
pair of types per simulation kind, which costs little at two kinds and
is safer than a stringly-typed action format), and a single JSON string
for `Concept` via `concept_detail_json` (its shape is large and still
evolving, so it isn't hand-mirrored as a second struct). Challenge
answers are validated Rust-side (`validate_challenge` re-derives the
same challenge from `(concept_id, difficulty, seed)` rather than trusting
a client-held expected answer) — determinism doubles as the mechanism
that keeps challenge state out of the FFI object entirely.

Swift bindings are generated via `uniffi-bindgen` in **library mode**
(reads metadata straight from the compiled `cdylib`, no UDL, no
build.rs) — regenerate with `./scripts/generate-swift-bindings.sh`,
never by hand-editing `ios/Generated/`.

## 11. First vertical slice (concrete mapping of spec §22)

| Layer | Slice deliverable |
|---|---|
| bit-knowledge + bit-content | ~11 foundation concepts (Bits, Binary, Decimal, Hex, Bytes, Boolean Logic, AND, OR, XOR, NOT, Shifting) as real RON+MD, loaded and graph-validated |
| bit-simulation | `BinaryCircuitSim` (switches ↔ value), `LogicGateSim` (wireable AND/OR/XOR/NOT) |
| bit-challenge | `ConversionGenerator` (binary↔decimal↔hex), `GateEvalGenerator`, both seeded and unit-tested for determinism |
| bit-progression | `RoundStage` pacing (Demonstration → Fiddle → Guided → timed) exercised end-to-end for one concept |
| bit-build | Binary Counter, Digital Lock, Pixel Builder, Bit Flag System, all riding the two simulations above |
| bit-persistence | Local save/load, scripted-session test (discover → practice → master → reload) |
| SwiftUI shell | 5 tabs; Explore as a real recursive tree + search; Play driving one concept through all four rounds; Lab as a freeform binary + logic-gate playground; Profile showing discoveries/mastery — all backed by `bit-ffi` |

Explicitly deferred: Daily Challenge, Missions polish, adaptive-difficulty
tuning beyond raw signal capture, every domain past Foundations.

**Content status beyond the original slice (updated as domains grow):**
a second top-level domain, **Data Representation**, now exists —
Integers (Unsigned Integers, Signed Integers, Two's Complement), Text
Encoding (ASCII, Unicode, UTF-8), Data Integrity (Checksums). 18
concepts total across 8 domains. Two's Complement rides the *same*
`BinaryCircuitSim` as Binary (`State::signed_value` reads the identical
switches with a negative-weight sign bit — see bit-simulation), and adds
two new challenge generators (`TwosComplementGenerator`,
`AsciiLookupGenerator`) rather than new builds — no new Build
definitions were added for this domain, and Play/Lab/Build still only
surface the original Foundations builds; Explore is the only tab that
needed zero code changes to make the new domain reachable (confirmed via
`BitAppUITests`). Floating point/IEEE 754, Serialization, and
Compression (also listed under DATA REPRESENTATION in spec §6) are
deliberately deferred — floating point specifically needs its own
sign/exponent/mantissa simulation, not a reinterpretation of the
existing one.

## 12. Technical risks

1. **FFI schema duplication.** Mitigate via the JSON-blob-for-large-
   payloads strategy in §10; keep the hot path (actions/snapshots) as
   real UniFFI records.
2. **Animation fidelity without a game engine.** SwiftUI `Canvas`/
   `TimelineView` must sell "bits physically moving." Spike the hardest
   case (a CPU pipeline advancing) early, not last — if it's genuinely
   unworkable, that's the trigger to reconsider a renderer, not before.
3. **Content authoring scale.** RON is verbose at hundreds+ of
   concepts. `content-lint` (§9) is non-optional infrastructure, not a
   nice-to-have, and should exist before content volume grows past
   what one person can eyeball.
4. **Determinism across the FFI boundary.** All seeded RNG stays in
   Rust; Swift never generates or perturbs a seed, only reads results —
   otherwise Daily Challenge reproducibility breaks silently.
5. **iOS bundle size.** Data-driven content plus any bundled media
   grows with domain count; not a v1 concern, but watch it once
   Advanced/Extreme domains land, and consider On-Demand Resources for
   rarely-visited domains only if it becomes real.
6. **Save schema evolution.** Must exist from commit one (§8) — endless
   progression means saves outlive any single content release.
7. **Two build systems to keep in sync.** Cargo workspace + Xcode
   project both need updating when crates change; keep `bit-ffi`'s
   generated Swift bindings checked into a predictable location and
   regenerate via a single script, not manually.

## 13. Incremental implementation order

1. ~~Scaffold the Cargo workspace + crate skeletons.~~ Done.
2. ~~`bit-knowledge` + `bit-content`, real foundation concepts, unit
   tests.~~ Done — 11 Foundations concepts across 4 domains.
3. ~~`bit-simulation`: `BinaryCircuitSim` + `LogicGateSim`, unit
   tests.~~ Done — plus `PixelGridSim` for Pixel Builder.
4. ~~`bit-challenge`: the two v1 generators, deterministic seeded
   tests.~~ Done — `BinaryConversionGenerator` + `GateEvalGenerator`.
5. ~~`bit-progression` + `bit-persistence`, a scripted play-session
   test.~~ Done. (Also done, ahead of schedule: `bit-build`'s catalog
   and all four vertical-slice builds, content-loaded and lint-checked.)
6. ~~`bit-ffi` exposing a minimal `BitEngine`.~~ Done — see §10.
   `FfiEngine` covers Explore, Progression, Persistence, Builds, and
   Challenges; Swift bindings generate cleanly via library-mode
   `uniffi-bindgen`.
7. ~~iOS/SwiftUI shell wired to `bit-ffi`, five tabs at vertical-slice
   fidelity.~~ Done — see §14.
8. Playtest. Then expand content and domains. **← next.**

## 14. iOS app (BitApp) — implemented

`ios/BitApp/BitApp.xcodeproj` is a native Xcode project (hand-authored
`project.pbxproj`, generated by `scripts/gen_pbxproj.py` — regenerate
that script's output, never hand-edit the file) linking directly against
`BitFFI.xcframework` — no Swift Package Manager layer. That framework is
a static-lib xcframework built from `libbit_ffi.a` cross-compiled for
`aarch64-apple-ios` (device) and `aarch64-apple-ios-sim` (Apple Silicon
simulator), with the generated C header + `module.modulemap` embedded
per-slice so `import bit_ffiFFI` resolves without manual search paths.
`ios/Generated/bit_ffi.swift` (pure Swift, no C symbols of its own) is
added to the app target as a source file directly. `/content` is added
as an Xcode **folder reference** (not individually-listed files), so it
copies into the app bundle as a real directory tree — `Engine.swift`
points `FfiEngine.init(contentDir:)` at `Bundle.main.resourcePath +
"/content"` at runtime, exactly the `FilesystemSource` path used by
every Rust-side test.

Rebuild everything with `./scripts/build-ios-app.sh` (cross-compile →
xcframework → Swift bindings → `xcodebuild`). Verified end-to-end: built
for the simulator, installed and launched on iPhone 16 (iOS 18.5),
confirmed via screenshot to render live data from the real content graph
— round-stage-gated Play (Round 0 Demonstration, no timer/pressure chrome,
per spec §3), not placeholder UI — and then driven through real automated
taps via `scripts/run-ios-ui-tests.sh` (see below).

Five tabs, all backed by real `FfiEngine` calls, no mocked data:

- **Play** — single concept ("binary") driven through all four round
  stages (`RootView.swift` → `PlayView.swift`). Demonstration/Fiddle
  reuse the `binary_counter` build session as an interactive switches
  widget with objective chrome simply never shown; Guided/Timed switch to
  `generateChallenge`/`validateChallenge`.
- **Build** — `listBuilds()` → pick one → drives it to `objectiveStatus
  == .met` via the shared `SwitchesView`/`PixelGridView` components
  (`BuildView.swift`).
- **Explore** — recursive `DomainListView` (unbounded depth, per §6) +
  `.searchable` search, concept pages decode `concept_detail_json`
  through a `Decodable` `ConceptDetail` that only declares the fields the
  UI renders (`ExploreView.swift`).
- **Lab** — the exact same build sessions as Build, freeform: the UI
  layer simply never reads `objectiveStatus` (`LabView.swift`).
- **Profile** — `allConcepts()` + per-concept `mastery()`, discovery/
  mastery framing only, no grade language (`ProfileView.swift`).

**Interaction verified via real automated taps, not just a screenshot.**
`ios/BitApp/BitAppUITests` (added to the project by the same
`scripts/gen_pbxproj.py`, run via `scripts/run-ios-ui-tests.sh`) drives
`XCUIApplication` against the simulator through Xcode's test runner.
This is the correct mechanism — AppleScript/System Events was tried
first and rejected: Simulator's window isn't exposed as a standard
accessibility element to `System Events`, so host-level "click at
coordinates" automation can't reliably reach it, and there's no way to
grant that permission non-interactively. XCUITest talks to the simulator
directly and doesn't have that problem. Two tests, both passing:

- `testPlayRoundProgressionAndTabNavigation`: launches the app, confirms
  Round 0 Demonstration renders (value `0`, all switches off), taps
  "I see it — let me try", confirms the transition to Round 1 Fiddle
  (demonstration button gone), taps a switch and confirms the displayed
  value actually changes `0 → 1 → 0` on tap/untap — the full round-trip
  through `applyCircuitAction` and back — then taps through all five
  tabs and confirms each renders real content (a real build name, a real
  domain, a real Profile stat).
- `testExploreDrillDownAndSearch`: drills three levels into the real
  domain tree (Foundations → Bits & Binary → Binary), confirms the
  concept detail page renders real prerequisites/objectives/real-world-
  uses sections, backs out to root, then types into the live search
  field and confirms a real result appears.

Not yet done, deliberately deferred: app icon/launch-screen polish,
animation beyond SwiftUI's default transitions, and Missions/Daily
Challenge screens (their Rust engines don't exist yet either).

## 15. UX pass: theming, interactive demos, persistence, Play generalization

A batch of cross-cutting improvements, all verified via `XCUITest` (not
just build success) and 75 Rust tests total across the workspace.

**A real bug fixed first.** `MasteryState.stage` never left `Discovered`
— nothing advanced it — so `round_stage_for` could never reach
`TimedChallenge` no matter how much a concept was practiced. Play
generalizing to more concepts made this concrete rather than
theoretical, so it was fixed before anything downstream depended on it:
`bit-progression` gained `MasteryState::advance_stage(StageThresholds)`,
driven by each concept's own content-authored `MasteryRule`
(`min_practice_correct`/`min_accuracy`), called from `bit-core::
record_attempt`. Promotion only, never demotion; `Mastered` requires a
fresh streak at `Proficient`, not just meeting the bar once.

**Persistence is now actually wired to the app**, not just present in
the FFI layer. `Engine.swift` loads `Documents/bit-save.json` on launch
and writes it after every mutation (`recordDiscovery`/`recordAttempt`)
plus on `scenePhase == .background` as a safety net. Verified by an
XCUITest that discovers a concept, force-terminates the app (not just
backgrounds it), relaunches, and confirms the round stage survived.

**Every `interactive_demo` id referenced anywhere in `/content` now
resolves to a real, live widget** — `DemoContainerView` (`Demos.swift`)
dispatches by id to `CircuitDemoView` (FFI-backed, dynamic bit width,
optional signed/hex readings), `GateDemoView` (FFI-backed, AND/OR/XOR/
NOT picker), `AsciiTableView`/`Utf8BreakdownView`/`XorChecksumDemoView`
(pure local Swift state — no simulation needed for those). Two new
ad hoc bit-ffi constructors make this possible without forcing every
demo through the content-driven Build catalog: `start_circuit_demo` and
`start_gate_demo` construct a `BuildSession` directly (always freeform,
always succeeds — there's no id lookup that can fail), which is also
why `bit_build::catalog::LiveBuildSession` grew a `LogicGate` variant
even though no content Build rides `LogicGateSim` yet. `ConceptDetailView`
renders the demo before any prose section (spec §4: INTERACT before
EXPLAIN). `bit_build::BuildSession` gained a `simulation()` accessor so
callers can read facts about the simulation itself (e.g. which gate a
session is wired to), not just its state.

**Play no longer hardcodes "binary."** It picks from
`practicable_concepts()` (every concept `GeneratorKind::
default_for_concept` resolves — `bit-core` filters `all_concepts()` by
probing `generate_challenge`) and reuses the exact same `DemoContainerView`
Explore uses for Rounds 0–1, switching to the existing generate/validate-
challenge flow for Rounds 2–3. This is also why the demo work above
wasn't optional scope creep — Play's generalization depends on it.

**Domain-scoped theming**, Infinity-Loop-style but keyed to top-level
domain rather than per-level: `Theme.swift` maps a domain id's root
segment (`"data_representation.integers"` → `"data_representation"`) to
a color, via an explicit table for known domains falling back to a
deterministic FNV-1a hash (not Swift's `Hasher`, whose seed is
randomized per process — hashing would give a different color every
launch) so any future domain gets a stable color with no code change.
Applied via `.tint()` on Explore's domain/concept screens and Play's
round view; deliberately *not* applied to Build/Lab (no domain linkage
is exposed for a `FfiBuildSummary` yet) — scope boundary, not an
oversight.

**Accessibility and game feel.** Every switch/pixel/gate button gained a
deterministic `accessibilityIdentifier` and a real `accessibilityLabel`
— previously multiple switches all shared the ambiguous label "0",
which is also what made writing reliable `XCUITest`s for them possible.
`Haptics.swift` adds light-impact feedback on every toggle and success/
error notification feedback on a correct/incorrect Play answer and on
first reaching an objective-met state (`ObjectiveBanner` tracks
"already celebrated" internally so it fires once, not on every
re-render while already met).

**A UI-testing lesson worth keeping**: two tests were flaky specifically
under full-suite load (not standalone) — a `NavigationLink`'s
destination can pre-materialize accessibility elements before its push
visually completes, making a bare `staticTexts["Name"]` query briefly
ambiguous when the destination reuses the same name as its own title
(every concept detail page does, via `ConceptDetailView`'s
`.navigationTitle`). Fixed by waiting for the destination's
`navigationBars[...]` to exist before the *next* tap, and using
`.firstMatch` on any label shared between a list row and its
destination's title. Also: a plain `Shape`/`View` isn't automatically an
accessible element the way a `Button` is — `GateDemoView`'s output
swatch needed an explicit `.accessibilityElement(children: .ignore)`
before `otherElements["gate-output"]` (not `.buttons`) could find it.

## 16. Animation pass: flips, domain washes, and a zoom transition

Deliberately scoped to native SwiftUI 3D transforms and spring physics,
not a 3D engine — see the design discussion that preceded this: full
3D (RealityKit/SceneKit) is real complexity and risks feeling gimmicky
for an app whose value is precise bit states, not spectacle. True 3D is
reserved for later, concept-driven builds (a CPU pipeline, packets
traveling a network path) once those domains exist — there's nothing to
attach it to yet.

- **Switch/pixel/gate-input flip.** `FlipToggleCell` (`Components.swift`,
  shared by `SwitchesView`, `PixelGridView`, and `GateDemoView`) rotates
  180° around the Y axis per tap via `rotation3DEffect`, incrementing a
  local counter rather than toggling between two fixed angles — repeated
  taps keep spinning forward instead of snapping back. No front/back
  content-swap trick was needed: 0/1 digits and solid-color pixels both
  read correctly mirrored, so the simple version is the correct version,
  not a shortcut.
- **Value assembly bounce.** `SwitchesView`'s value text scales up then
  settles via two chained `withAnimation` spring calls on `state.value`
  changing — a small "thunk" as the number lands, on top of the existing
  `.contentTransition(.numericText())`.
- **Domain-entry wash** (`DomainWashOverlay`, `ExploreView.swift`): a
  brief full-screen flash in the domain's theme color, fading over
  ~0.45s — deliberately gated to *top-level* domain entry only
  (`!domainId.contains(".")`), not every subdomain push, so it marks the
  moment the palette actually changes rather than firing on every tap.
- **Zoom navigation transition**: iOS 18's `.navigationTransition(.zoom
  (sourceID:in:))` morphs a concept's list row into its detail page
  header. Wrapped in `zoomSource`/`zoomDestination` view-extension
  helpers that degrade to a plain push below iOS 18 (`if #available`),
  so the deployment target stayed at 17.0 rather than forcing a bump for
  one visual flourish. Only wired from `DomainListView`'s list rows —
  Related/Prerequisites links inside `ConceptDetailView` construct a
  fresh `ConceptDetailView` recursively without a matching source, so
  `zoomId`/`zoomNamespace` are optional and default to nil there (plain
  push, no error).
- **Correct-answer / objective-met celebration**: native
  `.symbolEffect(.bounce)` on a checkmark (`ObjectiveBanner`, Play's
  feedback row) rather than a hand-rolled particle system — iOS 17+,
  zero new dependencies, and reads as more "native app" than a custom
  effect would.

All verified by re-running the existing `XCUITest` suite (flip/bounce/
wash/zoom don't block or misdirect any tap) plus two screen recordings
captured via `xcrun simctl io <device> recordVideo` during live test
runs — static screenshots can't show motion, so for this pass the actual
proof is video, not an image.

## 17. Play redesign: endless game mode, separate from learning

A product decision, not just a UI change: **Explore now owns all
learning** (interactive demos, prose, discovery) and **Play owns all
practicing** — the two used to overlap (Play carried its own
Demonstration/Fiddle rounds duplicating what a concept's page already
showed). Play no longer has a concept picker or any interactive-demo
rendering; opening the tab drops straight into an endless stream of
generated challenges.

- **The question pool** (`BitEngine::game_pool`) is practicable ∩
  discovered concepts — exactly "only topics the player has learned," so
  the puzzle mix grows automatically as Explore is used, with zero
  additional wiring.
- **The level number** (`game_total_correct`, new `SaveState` field) is
  a lifetime count of correct answers in game mode, deliberately just a
  bare number at the top — no "Level" label, no per-concept framing.
  Difficulty ramps gently with it (`3 + level/10`, capped by whatever
  each generator itself clamps to) rather than staying fixed.
- **New-topic suggestion**: `BitEngine::next_learnable_concept` finds a
  concept whose *every* prerequisite is discovered but which isn't
  itself discovered yet (ties broken by `Difficulty::rank`, easiest
  first). After `NEW_TOPIC_SUGGESTION_THRESHOLD` (15, the middle of the
  requested "10-20") correct answers *since the last suggestion* — a
  count that doesn't reset on a single miss, so one wrong answer doesn't
  erase progress toward the next prompt — Play offers a sheet to jump
  straight to that concept. Deliberately a dismissible suggestion, not a
  forced redirect, matching spec's "do not punish learning."
- **Cross-tab deep-linking**: `RootView` now owns `selectedTab` and an
  `explorePath: NavigationPath`, passed into `ExploreView` as a binding.
  Accepting the suggestion pushes the concept id onto that path and
  switches tabs — the player lands exactly on the new concept's page,
  not just "somewhere in Explore." `ExploreView`'s existing inline-
  `NavigationLink` navigation and this value-based
  `.navigationDestination(for: String.self)` coexist in the same
  `NavigationStack` without conflict.
- **Testability tradeoff, stated plainly**: a puzzle's expected answer is
  seeded randomly client-side and deliberately never exposed over FFI
  (see `FfiChallenge`'s doc comment), so a UI test cannot know a
  *correct* answer to type. `BitAppUITests` therefore verifies the
  observable mechanics end-to-end (pool populates on discovery, wrong-
  answer feedback renders, level doesn't move on a miss, cross-tab
  navigation works, discovery persists across relaunch) while the
  correct-answer → level-increment path is proven at the Rust level
  instead (`bit-ffi`'s `game_progress_counts_and_suggests_a_new_topic_
  after_threshold`).
- **A real UX bug this surfaced**: submitting an answer left the
  keyboard up, which then intercepted the tab bar's hit-test on the next
  tap (`Computed hit point {-1, -1}` in the failure trace — a real
  symptom, not test flakiness). Fixed with `@FocusState` +
  `answerFieldFocused = false` in `checkAnswer()`, which is the correct
  fix regardless of testing — a form should dismiss its own keyboard on
  submit.

**Simulator note**: mid-session, the previously-used iOS 18.5 simulator
started crashing at launch (`Library not loaded: @rpath/
lib_TestingInterop.dylib`) — Xcode 26's Swift Testing support requires
a runtime built for it, and 18.5 predates that. `run-ios-ui-tests.sh`
and `build-ios-app.sh` now pick the newest available "iPhone 17"/
"iPhone 16" runtime dynamically instead of a fixed device UDID, so this
doesn't recur as Xcode/runtimes are updated.

This document covers steps 1–11 of that order's prerequisite (the
proposal itself). Implementation starts now with step 1 only:
workspace + crate skeletons, no game logic yet — see §14 through §19
above for how far past that this project now is.

## 19. Interactivity/detail polish: Build, Lab, Profile, and Play

§15/§16 gave Play and Explore the full animation/theming treatment;
Build, Lab, and Profile stayed functional but plain. This phase closes
that gap and adds a few small-detail touches to Play, without touching
any Rust — everything here is presentation-only, riding data the FFI
layer already exposed.

- **`SquishyButtonStyle`** (`Components.swift`): one shared tinted-
  capsule button style with a press-down squish, replacing
  `.borderedProminent` on every primary CTA (Play's Check button, the
  empty-state "Go to Explore", the new-topic suggestion's "Learn X",
  Array Builder's "Set"). It reads `Color.accentColor`, which resolves
  from whatever `.tint()` is active in the environment, so
  domain/tier theming still flows through exactly like
  `.borderedProminent` did — this is styling, not a second parallel
  theming mechanism.
- **Build**: rows became tier-colored cards (`FfiBuildTier.accentColor`/
  `.icon`/`.label` — green leaf for Beginner, blue cube for Core,
  purple bolt for Advanced, red flame for Extreme) instead of plain
  text, reusing Explore's `.zoomSource`/`.zoomDestination` extensions
  for the row-morphs-into-session transition. `BuildSessionView` tints
  itself with the same tier color, so a build's identity carries from
  the list into the session.
- **Lab**: the segmented picker gained icons (`Label(title:icon:)` per
  mode), and each playground now sits in a softly tinted rounded
  container with its own fixed color (blue/purple/teal — deliberately
  *not* `Theme.color(forDomain:)`, since Lab isn't tied to one
  concept) that cross-fades/scales in on mode switch.
- **Profile got a real fix, not just polish**: the old version's single
  "Foundations" section header was hardcoded text applied to *every*
  concept regardless of actual domain. It's now grouped by root domain
  (via `rootDomains()` + `Theme.rootDomainId`), each section colored to
  match that domain's Explore color, with a `MasteryBadge` (icon +
  color per `FfiMasteryStage`) replacing the old plain-text status
  string. Two `StatCard`s ("Concepts discovered", "Concepts mastered")
  give the header real visual weight — those exact label strings are
  kept as plain `Text`, not folded into a `Label`, specifically so
  `BitAppUITests`' existing `staticTexts["Concepts discovered"]` query
  keeps working unchanged.
- **Play small details**: a streak counter (flame icon + "N in a row",
  shown once streak ≥ 2, reset on a miss) sitting under the level
  number; a brief expanding/fading ring behind the level number on
  every level-up, on top of the existing number bounce; a per-puzzle
  slide/fade transition (`.id(challenge.seed)`) so a new puzzle
  visibly arrives rather than snapping in place; and a quiet
  difficulty readout (8 small capsules, filled up to
  `currentDifficulty`) under the concept badge — a texture, not a
  second score to chase.

**Verification**: full simulator rebuild succeeded; all 6 UI tests
pass (one incidental full-suite-load flake in
`testDataStructuresAndBitManipulationAreReachable` reproduced green in
isolation and on a clean full-suite rerun, so not a regression from
this phase — see §15's existing note on that class of flake).
`testPlayEndlessGameMode` now also attaches `build-list`,
`lab-binary-playground`, and `profile-overview` screenshots (it
already visited all three tabs) alongside its existing
`play-active-puzzle` attachment, giving every polished screen a real
on-device screenshot in the test artifacts, not just Play/Explore.

## 18. Content expansion: Bit Manipulation and Data Structures

A pure content-and-mechanics growth phase — no product redesign, just
"more concepts, more puzzles, more fundamentals, more projects to
build," following the same incremental pattern as every prior content
phase (see §7's Data Representation domain).

**Bit Manipulation** (`foundations.bit_manipulation`, order 30): three
concepts — Bitwise Masks, Population Count, Bit Rotation — that build
directly on Foundations.Logic without any new simulation. Masking is
framed explicitly as "the same AND/OR/XOR you already know, applied
across a whole byte instead of one wire," which is also why
`bit_flag_system.ron`'s `concept_ids` grew a `"bitwise_masks"` entry:
that build was always a masking exercise wearing a permissions
costume, and now says so.

**Data Structures** (new top-level domain, order 20, with one leaf
`data_structures.linear`): Arrays, Stacks, and Queues. All three ride
one new simulation, `bit_simulation::array_seq::ArraySim` — a
fixed-capacity `Vec<u8>` of indexed slots with `Set`/`Clear` actions
and a `filled: Vec<bool>` companion tracking which slots hold a real
value. Deliberately a dumb indexed slot machine with no push/pop
discipline built in, matching how `binary_circuit::State::signed_value`
reinterprets the same switches instead of `BinaryCircuitSim` growing a
second mode: Stacks and Queues get their LIFO/FIFO discipline entirely
from their challenge generators (which narrate a push/pop or
enqueue/dequeue sequence in prose), not from the simulation. Only
Arrays gets a live interactive demo (`array_slots`, tap a slot then
type a value — the first widget in this app where a discrete tap isn't
enough information, so `ArraySlotsView` pairs the slot grid with a
small numeric `TextField`), because indexed read/write *is* what an
array demonstrates, whereas Stack/Queue's whole point is an access
*restriction* that's better taught as a worked trace than a freeform
sandbox.

**Six new challenge generators**
(`PopCountGenerator`/`MaskGenerator`/`RotationGenerator`/
`ArrayIndexGenerator`/`StackTraceGenerator`/`QueueTraceGenerator`),
registered the same way as the original four: a `GeneratorKind`
variant, a `GeneratorKind::default_for_concept` match arm, and a
`default_registry()` entry. `StackTraceGenerator`/`QueueTraceGenerator`
narrate a short randomized operation sequence as prose ("Start with an
empty stack. push(5), push(9), pop() ...") and ask what the final
operation returns or leaves on top/front — the puzzle *is* the LIFO/
FIFO trace, so it's generated directly rather than round-tripped
through a live `ArraySim` session.

**One new Build**: `array_builder.ron` (Core tier) — fill a six-slot
array to match a target byte sequence, exactly the same "guess toward
a hidden target, `ObjectiveBanner` tells you when you're there"
mechanic as Binary Counter, just riding `ArraySim` instead of
`BinaryCircuitSim`. `bit_build::catalog::LiveBuildSession` gained an
`Array` variant and `instantiate()` gained one match arm — per that
module's stated philosophy, adding a fifth simulation-backed build
family costs one arm, not a redesign.

**FFI and Swift**: `bit-ffi` gained `FfiArrayAction`/`FfiArrayState`,
`start_array_demo`/`array_state`/`apply_array_action`, and an `Array`
arm in every existing match on `LiveBuildSession` (`objective_status`,
`session_kind_name`). Swift gained `ArraySlotsView` (in
`Components.swift`, alongside `SwitchesView`/`PixelGridView`) and
`ArrayDemoView` (in `Demos.swift`, mirroring `CircuitDemoView`'s use of
an ad hoc `start_*_demo` session rather than the Build catalog) — reused
in three places: Explore's Arrays concept page, `BuildSessionView`'s
new `isArray` branch, and Lab's new "Array" playground segment
(`LabArrayView`, riding the real `array_builder` session in freeform
display, same pattern as `LabPixelView`).

**Verification**: `content-lint` reports 11 domains / 24 concepts / 5
builds, all instantiable. `cargo test --workspace` is green across
every crate (bit-simulation 21, bit-challenge 22, bit-build 12, bit-ffi
13, plus the unchanged suites elsewhere). A new UI test,
`testDataStructuresAndBitManipulationAreReachable`, drives both new
domains end-to-end through the real simulator build — including
tapping an array slot, typing a value, confirming the FFI round trip
via the slot's own accessibility label, and confirming Array Builder
is listed in the Build tab — alongside the five pre-existing UI tests,
all passing on the same iPhone 17 / iOS 26.5 runtime as §17.

## 20. Daily-user retention pass: streaks, review, milestones

Eight features, built one at a time in priority order from a "what
would actually bug me as a daily player" review — every one required a
real Rust decision, not just a Swift screen, because they all needed
new persisted state.

**One recurring pattern**: every date-touching feature keeps Rust
wall-clock-free (see /ARCHITECTURE.md's founding rule) by having Swift
supply "today" as a plain `"YYYY-MM-DD"` string (`Engine.todayString()`,
`en_US_POSIX` + `.current` timezone) and Rust do pure calendar-day math
against it — `bit_core::days_since_epoch` (Howard Hinnant's
`days_from_civil`, dependency-free) is the one place that math lives,
reused by both the daily streak and staleness ("rusty") checks below.

1. **Daily calendar streak**, distinct from Play's existing in-session
   correct-answer streak. `SaveState` gained `last_active_day` /
   `daily_streak` / `longest_daily_streak`; `BitEngine::
   record_daily_activity(today)` is idempotent within a day, increments
   on a consecutive calendar day, resets to 1 on any gap, and ignores
   an out-of-order date outright (clock skew safety) rather than
   corrupting the streak. Called from `Engine.swift` on the app's two
   "real engagement" actions — a discovery and a correct answer — not
   on every attempt. Surfaced as a calendar-icon stat in Profile.

2. **Wrong-answer "Explain this" link**: Play's miss feedback grew a
   button that calls the same `onNavigateToExplore` cross-tab callback
   the new-topic suggestion already used, landing directly on the
   missed concept's Explore page. Pure Swift, no Rust change.

3. **Mastery decay surfaced as a "rusty" review weighting**: content
   already carried `decay_days` per concept (`MasteryRule`) but nothing
   read it. `MasteryState` gained `last_practiced_day` (stamped by
   `BitEngine::record_attempt`, which now takes `today` too — a
   deliberate breaking signature change, updated at its three call
   sites); `BitEngine::is_rusty(concept_id, today)` compares that
   against `decay_days`. Play's `pickNewPuzzle` builds a weighted
   candidate list (3x for anything rusty) instead of a flat
   `randomElement()`, and shows an orange "Review" badge when the
   current puzzle is one.

4. **Profile activity heatmap**: `SaveState` gained `activity_by_day:
   HashMap<String, u32>`, tallied unconditionally (even repeat same-day
   calls) inside `record_daily_activity` — one call site already firing
   for the streak now also feeds the heatmap for free. Exposed as
   `Vec<FfiDayActivity>` (uniffi doesn't carry raw maps well; every
   other bulk FFI value in this app is already a `Vec<Record>`, not a
   map). `ActivityHeatmapView` renders it GitHub-contributions-style:
   14 columns of Sunday-to-Saturday weeks, computed locally in Swift
   from `Calendar`, days beyond today rendered as empty space rather
   than a fabricated future.

5. **Milestone celebrations**: a generic, reusable mechanism —
   `SaveState.celebrated_milestones: Vec<String>` plus `BitEngine::
   has_celebrated`/`mark_celebrated`, keyed by a caller-defined string
   id — rather than one bespoke field per achievement. Detection stays
   entirely in Swift (Rust just stores flags): Play checks "100 correct"
   and "first concept mastered" right after a correct answer;
   `BuildSessionView` checks "first build completed" on a genuine
   `.inProgress` -> `.met` transition (guarded against freeform builds,
   which report `.met` unconditionally). One shared `MilestoneInfo` +
   `MilestoneCelebrationSheet` (`Components.swift`) backs all three.
   Play's suggestion sheet and milestone sheet were unified behind a
   single `PlaySheetKind` enum and one `.sheet(item:)` — two independent
   `.sheet(isPresented:)` modifiers on the same view can't safely be
   active at once, so a milestone always takes priority over a
   same-answer new-topic nudge; the nudge just waits for the next one.

6. **Onboarding**: a brand-new install's empty Play state used to say
   only "go learn something." It now calls the same
   `next_learnable_concept()` the new-topic suggestion already uses and
   names a concrete first concept ("Start with Bits"), which — since
   "bits" itself has no puzzle generator — chains naturally into
   suggesting "Binary" next once "bits" is discovered. No new engine
   method needed; this was purely a Swift-side reuse.

7. **Per-generator keyboard type**: the answer field defaulted to
   `.numbersAndPunctuation` for every puzzle. Now `.numberPad` for
   anything digit-only (most generators), `.numbersAndPunctuation` kept
   only for Two's Complement (needs a minus sign), and `.asciiCapable`
   for ASCII Lookup (its answer is a number *or* a single character,
   direction chosen randomly per puzzle — a numeric pad would make half
   its puzzles untypeable).

8. **Skip puzzle**: a plain "Skip this one" button that calls
   `pickNewPuzzle()` directly, never `recordAttempt`/
   `recordGameCorrectAnswer` — by construction it can't touch mastery,
   the level number, or the in-session streak.

**Verification**: `cargo test --workspace` green throughout (each
feature landed with its own Rust tests before any Swift work started
on it — daily streak: 9 new tests including a month-boundary and a
clock-skew case; `is_rusty`: 4; activity tally: 2; milestones: 2, plus
FFI-level round-trip tests for all three). Full simulator rebuilds
after each feature; the existing `testPlayEndlessGameMode` UI test was
rewritten in place to drive the new two-step onboarding chain and
assert the "Explain this" link lands back on the missed concept, and a
new `testFirstBuildCompletionCelebratesExactlyOnce` proves the
milestone fires once, then never again for a second completion of the
same build. All 7 UI tests pass together on the same iPhone 17 / iOS
26.5 runtime as prior phases.

## 21. Bookmarks, achievements, algorithms, and a launch splash

A grab-bag round driven directly by user feedback while using the app:
a genuinely dead schema field, a confusing layout, sparse screens, and
two explicit feature requests (a launch animation, "don't repeat
questions").

- **Bookmarks, actually wired up.** `SaveState.bookmarks` existed since
  schema v1 but `BitEngine` never read or wrote it — every save
  silently dropped it. Added `toggle_bookmark`/`is_bookmarked`/
  `bookmarked_concepts` to `BitEngine`, wired through `save`/`restore`,
  exposed via FFI, added a bookmark toolbar button to
  `ConceptDetailView`, and a "Bookmarks" section in Profile that
  navigates straight back to the concept (Profile now takes
  `onNavigateToExplore`, threaded from `RootView` the same way Play's
  suggestion sheet already did).
- **Achievements page**: Profile lists all three milestone ids
  (`AchievementDefinition.all`, a plain Swift list — Rust only ever
  needed to store/check an opaque id string) with locked/unlocked
  state, reusing `has_celebrated` with zero new Rust.
- **Build page's "empty space" was really a missing Section header.**
  A flat `List` with no `Section` still reserves the grouped-style top
  inset; with no header text there, it just read as unexplained blank
  space. Fixed by grouping builds by tier (Beginner -> Extreme, only
  tiers with a build shown) — the reserved space now holds a tier
  label, and two new builds (`pixel_art_studio`, Advanced — the app's
  first Advanced-tier build, so purple actually appears somewhere;
  `memory_simulator`, Core, byte-addressed framing over the same
  `ArraySim`/`array_builder` mechanic) round the catalog out to 7,
  giving every tier but Extreme at least one entry.
- **Domain descriptions**: `bit_knowledge::Domain` gained a
  `#[serde(default)]` `description` field, authored for all 13 domains
  (11 existing + 2 new), exposed via FFI (`FfiDomain.description`), and
  shown both as a subtitle under each folder row and as a caption atop
  the domain's own page — Explore's folders said only a name before
  this, never what was inside.
- **A new top-level Algorithms domain** (`algorithms` ->
  `algorithms.searching`): Linear Search and Binary Search, each with a
  genuinely new, non-reused challenge generator —
  `LinearSearchGenerator` (prints a random array, asks the index a
  left-to-right scan finds the target at, or -1) and
  `BinarySearchGenerator` (prints a random *sorted*, duplicate-free
  array, asks how many comparisons binary search needs — computed by
  actually running the standard `lo..=hi`, `mid=(lo+hi)/2` loop, never
  estimated, so the expected answer can't drift from the real
  algorithm). No new simulation needed — both are pure array-trace
  generators in the same family as `ArrayIndexGenerator`.
- **Lab got a 4th playground** ("Gates," reusing `GateDemoView` — the
  exact same FFI-backed widget Explore's gate concepts already use, so
  this cost zero new Rust or FFI) **and an explanatory caption per
  mode** — previously each mode was a bare interactive widget with no
  indication of what it was for.
- **Launch splash** (`SplashView.swift` + `LaunchView`): eight bits
  flip on in a staggered sequence — the exact demonstration Foundations
  opens with — then a "BIT" title fades in, then the whole thing
  scales/fades into `RootView`. ~1.4s total, timed explicitly (stagger
  0.07s x 8, settle 0.35s, hold 0.35s, exit 0.25s), not a loading
  screen — there's nothing to actually wait for, since content loads
  synchronously in `Engine.init`. Skipped entirely under
  `UITEST_SAVE_TOKEN` so it never adds dead time to a UI test run.
- **Play stops repeating questions.** `pickNewPuzzle` now keeps a
  6-prompt memory (`recentPrompts`) and re-rolls (new concept, new
  seed) up to 8 times if a freshly generated prompt matches one still
  in memory, falling back to the last attempt if every retry somehow
  collides. Also clarified, in place, what the difficulty dots actually
  mean (a caption + doc comment): the bar is the *lifetime* difficulty
  ramp tied to the level number, identical across every concept at a
  given level — it was being misread as a per-puzzle or per-concept
  meter, which it never was.

**Verification**: `cargo test --workspace` and `content-lint` green
throughout (content-lint: 13 domains / 26 concepts / 7 builds, all
instantiable). Full simulator rebuilds after each cluster of changes.
Two new UI tests — `testBookmarkingAConceptSurfacesItInProfile` and
`testAlgorithmsDomainAndDomainDescriptionsAreReachable` — join the
existing six (`testFirstBuildCompletionCelebratesExactlyOnce`'s tier-
grouped Build list still resolves "Binary Counter" by static text
unchanged), all 8 passing together on the same iPhone 17 / iOS 26.5
runtime as prior phases.

## 22. Notes, Bubble Sort, and two more milestones

- **Notes, wired up like bookmarks were.** `SaveState.notes` existed
  since schema v1 but `BitEngine` never touched it — the same dead-
  field pattern as bookmarks. Added `set_note`/`note_for` (an empty
  string clears the entry rather than storing blank text), wired
  through `save`/`restore`, exposed via FFI. `ConceptDetailView` grew a
  "Your notes" `TextEditor` — the "creation" pillar spec §2's PROFILE
  section calls for, previously entirely absent. Saves on losing focus
  and again on `onDisappear` (belt and suspenders, same reasoning as
  `Engine.persist()`'s background-transition save), not per keystroke.
- **Algorithms gained a Sorting sibling to Searching**: Bubble Sort,
  with a new `BubbleSortPassGenerator`. Its puzzle asks how many swaps
  happen in one full left-to-right pass — deliberately *not* "what does
  the array look like after," which would need the player to type a
  whole formatted array back, breaking both the app's short-answer
  convention and the `.numberPad` keyboard default from §20. The swap
  count is computed by actually running the pass.
- **Two more milestones** — "10 Concepts Discovered" and "7 Day
  Streak" — cost almost nothing thanks to §15's generic
  `has_celebrated`/`mark_celebrated` design: one new `BitEngine` method
  (`discovered_concept_count`, avoiding an O(n) `mastery_for` call per
  concept from Swift) and two `AchievementDefinition` entries. Checked
  from the same `PlayView.checkMilestones` every other milestone already
  uses.
- **A real bug this surfaced**: adding two more achievements pushed
  Profile's "Bookmarks" section further down its `List`, and the
  section stopped reliably appearing in the accessibility tree under
  full-suite load — not a flake, reproduced in isolation too. Root
  cause: List content grows over time (more achievements keep getting
  added), so anything below a growing section keeps drifting further
  from the top. Fixed by reordering Bookmarks *above* Achievements —
  a user's own curated list is more immediately actionable than a
  checklist of locked achievements anyway, so the fix is also the
  better design.

**Verification**: `cargo test --workspace` and `content-lint` green
(14 domains / 28 concepts / 7 builds). Full simulator rebuild. Two new
UI tests (`testPersonalNoteSurvivesNavigatingAwayAndBack`,
verifying the note editor round-trips through a real navigate-away-
and-back cycle) join the suite; all 9 pass together, including the
existing bookmark test once the section-ordering fix landed.

## 23. A game-like visual identity, project detail pages, and two new domains

The single biggest theme this round: "it looks like a plain white
list app, not a game." Addressed with a consistent, bounded set of
changes rather than a full redesign.

- **App-wide visual identity**: `Theme.brand` (a saturated electric
  violet, deliberately distinct from every domain/tier color already
  in use) set as the app-wide `.tint()` at `RootView`, cascading
  everywhere nothing more specific overrides it. `Theme.
  backgroundGradient` (a soft top-to-bottom tint, adaptive for dark
  mode via inline `UIColor` trait closures — no asset catalog needed)
  replaces the flat system white/black on every screen via a new
  `gameBackground()` view modifier; List-based screens additionally get
  `.scrollContentBackground(.hidden)` so the gradient shows through
  behind their rows. The splash screen's "BIT" title now uses this
  brand color in a monospaced design — a small, deliberate nod toward
  "retro computing terminal" rather than a full re-theme, which is
  flagged as a larger follow-up if wanted.
- **A real alignment bug, not just polish**: Profile's stat-card row
  had `.listRowInsets` explicitly zeroed out, making it span edge-to-
  edge while every other row (Bookmarks, Achievements, domain lists)
  kept the List's own default margins — the actual cause of "not
  aligned left/right." Fix: stop overriding insets at all, so every row
  inherits the same default. Same root cause, second occurrence, as
  the Build page empty-space fix in §21 — a Section without a header
  still reserves top-of-list space, which Profile's headerless first
  Section (now labeled "Overview") and Build's now-present intro
  banner both needed a real header/content for.
- **Activity heatmap visual upgrade**: sparse M/W/F weekday labels, a
  Less->More legend row, and a soft branded card background — same
  underlying data, meaningfully more informative and less bare.
- **BuildSessionView shows the complete project brief**, not just a
  bare interactive widget: tier badge, full description, and a new
  "Practices" list of the build's actual `concept_ids` (resolved via a
  new `build_related_concepts` FFI method mirroring `bookmarked_
  concepts`'s dangling-id-tolerant pattern) linking straight back to
  Explore.
- **ConceptDetailView's prose sections became floating cards** with a
  colored icon header each (Prerequisites/blue, "You'll learn to"
  /indigo, Real world/orange, Related/purple, Your progress/domain
  color, Your notes/brown) instead of plain stacked text under a bare
  title — the single biggest lever on how "plain" a concept page reads.
- **Build and Lab both gained an explanatory intro banner** stating
  outright what the tab is for and why it's worth using — previously
  implicit, now stated.
- **Two new, well-scoped content domains**, chosen to reuse existing
  mechanics wherever the underlying concept genuinely is the same one:
  - **Networking** -> IP Addresses: an IPv4 octet *is* a byte, so this
    concept reuses `GeneratorKind::BinaryConversion` outright — zero
    new Rust. The entire addition is content plus one match-arm line.
  - **Foundations.Arithmetic** -> Binary Addition: a genuinely new
    mechanic (ripple-carry addition), backed by a new
    `BinaryAdditionGenerator` that computes `a + b` directly and
    formats with `{:b}`, so the expected answer can never drift from
    real addition across any carry chain a seed produces. Content
    includes worked examples through a full carry chain (the "9 + 1 =
    10" analogy), matching the requested "theory with multiple
    examples" depth.
  - Subtraction, multiplication, division, and further domains
    (Security, Databases, Programming, Cloud, ...) remain natural next
    additions, deliberately not attempted in the same round to keep
    each addition at full quality rather than spreading thin.

**A real regression caught along the way**: growing Build's list (a
new intro banner + a 7th build) pushed "Array Builder" out of the
initially-materialized portion of a lazy `List`, failing even in
isolation — not a flake. Same class of bug as §22's Bookmarks-section
issue. Fixed in the test with an explicit `swipeUp()` rather than in
the view, since the underlying list-growing-over-time behavior here is
expected and correct, unlike §22 where reordering was the right product
fix.

**Verification**: `cargo test --workspace` and `content-lint` green
throughout (16 domains / 29 concepts / 7 builds). Full simulator
rebuilds after each cluster of changes. A new
`testNetworkingAndArithmeticContentIsReachable` joins the suite; all
10 UI tests pass together on a clean run (two incidental full-suite-
load flakes reproduced green in isolation, consistent with the flake
class documented since §15 — not regressions).

## 24. Binary Subtraction/Multiplication/Division, a first non-array
data structure, and a genuinely new sorting mechanic

Filled out the arithmetic quartet Binary Addition started, and added
one concept each to Data Structures and Algorithms, continuing §23's
"reuse the mechanic when it's genuinely the same puzzle" discipline.

- **Binary Subtraction, Multiplication, Division** — three new
  generators (`BinarySubtractionGenerator`, `BinaryMultiplication
  Generator`, `BinaryDivisionGenerator`), each computing the real
  operation and formatting the answer directly, so correctness can
  never drift from a hand-authored answer key. Subtraction is the
  interesting one: operands are capped so `a - b` always fits `i32`,
  and the result — which can go negative — is formatted through the
  existing `to_twos_complement_bits` helper (already used elsewhere
  for the `twos_complement` concept), deliberately reusing that
  representation rather than inventing a second one. This is also how
  the request for "negative binary conversions" got satisfied: rather
  than a separate concept, Binary Subtraction's puzzle sometimes hands
  you `b > a` and expects the real two's complement bit pattern of the
  negative result, with the content's "When the result would go
  negative" section walking through `3 - 5 = -2 -> 11111110` by hand.
- **Linked List reuses `ArrayIndexGenerator`** — traversing from the
  head to a position is the same "find the value at index N" puzzle an
  array asks, just walked instead of jumped. The generator was made
  concept-aware (checking `ctx.template.concept_ids.first()`, the same
  pattern `GateEvalGenerator` already used to pick its gate kind) so a
  linked-list question prints "Linked list, head to tail: [...]"
  instead of "Array: [...]" — reusing the mechanism without producing
  a misleading prompt. This is the precedent worth keeping: reusing a
  generator for a new concept is fine, but the prompt text still has to
  match what's actually being asked about.
- **Selection Sort is a genuinely new mechanic**, not a reuse — unlike
  Bubble Sort's adjacent-pair passes, one pass scans the *entire*
  remaining array for its minimum and swaps it to the front. Content
  contrasts the two directly: Bubble Sort can finish early on an
  already-sorted array, Selection Sort never can, because "find the
  minimum of what's left" always means a full scan. The puzzle asks
  only for the resulting minimum value (not the whole post-swap array),
  matching the short-answer convention every other generator in this
  codebase already follows.
- **A real gap surfaced while extending UI test coverage, not fixed in
  this round**: `Concept.explanation` (the `.md` filename, e.g.
  `"binary_subtraction.md"`) is serialized into `concept_detail_json`
  and reaches the Swift side, but `ConceptDetailView`'s `ConceptDetail`
  struct never decodes it, and no code path anywhere loads the file's
  actual prose. Every `.md` file written across every content phase so
  far — worked examples, "why this matters" sections, the new
  negative-result walkthrough included — is real, lint-validated
  content that is currently invisible in the app; only the structured
  RON fields (learning objectives, real-world uses, prerequisites,
  related) render. Caught because a UI test assertion against a
  markdown heading ("When the result would go negative") failed
  consistently, even in isolation — not a flake, a wrong assumption
  about what the app renders. The test was corrected to check the
  fields that actually appear (Real world, prerequisite links);
  wiring the markdown body into the app itself is unstarted and
  flagged as a real follow-up, not a small one — it needs the `.md`
  files bundled as app resources (or piped through FFI as raw text),
  a markdown renderer in SwiftUI, and a decision about where in
  `ConceptDetailView`'s card layout the prose belongs.

**Verification**: `cargo test --workspace` and `content-lint` green
throughout (16 domains / 34 concepts / 7 builds — up from 29). Full
simulator rebuild. Two new/extended UI tests
(`testNetworkingAndArithmeticContentIsReachable` extended to also
check the Subtraction/Multiplication/Division trio and drill into
Binary Subtraction; a new `testLinkedListAndSelectionSortAreReachable`)
both failed on first full-suite run — verified as real bugs in the
test code itself (a wrong markdown-rendering assumption, and an
under-counted back-navigation tap in a 3-levels-deep domain path), not
app regressions, by reproducing and fixing each in isolation. All 12
UI tests pass together on a clean run afterward.

One more flake surfaced during this round's isolation reruns, unrelated
to any content change: `testPlayEndlessGameMode` failed once (of four
runs total) with "Neither element nor any descendant has keyboard
focus" while typing into `play-answer-field` — a text-field-focus
synthesis race in the simulator, not a real bug in the app. Passed on
every other run, including the final clean 12/12. Noted here rather
than chased, matching the existing flake-tolerance policy from §15.

## 25. Retention pass: Puzzle of the Day, streak reminders, combo
scoring, and a distribution-readiness sweep

A shift in focus for this round: not new content, but keeping a daily
player coming back, plus preparing the project to actually ship. Four
pieces, each independently scoped.

- **Pre-distribution bug sweep, done first**: `cargo clippy --workspace
  --all-targets` (13 purely cosmetic `is_multiple_of`/cast lints,
  auto-fixed, zero behavior change, reconfirmed via `cargo test
  --workspace`), a scan for `TODO`/`FIXME`/force-unwraps/`try!`/`as!`
  across the Swift app target (none found — the only `fatalError`s are
  justified unrecoverable-startup-state cases), and a check of the
  App Store-relevant project settings. Found and fixed: no explicit
  `MARKETING_VERSION`/`CURRENT_PROJECT_VERSION` (now `1.0.0`/`1`), and
  `CODE_SIGNING_ALLOWED/REQUIRED = NO` on every config, meaning the
  project could only ever build for the Simulator. Fixed for Release
  only (added `DEVELOPMENT_TEAM`, flipped signing to `YES`) — Debug was
  left untouched deliberately, so every existing simulator-only script
  (`build-ios-app.sh`, `run-ios-ui-tests.sh`) keeps working exactly as
  before, verified by a full rebuild + full UI suite pass immediately
  after the edit.
- **Puzzle of the Day**: a genuinely new `BitEngine::daily_challenge`
  method — sorts every practicable concept's id (stable regardless of
  content load order), indexes by `days_since_epoch(today) modulo pool
  size`, and generates a challenge from that `(concept_id, seed)` at a
  fixed difficulty. Zero RNG, zero player-state dependency: every
  install gets the identical puzzle on the identical date, which is the
  entire point — a shareable result only means something if it's
  comparable across players, the same reason Wordle's word is global,
  not personalized. The user-facing "Day #N" is deliberately *not* the
  raw `days_since_epoch` value (that produces something absurd like
  "Day #20,698" on launch day) — a separate `daily_challenge_launch_
  anchor` constant (`2026-01-01`) offsets only the *displayed* number;
  concept/seed selection still keys off the raw value so it never has
  to change again. Answering reuses `validate_challenge` outright — no
  bespoke daily-only validation path. A new `daily_challenge_results:
  HashMap<String, bool>` save field tracks same-day idempotent
  attempts, and `record_daily_challenge_result` also calls `record_
  daily_activity`, so solving the daily puzzle alone is enough to keep
  the regular calendar streak alive — one system, not two competing
  streaks. On iOS: `DailyChallengeCard` (a compact entry point embedded
  at the top of Play, visible from the very first launch since it
  doesn't depend on `gamePool`/discovery) opens `DailyChallengeView`, a
  sheet with the answering flow and a Wordle-style plain-text
  `ShareLink` result ("BIT Puzzle of the Day #N / ✅ or ❌ concept /
  🔥 N-day streak") — text, not an image, matching how Wordle itself
  actually shares.
- **Local streak-reminder notifications**: `StreakReminders` (a plain
  enum, no class needed) wraps `UNUserNotificationCenter` behind three
  calls — `enable`/`disable`/`reschedule`. Opt-in only, via a new
  "Reminders" toggle in Profile (no cold-open permission prompt).
  `reschedule` always removes the one reusable pending notification
  first, then re-adds it only if reminders are on *and* today has no
  recorded activity yet (checked via `activityHistory()`, no new FFI
  surface needed) — so a player who already engaged today never gets
  nagged, and disabling never leaves a stale notification behind. Wired
  into the single `Engine.persist()` chokepoint (every mutating call
  already routes through it) plus app-foreground, rather than
  sprinkled across every `record*` call site.
- **In-session combo scoring for Play**: deliberately presentational,
  not a new scoring dimension — `comboMultiplier` is a pure function of
  the existing in-session `streak` int (coarse tiers: ×1/×2/×3/×4, not
  1:1 with streak, so it reads as an occasional milestone rather than a
  number that churns every answer), shown as a pulsing badge next to
  the existing flame/streak indicator with a distinct heavier haptic
  (`Haptics.comboUp()`) on a tier-up. Never touches `gameTotalCorrect`,
  never persisted — resets for free the instant `streak` resets on a
  miss. No Rust changes at all for this one.
- **Home screen widget — deliberately not built this round.** Needs a
  new WidgetKit extension target (its own `PBXNativeTarget`, build
  phases, embed-extension phase) plus an App Group entitlement shared
  between it and the main app so it can read streak data — and this
  project has no git, so a bad hand-edit to `project.pbxproj` has no
  undo. App Group registration also normally requires Xcode itself
  signed into the developer account to provision with Apple's servers,
  which can't be driven headlessly. Given the choice, added the *safe*
  half now — `Engine.persist()` mirrors `dailyStreak`/`dayNumber`/
  `solvedToday` into `UserDefaults(suiteName: "group.dev.bit.app")`
  (a harmless no-op until the App Group actually exists) via a new
  `SharedWidgetData` constants file, and a ready-to-paste widget
  implementation (`TimelineProvider`, view, `WidgetBundle`) staged at
  `ios/BitWidgetSource/BitWidget.swift` — left unbuilt until the target
  exists, since a fresh Widget Extension target isn't required to reuse
  it, and manually reconstructing one via raw `pbxproj` text was judged
  too risky without version control as a safety net.

**Verification**: `cargo test --workspace` green throughout (146 tests
across 9 crates; `bit-core` grew 37 -> 42 for the new daily-challenge
methods). `cargo clippy --workspace --all-targets` clean (0 warnings).
Full simulator rebuild after every cluster of changes. One real,
non-flaky regression caught and fixed: adding Profile's new Reminders
section pushed Achievements out of the initially-materialized portion
of the lazy `List` — same class of bug as §21/§23, fixed with an
explicit `swipeUp()` in the test, reproduced failing in isolation
first to confirm it wasn't the usual full-suite-load flake. All 13 UI
tests (two new: Puzzle of the Day end-to-end, plus the extended
bookmark test) pass together on a clean run.

## 26. The widget, finished — and why `dev.bit.app` had to be renamed

§25's widget was left half-built on purpose: the remaining step (a new
`PBXNativeTarget`, an App Group entitlement) needed Xcode itself,
signed into a real account, to provision anything with Apple's
servers — not something drivable headlessly, and this project has no
git, so hand-editing `project.pbxproj` for something that large wasn't
worth the risk. The user created the `BitWidgetExtension` target
through Xcode's own UI (File > New > Target > Widget Extension); from
there the rest finished as a normal session.

- **Boilerplate replaced**, not the extra generated files removed —
  Xcode's template included Configuration Intent, a Control Widget, and
  a Live Activity, none of which this feature needs. Deleting file
  *references* from `project.pbxproj` by hand was judged riskier than
  it was worth for a cosmetic cleanup, so `BitWidgetControl.swift` and
  `BitWidgetLiveActivity.swift` were emptied to harmless stub comments
  instead, and `BitWidget.swift`/`BitWidgetBundle.swift` got the real
  `StaticConfiguration`-based implementation staged back in §25.
- **A real, unrelated blocker surfaced along the way**: the bundle
  identifier `dev.bit.app` turned out to already be registered to a
  *different* Apple developer account — App ID namespace is global
  across every developer, not per-account, so this had nothing to do
  with App Groups specifically and would have blocked real App Store
  submission regardless. Renamed everywhere (`PRODUCT_BUNDLE_
  IDENTIFIER` for all three targets, both `.entitlements` files, the
  App Group id itself, `SharedWidgetData.suiteName` on both sides, and
  `build-ios-app.sh`'s launch hint) to `dev.kartiksanil.bit` / `group.
  dev.kartiksanil.bit` — safe because the app had never actually been
  submitted or distributed under the old id.
- **A second, quieter blocker**: even after the rename, `codesign -d
  --entitlements` on the built app showed an empty entitlements dict —
  `CODE_SIGNING_ALLOWED = NO` (set back when this project had no team
  at all, purely to let simulator builds succeed unsigned) meant Xcode
  skipped codesigning entirely, so the App Group entitlement was never
  actually embedded no matter what Signing & Capabilities showed.
  Fixed by flipping `CODE_SIGNING_ALLOWED`/`REQUIRED` to `YES` on
  *Debug* too (previously only Release had been flipped, in §25) for
  both the BitApp and BitWidgetExtension targets — Simulator doesn't
  require a real provisioning profile to honor entitlements, just an
  actual signature, which "Sign to Run Locally" (Xcode's automatic
  choice once a team is set) provides for free.
- **Verified for real, not just "build succeeded"**: `codesign -d
  --entitlements` on the build product still isn't sufficient proof —
  `xcodebuild` from the CLI can't perform the online App Group
  registration Xcode's GUI does, so that check kept coming back empty
  even after the fixes above, until the user ran the app once from
  Xcode itself. After that, `xcrun simctl listapps` showed a real
  `GroupContainers` entry, and — after driving the app through one
  real interaction via a UI test on that exact simulator (a plain
  launch never calls `persist()`, nothing gets written until something
  actually mutates state) — `group.dev.kartiksanil.bit.plist` appeared
  in the shared container with real values: `dailyStreak: 1,
  dayNumber: 245, solvedToday: true`. The widget reads real data.

**The actual cause of the "Failed to show Widget" error, found via
`log show`**: not stale simulator state after all — the widget target
inherited `IPHONEOS_DEPLOYMENT_TARGET = 26.5` from Xcode's template
default (matching the installed SDK), while the simulator being run
was iOS 26.4, one version behind. `installd`'s own log line said it
outright: "Ignoring app extension ... because it doesn't work on this
OS version" — the OS silently filtered the `.appex` out at install
time, so `pkd` (PlugInKitDaemon) could never find an extension record
for it, `chronod` had nothing to fetch descriptors for, and
SpringBoard's attempt to show it failed several layers up the call
stack from the real problem. Fixed by lowering the widget's deployment
target to `17.0`, matching the main app (WidgetKit's `containerBackground
(for:)` API used here only needs iOS 17 anyway). Confirmed via
`pluginkit -m -p com.apple.widgetkit-extension` — `dev.kartiksanil.bit.
BitWidget` now actually appears in the registered-extensions list
(the leading `!` there is normal for any non-Apple extension, not a
fault marker). The widget is now addable from a simulator's home
screen and renders live streak data.

**Verification**: full `run-ios-ui-tests.sh` (13/13) and `cargo test
--workspace` (146 tests) both green after every change in this
section, confirmed with a final full pass once the deployment-target
fix landed.
