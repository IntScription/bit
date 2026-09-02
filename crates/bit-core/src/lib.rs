//! Facade crate: composes knowledge, content, challenge, progression, and
//! persistence into the single `BitEngine` surface that `bit-ffi` exposes
//! to Swift. See /ARCHITECTURE.md §1 and §10.
//!
//! Screens should never reach past `BitEngine` into an inner crate
//! directly — that's what keeps the FFI surface stable while the engines
//! underneath evolve.

use std::collections::HashMap;

use bit_build::catalog::{self, BuildFactoryError, LiveBuildSession};
use bit_build::BuildDefinition;
use bit_challenge::{Answer, Challenge, ChallengeRegistry, ChallengeTemplate, DifficultyLevel, GenerationContext, GeneratorKind, MasterySnapshot, ValidationResult};
use bit_content::{load_all, ContentError, ContentSource};
use bit_knowledge::{Concept, KnowledgeGraph};
use bit_progression::{MasteryState, StageThresholds};

/// Correct answers accumulated since the player last saw a "ready for
/// something new?" suggestion (a wrong answer in between doesn't reset
/// this — one miss shouldn't erase progress toward the next suggestion).
/// Picked from the middle of spec's stated "10-20 correct answers" range.
/// Tunable; not load-bearing elsewhere.
const NEW_TOPIC_SUGGESTION_THRESHOLD: u32 = 15;

pub struct BitEngine {
    graph: KnowledgeGraph,
    builds: Vec<BuildDefinition>,
    challenges: ChallengeRegistry,
    mastery: HashMap<String, MasteryState>,
    game_total_correct: u32,
    game_streak_since_suggestion: u32,
    last_active_day: Option<String>,
    daily_streak: u32,
    longest_daily_streak: u32,
    activity_by_day: HashMap<String, u32>,
    celebrated_milestones: Vec<String>,
    bookmarks: Vec<String>,
    notes: HashMap<String, String>,
    daily_challenge_results: HashMap<String, bool>,
}

/// A single, date-seeded challenge every install generates identically —
/// same concept, same puzzle, same day, everywhere. `day_number` is the
/// `days_since_epoch` value for `today`, exposed so the UI can render a
/// stable "Day #N" the way Wordle numbers its puzzles.
pub struct DailyChallenge {
    pub day_number: u32,
    pub concept_id: String,
    pub difficulty: u8,
    pub challenge: Challenge,
}

/// Fixed rather than difficulty-scaled: everyone should be solving the
/// *same* puzzle each day for the share card to mean anything, so there's
/// no per-player knob here the way Play's endless mode has one.
const DAILY_CHALLENGE_DIFFICULTY: u8 = 3;

/// Day 1 of the Puzzle of the Day's user-facing numbering. Deliberately
/// separate from `days_since_epoch`'s own epoch, which is a proleptic-
/// calendar implementation detail — using it directly would show
/// something absurd like "Day #20,698" on launch day. Concept/seed
/// selection still keys off the raw `days_since_epoch` value (so it never
/// has to change again); only the number actually shown to players is
/// anchored here.
fn daily_challenge_launch_anchor() -> i64 {
    days_since_epoch("2026-01-01").expect("hardcoded date always parses")
}

/// Days since the Unix epoch for a "YYYY-MM-DD" date (Howard Hinnant's
/// `days_from_civil` algorithm) — dependency-free date math so the daily
/// streak stays pure and unit-testable like everything else in this
/// crate. "What day is it right now" is deliberately never asked here:
/// Swift supplies `today` as a plain string, the same way callers supply
/// a seed to the challenge engine instead of it reading the clock itself.
fn days_since_epoch(date: &str) -> Option<i64> {
    let mut parts = date.splitn(3, '-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

impl BitEngine {
    pub fn new(source: &dyn ContentSource) -> Result<Self, ContentError> {
        let loaded = load_all(source)?;
        Ok(Self {
            graph: loaded.graph,
            builds: loaded.builds,
            challenges: bit_challenge::default_registry(),
            mastery: HashMap::new(),
            game_total_correct: 0,
            game_streak_since_suggestion: 0,
            last_active_day: None,
            daily_streak: 0,
            longest_daily_streak: 0,
            activity_by_day: HashMap::new(),
            celebrated_milestones: Vec::new(),
            bookmarks: Vec::new(),
            notes: HashMap::new(),
            daily_challenge_results: HashMap::new(),
        })
    }

    pub fn challenges_mut(&mut self) -> &mut ChallengeRegistry {
        &mut self.challenges
    }

    pub fn graph(&self) -> &KnowledgeGraph {
        &self.graph
    }

    pub fn builds(&self) -> &[BuildDefinition] {
        &self.builds
    }

    pub fn start_build(&self, build_id: &str) -> Result<LiveBuildSession, BuildFactoryError> {
        let definition = self
            .builds
            .iter()
            .find(|b| b.id == build_id)
            .ok_or_else(|| BuildFactoryError::UnknownBuildId(build_id.to_string()))?;
        catalog::instantiate(definition)
    }

    pub fn mastery_for(&self, concept_id: &str) -> MasteryState {
        self.mastery.get(concept_id).cloned().unwrap_or_default()
    }

    /// How many concepts have been discovered, total — cheaper than
    /// having a caller call `mastery_for` once per concept just to
    /// count `.discovered` flags (Profile's discovered-count stat and
    /// the "10 Concepts Discovered" milestone both want this).
    pub fn discovered_concept_count(&self) -> usize {
        self.mastery.values().filter(|m| m.discovered).count()
    }

    pub fn record_discovery(&mut self, concept_id: &str) {
        self.mastery.entry(concept_id.to_string()).or_default().record_discovery();
    }

    /// Toggles a concept's bookmark on/off — save-worthy state has lived
    /// in `SaveState.bookmarks` since schema v1, but nothing ever wrote
    /// to or read it until now.
    pub fn toggle_bookmark(&mut self, concept_id: &str) {
        match self.bookmarks.iter().position(|b| b == concept_id) {
            Some(index) => {
                self.bookmarks.remove(index);
            }
            None => self.bookmarks.push(concept_id.to_string()),
        }
    }

    pub fn is_bookmarked(&self, concept_id: &str) -> bool {
        self.bookmarks.iter().any(|b| b == concept_id)
    }

    /// Bookmarked concepts, in the order they were bookmarked — dangling
    /// ids (a bookmark surviving past a concept being removed from
    /// content) are silently skipped rather than surfaced as an error;
    /// there's nothing actionable a caller could do with them.
    pub fn bookmarked_concepts(&self) -> Vec<&Concept> {
        self.bookmarks.iter().filter_map(|id| self.graph.concept(id)).collect()
    }

    /// A blank note removes the entry entirely rather than storing an
    /// empty string — keeps `notes` (and the save file) exactly as big
    /// as the player's actual writing, and makes "has a note" a plain
    /// `contains_key` check for callers.
    pub fn set_note(&mut self, concept_id: &str, text: &str) {
        if text.is_empty() {
            self.notes.remove(concept_id);
        } else {
            self.notes.insert(concept_id.to_string(), text.to_string());
        }
    }

    pub fn note_for(&self, concept_id: &str) -> Option<&str> {
        self.notes.get(concept_id).map(String::as_str)
    }

    /// `today` ("YYYY-MM-DD") stamps `MasteryState::last_practiced_day` —
    /// the seam `is_rusty` below reads to decide whether a concept is
    /// due for review. Same wall-clock-free pattern as
    /// `record_daily_activity`: Swift supplies the date, Rust just does
    /// pure comparisons with it.
    pub fn record_attempt(&mut self, concept_id: &str, correct: bool, today: &str) {
        let thresholds = self
            .graph
            .concept(concept_id)
            .map(|c| StageThresholds { min_practice_correct: c.mastery.min_practice_correct, min_accuracy: c.mastery.min_accuracy })
            .unwrap_or(StageThresholds { min_practice_correct: u32::MAX, min_accuracy: 1.0 });
        let state = self.mastery.entry(concept_id.to_string()).or_default();
        state.record_attempt(correct);
        state.advance_stage(thresholds);
        state.last_practiced_day = Some(today.to_string());
    }

    /// Whether a discovered concept is overdue for review: it has a
    /// content-authored `decay_days`, it's actually been practiced at
    /// least once, and at least that many calendar days have passed
    /// since. Concepts with no `decay_days` (or never practiced) are
    /// never rusty — there's nothing to decay yet. Play's puzzle pool
    /// uses this to weight selection toward what's going stale, instead
    /// of a flat random pick across everything discovered.
    pub fn is_rusty(&self, concept_id: &str, today: &str) -> bool {
        let Some(decay_days) = self.graph.concept(concept_id).and_then(|c| c.mastery.decay_days) else {
            return false;
        };
        let mastery = self.mastery_for(concept_id);
        let Some(last) = mastery.last_practiced_day.as_deref() else { return false };
        let (Some(today_days), Some(last_days)) = (days_since_epoch(today), days_since_epoch(last)) else {
            return false;
        };
        today_days > last_days && (today_days - last_days) as u32 >= decay_days
    }

    pub fn round_stage_for(&self, concept_id: &str) -> bit_progression::RoundStage {
        bit_progression::round_stage_for(&self.mastery_for(concept_id))
    }

    /// Every concept Play can actually offer a challenge for — i.e. every
    /// concept `GeneratorKind::default_for_concept` resolves. Difficulty
    /// 1 / seed 0 is just a probe; the result doesn't depend on either.
    pub fn practicable_concepts(&self) -> Vec<&Concept> {
        self.graph.all_concepts().into_iter().filter(|c| self.generate_challenge(&c.id, 1, 0).is_some()).collect()
    }

    /// The endless game mode's question pool: practicable concepts the
    /// player has actually discovered. Deliberately *not* "every
    /// practicable concept" — the whole point is that the puzzle mix
    /// only ever reflects what's been learned so far.
    pub fn game_pool(&self) -> Vec<&Concept> {
        self.practicable_concepts().into_iter().filter(|c| self.mastery_for(&c.id).discovered).collect()
    }

    /// A concept the player hasn't discovered yet, but whose every
    /// prerequisite they have — i.e. genuinely ready to learn next, not
    /// just "anything undiscovered." Ties broken by difficulty (easiest
    /// first), then id for determinism. `None` when nothing is unlocked
    /// yet (still on the very first concept) or everything reachable has
    /// already been discovered.
    pub fn next_learnable_concept(&self) -> Option<&Concept> {
        let mut candidates: Vec<&Concept> = self
            .graph
            .all_concepts()
            .into_iter()
            .filter(|c| {
                !self.mastery_for(&c.id).discovered
                    && self.graph.prerequisites(&c.id).iter().all(|p| self.mastery_for(&p.id).discovered)
            })
            .collect();
        candidates.sort_by_key(|c| (c.difficulty.rank(), c.id.clone()));
        candidates.into_iter().next()
    }

    /// Call once per *correct* answer in Play's game mode (separate from
    /// `record_attempt`, which callers should still call too, for
    /// per-concept mastery — this only tracks the global "level number").
    pub fn record_game_correct_answer(&mut self) {
        self.game_total_correct += 1;
        self.game_streak_since_suggestion += 1;
    }

    pub fn game_total_correct(&self) -> u32 {
        self.game_total_correct
    }

    /// Whether Play should offer a "ready for something new?" prompt right
    /// now — both the streak threshold is met *and* there's actually
    /// something new to suggest (no point prompting into a dead end).
    pub fn should_suggest_new_topic(&self) -> bool {
        self.game_streak_since_suggestion >= NEW_TOPIC_SUGGESTION_THRESHOLD && self.next_learnable_concept().is_some()
    }

    /// Resets the streak counter once the player has seen the suggestion
    /// (whether they acted on it or dismissed it) — call this right after
    /// showing the prompt, not only if they accept it, so a dismissal
    /// doesn't immediately re-trigger the same prompt next question.
    pub fn acknowledge_new_topic_suggestion(&mut self) {
        self.game_streak_since_suggestion = 0;
    }

    /// Call with today's date (`"YYYY-MM-DD"`, caller's local calendar
    /// day) whenever the player does something that counts as real
    /// engagement — a correct answer, a discovery. Idempotent within a
    /// single day: calling it five times on the same date only counts
    /// once. Consecutive calendar days increment the streak; a gap of two
    /// or more days resets it to 1; an out-of-order date (clock skew, or
    /// a date earlier than the last recorded one) is ignored entirely
    /// rather than corrupting the streak.
    pub fn record_daily_activity(&mut self, today: &str) {
        // Tallied unconditionally (even a same-day repeat call, unlike
        // the streak below) — the heatmap's whole point is "how much did
        // you do today," which a second correct answer in the same
        // session genuinely adds to.
        if days_since_epoch(today).is_some() {
            *self.activity_by_day.entry(today.to_string()).or_insert(0) += 1;
        }

        let Some(today_days) = days_since_epoch(today) else { return };
        let last_days = self.last_active_day.as_deref().and_then(days_since_epoch);
        match last_days {
            None => self.daily_streak = 1,
            Some(last) if today_days == last => return, // already counted today
            Some(last) if today_days == last + 1 => self.daily_streak += 1,
            Some(last) if today_days > last => self.daily_streak = 1,
            Some(_) => return, // today is before the last recorded day; ignore
        }
        self.last_active_day = Some(today.to_string());
        self.longest_daily_streak = self.longest_daily_streak.max(self.daily_streak);
    }

    pub fn daily_streak(&self) -> u32 {
        self.daily_streak
    }

    pub fn longest_daily_streak(&self) -> u32 {
        self.longest_daily_streak
    }

    /// Today's Puzzle of the Day — the same concept, at the same
    /// difficulty, with the same seed, on every install, so results are
    /// actually comparable across players. Concept choice: sort every
    /// practicable concept's id (stable regardless of content load
    /// order) and index by `day_number modulo pool size`, so the pick is
    /// a pure function of the date and the bundled content, no RNG state
    /// or player history involved. `None` only if the content bundle has
    /// no practicable concepts at all, or `today` doesn't parse.
    pub fn daily_challenge(&self, today: &str) -> Option<DailyChallenge> {
        let day_number = days_since_epoch(today)?;
        let mut ids: Vec<&str> = self.practicable_concepts().iter().map(|c| c.id.as_str()).collect();
        if ids.is_empty() {
            return None;
        }
        ids.sort_unstable();
        let index = (day_number.rem_euclid(ids.len() as i64)) as usize;
        let concept_id = ids[index].to_string();
        let seed = day_number as u64;
        let challenge = self.generate_challenge(&concept_id, DAILY_CHALLENGE_DIFFICULTY, seed)?;
        let display_day_number = (day_number - daily_challenge_launch_anchor() + 1).max(1) as u32;
        Some(DailyChallenge { day_number: display_day_number, concept_id, difficulty: DAILY_CHALLENGE_DIFFICULTY, challenge })
    }

    /// Records whether `today`'s daily challenge was answered correctly.
    /// Idempotent the same way `record_daily_activity` is — a second call
    /// the same day just overwrites the same key with the latest result,
    /// so re-checking an already-answered puzzle can't corrupt anything.
    /// Also feeds `record_daily_activity`, so solving the daily puzzle
    /// alone is enough to keep the regular daily streak alive.
    pub fn record_daily_challenge_result(&mut self, today: &str, correct: bool) {
        self.daily_challenge_results.insert(today.to_string(), correct);
        self.record_daily_activity(today);
    }

    /// `None` means not yet attempted today; `Some(correct)` otherwise.
    pub fn daily_challenge_result(&self, today: &str) -> Option<bool> {
        self.daily_challenge_results.get(today).copied()
    }

    /// Every day with recorded activity, unsorted — callers (bit-ffi)
    /// decide how to order/paginate for their UI.
    pub fn activity_by_day(&self) -> &HashMap<String, u32> {
        &self.activity_by_day
    }

    /// Whether a one-time celebration (id is caller-defined, e.g.
    /// "first_mastery") has already been shown — a generic flag set
    /// rather than bespoke fields per milestone, so adding a new
    /// milestone never touches this crate or the save schema.
    pub fn has_celebrated(&self, id: &str) -> bool {
        self.celebrated_milestones.iter().any(|m| m == id)
    }

    /// Idempotent: marking an already-celebrated id again is a no-op.
    pub fn mark_celebrated(&mut self, id: &str) {
        if !self.has_celebrated(id) {
            self.celebrated_milestones.push(id.to_string());
        }
    }

    /// Builds a `ChallengeTemplate` on the fly from just a concept id (via
    /// `GeneratorKind::default_for_concept`) and generates one challenge
    /// from it. Returns `None` when the concept has no default generator
    /// yet — not every concept in Explore is practicable in Play.
    pub fn generate_challenge(&self, concept_id: &str, difficulty: u8, seed: u64) -> Option<Challenge> {
        let ctx = self.challenge_context(concept_id, difficulty, seed)?;
        self.challenges.generate(&ctx)
    }

    /// Re-derives the same challenge from `(concept_id, difficulty, seed)`
    /// — generation is deterministic, so there's no need to keep a live
    /// challenge session around just to validate an answer against it.
    pub fn validate_challenge(
        &self,
        concept_id: &str,
        difficulty: u8,
        seed: u64,
        answer: &str,
    ) -> Option<ValidationResult> {
        let ctx = self.challenge_context(concept_id, difficulty, seed)?;
        let kind = ctx.template.generator;
        let challenge = self.challenges.generate(&ctx)?;
        self.challenges.validate(kind, &challenge, &Answer(answer.to_string()))
    }

    fn challenge_context(&self, concept_id: &str, difficulty: u8, seed: u64) -> Option<GenerationContext> {
        let generator = GeneratorKind::default_for_concept(concept_id)?;
        Some(GenerationContext {
            seed,
            difficulty: DifficultyLevel(difficulty),
            mastery: MasterySnapshot::default(),
            template: ChallengeTemplate {
                id: format!("{concept_id}_practice"),
                generator,
                concept_ids: vec![concept_id.to_string()],
            },
        })
    }

    pub fn save(&self) -> bit_persistence::SaveState {
        let mut save = bit_persistence::SaveState::new();
        save.mastery = self.mastery.clone();
        save.game_total_correct = self.game_total_correct;
        save.game_streak_since_suggestion = self.game_streak_since_suggestion;
        save.last_active_day = self.last_active_day.clone();
        save.daily_streak = self.daily_streak;
        save.longest_daily_streak = self.longest_daily_streak;
        save.activity_by_day = self.activity_by_day.clone();
        save.celebrated_milestones = self.celebrated_milestones.clone();
        save.bookmarks = self.bookmarks.clone();
        save.notes = self.notes.clone();
        save.daily_challenge_results = self.daily_challenge_results.clone();
        save
    }

    pub fn restore(&mut self, save: bit_persistence::SaveState) {
        self.mastery = save.mastery;
        self.game_total_correct = save.game_total_correct;
        self.game_streak_since_suggestion = save.game_streak_since_suggestion;
        self.last_active_day = save.last_active_day;
        self.daily_streak = save.daily_streak;
        self.longest_daily_streak = save.longest_daily_streak;
        self.activity_by_day = save.activity_by_day;
        self.celebrated_milestones = save.celebrated_milestones;
        self.bookmarks = save.bookmarks;
        self.notes = save.notes;
        self.daily_challenge_results = save.daily_challenge_results;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_content::MemorySource;

    fn source() -> MemorySource {
        MemorySource::new()
            .add(
                "domains",
                "ron",
                r#"Domain(id: "foundations", name: "Foundations", parent: None, order: 0)"#,
            )
            .add(
                "concepts",
                "ron",
                r#"Concept(
                    id: "bits",
                    title: "Bits",
                    domain: "foundations",
                    difficulty: Easy,
                    applicability: [Core],
                    prerequisites: [],
                    related: [],
                    learning_objectives: ["Understand a bit as a single binary digit"],
                    explanation: "bits.md",
                    interactive_demo: None,
                    practice_types: [],
                    build_refs: [],
                    lab_capabilities: [],
                    real_world_uses: ["Digital electronics"],
                    historical_relevance: None,
                    mastery: (min_practice_correct: 3, min_accuracy: 0.8, decay_days: None),
                )"#,
            )
            .add(
                "builds",
                "ron",
                r#"BuildDefinition(
                    id: "binary_counter",
                    name: "Binary Counter",
                    tier: Beginner,
                    description: "Count in binary",
                    concept_ids: ["bits"],
                    params: {"bit_width": "8", "target_value": "10"},
                    freeform: false,
                )"#,
            )
    }

    #[test]
    fn engine_exposes_and_can_start_a_loaded_build() {
        let source = source();
        let engine = BitEngine::new(&source).expect("engine should load from content");

        assert_eq!(engine.builds().len(), 1);
        assert_eq!(engine.builds()[0].id, "binary_counter");

        let session = engine.start_build("binary_counter").expect("build should instantiate");
        assert!(matches!(session, LiveBuildSession::BinaryCircuit(_)));

        assert!(engine.start_build("nonexistent").is_err());
    }

    #[test]
    fn engine_generates_and_validates_a_challenge_for_a_practicable_concept() {
        let source = source();
        let engine = BitEngine::new(&source).expect("engine should load from content");

        // "bits" has no default generator (see GeneratorKind::default_for_concept).
        assert!(engine.generate_challenge("bits", 4, 1).is_none());

        // The test source's concept graph only defines "bits", not
        // "binary" — but the generator mapping itself only needs the
        // concept id string, so this still proves the plumbing works
        // without requiring a full content graph.
        let challenge = engine.generate_challenge("binary", 4, 1).expect("binary should be practicable");
        let correct = engine
            .validate_challenge("binary", 4, 1, &challenge.expected_answer)
            .expect("should validate");
        assert!(correct.correct);

        let wrong = engine.validate_challenge("binary", 4, 1, "not-an-answer").expect("should validate");
        assert!(!wrong.correct);
    }

    #[test]
    fn record_attempt_eventually_advances_stage_to_unlock_timed_challenge() {
        let source = source();
        let mut engine = BitEngine::new(&source).expect("engine should load from content");

        // "bits" content's mastery rule: min_practice_correct: 3, min_accuracy: 0.8.
        engine.record_discovery("bits");
        assert_eq!(engine.round_stage_for("bits"), bit_progression::RoundStage::Fiddle);

        engine.record_attempt("bits", true, "2026-09-01");
        assert_eq!(engine.round_stage_for("bits"), bit_progression::RoundStage::GuidedPractice);

        engine.record_attempt("bits", true, "2026-09-01");
        engine.record_attempt("bits", true, "2026-09-01");
        // 3/3 correct meets both thresholds -> Proficient -> TimedChallenge unlocked.
        assert_eq!(engine.round_stage_for("bits"), bit_progression::RoundStage::TimedChallenge);
    }

    #[test]
    fn practicable_concepts_excludes_concepts_without_a_default_generator() {
        let source = source();
        let engine = BitEngine::new(&source).expect("engine should load from content");

        // The test graph only has "bits", which has no default generator
        // (see GeneratorKind::default_for_concept) — so nothing should
        // come back practicable from this particular content set.
        assert!(engine.practicable_concepts().is_empty());
    }

    /// "bits" has no default generator (not practicable); "binary" does
    /// and requires "bits" as a prerequisite — enough to exercise both
    /// the game pool (practicable ∩ discovered) and next-topic
    /// suggestion (prerequisite-gated) without a full content directory.
    fn source_with_prerequisite_chain() -> MemorySource {
        MemorySource::new()
            .add(
                "domains",
                "ron",
                r#"Domain(id: "foundations", name: "Foundations", parent: None, order: 0)"#,
            )
            .add(
                "concepts",
                "ron",
                r#"Concept(
                    id: "bits",
                    title: "Bits",
                    domain: "foundations",
                    difficulty: Easy,
                    applicability: [Core],
                    prerequisites: [],
                    related: [],
                    learning_objectives: [],
                    explanation: "bits.md",
                    interactive_demo: None,
                    practice_types: [],
                    build_refs: [],
                    lab_capabilities: [],
                    real_world_uses: [],
                    historical_relevance: None,
                    mastery: (min_practice_correct: 3, min_accuracy: 0.8, decay_days: None),
                )"#,
            )
            .add(
                "concepts",
                "ron",
                r#"Concept(
                    id: "binary",
                    title: "Binary",
                    domain: "foundations",
                    difficulty: Easy,
                    applicability: [Core],
                    prerequisites: ["bits"],
                    related: [],
                    learning_objectives: [],
                    explanation: "binary.md",
                    interactive_demo: None,
                    practice_types: [],
                    build_refs: [],
                    lab_capabilities: [],
                    real_world_uses: [],
                    historical_relevance: None,
                    mastery: (min_practice_correct: 3, min_accuracy: 0.8, decay_days: None),
                )"#,
            )
    }

    #[test]
    fn game_pool_only_includes_discovered_practicable_concepts() {
        let source = source_with_prerequisite_chain();
        let mut engine = BitEngine::new(&source).unwrap();

        // Nothing discovered yet -> empty pool, even though "binary" is practicable.
        assert!(engine.game_pool().is_empty());

        engine.record_discovery("bits"); // discovered but not practicable
        assert!(engine.game_pool().is_empty());

        engine.record_discovery("binary"); // discovered AND practicable
        let pool_ids: Vec<&str> = engine.game_pool().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(pool_ids, vec!["binary"]);
    }

    #[test]
    fn next_learnable_concept_is_gated_by_prerequisites() {
        let source = source_with_prerequisite_chain();
        let mut engine = BitEngine::new(&source).unwrap();

        // "bits" has no prerequisites, so it's learnable immediately;
        // "binary" isn't, since its prerequisite "bits" isn't discovered.
        assert_eq!(engine.next_learnable_concept().map(|c| c.id.as_str()), Some("bits"));

        engine.record_discovery("bits");
        assert_eq!(engine.next_learnable_concept().map(|c| c.id.as_str()), Some("binary"));

        engine.record_discovery("binary");
        assert!(engine.next_learnable_concept().is_none(), "everything reachable is already discovered");
    }

    #[test]
    fn new_topic_suggestion_fires_after_threshold_only_when_something_is_learnable() {
        let source = source_with_prerequisite_chain();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_discovery("bits");
        engine.record_discovery("binary");
        // Both discovered -> nothing left to suggest, no matter the streak.
        for _ in 0..20 {
            engine.record_game_correct_answer();
        }
        assert!(!engine.should_suggest_new_topic());

        // A fresh engine with something still learnable: no prompt before
        // the threshold, a prompt at/after it, and acknowledging resets.
        let mut fresh = BitEngine::new(&source).unwrap();
        for _ in 0..14 {
            fresh.record_game_correct_answer();
        }
        assert!(!fresh.should_suggest_new_topic());
        fresh.record_game_correct_answer();
        assert!(fresh.should_suggest_new_topic());

        fresh.acknowledge_new_topic_suggestion();
        assert!(!fresh.should_suggest_new_topic());
        assert_eq!(fresh.game_total_correct(), 15, "acknowledging resets the streak, not the lifetime total");
    }

    #[test]
    fn game_progress_persists_through_save_and_restore() {
        let source = source_with_prerequisite_chain();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_game_correct_answer();
        engine.record_game_correct_answer();

        let saved = engine.save();
        let mut restored = BitEngine::new(&source).unwrap();
        restored.restore(saved);
        assert_eq!(restored.game_total_correct(), 2);
    }

    /// A single concept whose mastery rule sets `decay_days: Some(5)` —
    /// everything `is_rusty` needs and nothing `source()`'s concepts
    /// (all `decay_days: None`) provide.
    fn source_with_decay() -> MemorySource {
        MemorySource::new()
            .add(
                "domains",
                "ron",
                r#"Domain(id: "foundations", name: "Foundations", parent: None, order: 0)"#,
            )
            .add(
                "concepts",
                "ron",
                r#"Concept(
                    id: "binary",
                    title: "Binary",
                    domain: "foundations",
                    difficulty: Easy,
                    applicability: [Core],
                    prerequisites: [],
                    related: [],
                    learning_objectives: [],
                    explanation: "binary.md",
                    interactive_demo: None,
                    practice_types: [],
                    build_refs: [],
                    lab_capabilities: [],
                    real_world_uses: [],
                    historical_relevance: None,
                    mastery: (min_practice_correct: 3, min_accuracy: 0.8, decay_days: Some(5)),
                )"#,
            )
    }

    #[test]
    fn is_rusty_is_false_before_any_practice() {
        let source = source_with_decay();
        let engine = BitEngine::new(&source).unwrap();
        assert!(!engine.is_rusty("binary", "2026-09-01"), "never practiced yet, nothing to decay");
    }

    #[test]
    fn is_rusty_is_false_within_the_decay_window() {
        let source = source_with_decay();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_attempt("binary", true, "2026-09-01");
        assert!(!engine.is_rusty("binary", "2026-09-04"), "only 3 days since practice, decay_days is 5");
    }

    #[test]
    fn is_rusty_is_true_once_the_decay_window_has_passed() {
        let source = source_with_decay();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_attempt("binary", true, "2026-09-01");
        assert!(engine.is_rusty("binary", "2026-09-06"), "exactly 5 days since practice should count as rusty");
        assert!(engine.is_rusty("binary", "2026-09-10"), "well past the window should stay rusty");
    }

    #[test]
    fn is_rusty_is_false_with_no_decay_days_configured() {
        let source = source(); // "bits" concept has decay_days: None
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_attempt("bits", true, "2026-01-01");
        assert!(!engine.is_rusty("bits", "2027-01-01"), "no decay_days means it never goes stale");
    }

    /// `source()`'s only concept ("bits") has no default generator (see
    /// `practicable_concepts_excludes_concepts_without_a_default_
    /// generator`), so daily-challenge tests need a graph concept whose id
    /// actually resolves one — "binary" does (same as `engine_generates_
    /// and_validates_a_challenge_for_a_practicable_concept` above).
    fn source_with_practicable_concept() -> MemorySource {
        MemorySource::new()
            .add(
                "domains",
                "ron",
                r#"Domain(id: "foundations", name: "Foundations", parent: None, order: 0)"#,
            )
            .add(
                "concepts",
                "ron",
                r#"Concept(
                    id: "binary",
                    title: "Binary",
                    domain: "foundations",
                    difficulty: Easy,
                    applicability: [Core],
                    prerequisites: [],
                    related: [],
                    learning_objectives: [],
                    explanation: "binary.md",
                    interactive_demo: None,
                    practice_types: [],
                    build_refs: [],
                    lab_capabilities: [],
                    real_world_uses: [],
                    historical_relevance: None,
                    mastery: (min_practice_correct: 3, min_accuracy: 0.8, decay_days: None),
                )"#,
            )
    }

    #[test]
    fn daily_challenge_is_deterministic_for_the_same_date() {
        let source = source_with_practicable_concept();
        let engine = BitEngine::new(&source).unwrap();
        let a = engine.daily_challenge("2026-09-01").expect("binary is practicable");
        let b = engine.daily_challenge("2026-09-01").expect("binary is practicable");
        assert_eq!(a.concept_id, b.concept_id);
        assert_eq!(a.difficulty, b.difficulty);
        assert_eq!(a.challenge.seed, b.challenge.seed);
        assert_eq!(a.challenge.prompt, b.challenge.prompt);
        assert_eq!(a.day_number, b.day_number);
    }

    #[test]
    fn daily_challenge_differs_across_different_dates() {
        let source = source_with_practicable_concept();
        let engine = BitEngine::new(&source).unwrap();
        let day1 = engine.daily_challenge("2026-09-01").unwrap();
        let day2 = engine.daily_challenge("2026-09-02").unwrap();
        assert_ne!(day1.day_number, day2.day_number, "different calendar days must get different day numbers");
        assert_ne!(day1.challenge.seed, day2.challenge.seed, "different days must not reuse the same seed");
    }

    #[test]
    fn daily_challenge_result_is_none_until_recorded_then_round_trips() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        assert_eq!(engine.daily_challenge_result("2026-09-01"), None);
        engine.record_daily_challenge_result("2026-09-01", true);
        assert_eq!(engine.daily_challenge_result("2026-09-01"), Some(true));
        assert_eq!(engine.daily_challenge_result("2026-09-02"), None, "recording one day must not leak into another");
    }

    #[test]
    fn daily_challenge_result_also_feeds_the_regular_daily_streak() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        assert_eq!(engine.daily_streak(), 0);
        engine.record_daily_challenge_result("2026-09-01", false);
        assert_eq!(engine.daily_streak(), 1, "solving (or attempting) the daily puzzle alone should keep the streak alive");
    }

    #[test]
    fn daily_challenge_result_round_trips_through_save_and_restore() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_challenge_result("2026-09-01", true);
        let save = engine.save();
        let mut fresh = BitEngine::new(&source).unwrap();
        fresh.restore(save);
        assert_eq!(fresh.daily_challenge_result("2026-09-01"), Some(true));
    }

    #[test]
    fn daily_streak_starts_at_one_on_first_activity() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        assert_eq!(engine.daily_streak(), 0);
        engine.record_daily_activity("2026-09-01");
        assert_eq!(engine.daily_streak(), 1);
        assert_eq!(engine.longest_daily_streak(), 1);
    }

    #[test]
    fn daily_streak_is_idempotent_within_the_same_day() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-01");
        assert_eq!(engine.daily_streak(), 1);
    }

    #[test]
    fn daily_streak_increments_on_consecutive_calendar_days() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-02");
        engine.record_daily_activity("2026-09-03");
        assert_eq!(engine.daily_streak(), 3);
        assert_eq!(engine.longest_daily_streak(), 3);
    }

    #[test]
    fn daily_streak_crosses_a_month_boundary_correctly() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("2026-08-31");
        engine.record_daily_activity("2026-09-01");
        assert_eq!(engine.daily_streak(), 2);
    }

    #[test]
    fn daily_streak_resets_after_a_gap_but_remembers_the_longest() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-02");
        engine.record_daily_activity("2026-09-03");
        assert_eq!(engine.longest_daily_streak(), 3);

        // Skips 09-04: a gap resets the current streak but not the record.
        engine.record_daily_activity("2026-09-06");
        assert_eq!(engine.daily_streak(), 1);
        assert_eq!(engine.longest_daily_streak(), 3);
    }

    #[test]
    fn daily_streak_ignores_a_date_earlier_than_the_last_recorded_one() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("2026-09-05");
        engine.record_daily_activity("2026-09-01"); // clock skew / stale call
        assert_eq!(engine.daily_streak(), 1, "an out-of-order date should be ignored, not corrupt the streak");
    }

    #[test]
    fn daily_streak_ignores_malformed_dates() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("not-a-date");
        assert_eq!(engine.daily_streak(), 0);
    }

    #[test]
    fn activity_by_day_tallies_every_call_even_within_the_same_day() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-02");

        assert_eq!(engine.activity_by_day().get("2026-09-01"), Some(&3));
        assert_eq!(engine.activity_by_day().get("2026-09-02"), Some(&1));
        assert_eq!(engine.activity_by_day().get("2026-09-03"), None);
        // Tallying and the streak counter are independent: 2 consecutive
        // days of activity, regardless of how many events happened.
        assert_eq!(engine.daily_streak(), 2);
    }

    #[test]
    fn activity_by_day_persists_through_save_and_restore() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-01");

        let saved = engine.save();
        let mut restored = BitEngine::new(&source).unwrap();
        restored.restore(saved);
        assert_eq!(restored.activity_by_day().get("2026-09-01"), Some(&2));
    }

    #[test]
    fn daily_streak_persists_through_save_and_restore() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.record_daily_activity("2026-09-01");
        engine.record_daily_activity("2026-09-02");

        let saved = engine.save();
        let mut restored = BitEngine::new(&source).unwrap();
        restored.restore(saved);
        assert_eq!(restored.daily_streak(), 2);
        assert_eq!(restored.longest_daily_streak(), 2);

        // And the restored engine still applies date logic correctly
        // against the restored `last_active_day`.
        restored.record_daily_activity("2026-09-03");
        assert_eq!(restored.daily_streak(), 3);
    }

    #[test]
    fn milestones_are_uncelebrated_until_marked_and_idempotent_after() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        assert!(!engine.has_celebrated("first_mastery"));

        engine.mark_celebrated("first_mastery");
        assert!(engine.has_celebrated("first_mastery"));
        assert!(!engine.has_celebrated("100_correct"), "marking one milestone shouldn't mark another");

        // Marking twice is a no-op, not a duplicate entry.
        engine.mark_celebrated("first_mastery");
        assert!(engine.has_celebrated("first_mastery"));
    }

    #[test]
    fn celebrated_milestones_persist_through_save_and_restore() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.mark_celebrated("first_mastery");

        let saved = engine.save();
        let mut restored = BitEngine::new(&source).unwrap();
        restored.restore(saved);
        assert!(restored.has_celebrated("first_mastery"));
        assert!(!restored.has_celebrated("100_correct"));
    }

    #[test]
    fn discovered_concept_count_only_counts_discovered_concepts() {
        let source = source_with_prerequisite_chain();
        let mut engine = BitEngine::new(&source).unwrap();
        assert_eq!(engine.discovered_concept_count(), 0);

        engine.record_discovery("bits");
        assert_eq!(engine.discovered_concept_count(), 1);

        engine.record_discovery("binary");
        assert_eq!(engine.discovered_concept_count(), 2);

        // Practicing without discovering shouldn't be possible via the
        // public API, but recording an attempt on an already-discovered
        // concept shouldn't double count it either.
        engine.record_attempt("binary", true, "2026-09-01");
        assert_eq!(engine.discovered_concept_count(), 2);
    }

    #[test]
    fn set_note_stores_and_overwrites() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        assert_eq!(engine.note_for("bits"), None);

        engine.set_note("bits", "remember: this is the LSB");
        assert_eq!(engine.note_for("bits"), Some("remember: this is the LSB"));

        engine.set_note("bits", "updated note");
        assert_eq!(engine.note_for("bits"), Some("updated note"));
    }

    #[test]
    fn set_note_with_empty_text_clears_it() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.set_note("bits", "a note");
        assert!(engine.note_for("bits").is_some());

        engine.set_note("bits", "");
        assert_eq!(engine.note_for("bits"), None);
    }

    #[test]
    fn notes_persist_through_save_and_restore() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.set_note("bits", "a persisted note");

        let saved = engine.save();
        let mut restored = BitEngine::new(&source).unwrap();
        restored.restore(saved);
        assert_eq!(restored.note_for("bits"), Some("a persisted note"));
    }

    #[test]
    fn toggle_bookmark_adds_then_removes() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        assert!(!engine.is_bookmarked("bits"));

        engine.toggle_bookmark("bits");
        assert!(engine.is_bookmarked("bits"));
        assert_eq!(engine.bookmarked_concepts().len(), 1);

        engine.toggle_bookmark("bits");
        assert!(!engine.is_bookmarked("bits"));
        assert!(engine.bookmarked_concepts().is_empty());
    }

    #[test]
    fn bookmarked_concepts_skips_dangling_ids() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.toggle_bookmark("bits");
        engine.toggle_bookmark("does_not_exist");
        let ids: Vec<&str> = engine.bookmarked_concepts().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["bits"]);
    }

    #[test]
    fn bookmarks_persist_through_save_and_restore() {
        let source = source();
        let mut engine = BitEngine::new(&source).unwrap();
        engine.toggle_bookmark("bits");

        let saved = engine.save();
        let mut restored = BitEngine::new(&source).unwrap();
        restored.restore(saved);
        assert!(restored.is_bookmarked("bits"));
    }

    #[test]
    fn engine_loads_content_and_tracks_mastery_end_to_end() {
        let source = source();
        let mut engine = BitEngine::new(&source).expect("engine should load from content");

        assert!(engine.graph().concept("bits").is_some());
        assert_eq!(engine.round_stage_for("bits"), bit_progression::RoundStage::Demonstration);

        engine.record_discovery("bits");
        assert_eq!(engine.round_stage_for("bits"), bit_progression::RoundStage::Fiddle);

        let saved = engine.save();
        let mut restored = BitEngine::new(&source).unwrap();
        restored.restore(saved);
        assert_eq!(restored.round_stage_for("bits"), bit_progression::RoundStage::Fiddle);
    }
}
