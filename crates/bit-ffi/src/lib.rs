//! UniFFI bindings exposing `bit-core::BitEngine` to Swift. See
//! /ARCHITECTURE.md §10 and §13 step 6.
//!
//! Two payload strategies, chosen per field per §10: structured UniFFI
//! records/enums for small, stable, hot-path values (build actions and
//! state snapshots — only two simulation kinds exist, so typing them
//! explicitly costs little and is safer than a stringly-typed action
//! format); a single JSON string for `Concept`, whose shape is large and
//! still evolving (`concept_detail_json`) rather than hand-mirroring it as
//! a second, parallel struct definition.
//!
//! `FfiEngine` is a `uniffi::Object`, which Swift always holds through a
//! reference-counted handle — exported methods therefore take `&self`,
//! never `&mut self`. All actual mutable state (the engine, and live
//! build sessions) lives behind one `Mutex`; this app has no hot path
//! where per-field locking would matter.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bit_build::catalog::{BuildFactoryError, LiveBuildSession};
use bit_build::{BuildDefinition, BuildSession, BuildTier};
use bit_content::FilesystemSource;
use bit_core::BitEngine;
use bit_simulation::{array_seq, binary_circuit, logic_gate, pixel_grid, ObjectiveStatus, StepResult};

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("failed to load content from '{path}': {message}")]
    ContentLoad { path: String, message: String },
    #[error("build '{id}' could not be started: {message}")]
    BuildStart { id: String, message: String },
    #[error("no build session with id {id}")]
    SessionNotFound { id: u64 },
    #[error("build session {id} is a {actual} build, not a {expected} build")]
    WrongSessionKind { id: u64, expected: String, actual: String },
    #[error("failed to parse save data: {message}")]
    SaveParse { message: String },
}

// ---------------------------------------------------------------------
// Explore
// ---------------------------------------------------------------------

#[derive(Debug, Clone, uniffi::Enum)]
pub enum FfiDifficulty {
    Easy,
    Core,
    Practical,
    Advanced,
    Specialized,
    Extreme,
    Theoretical,
}

fn map_difficulty(d: bit_knowledge::Difficulty) -> FfiDifficulty {
    use bit_knowledge::Difficulty::*;
    match d {
        Easy => FfiDifficulty::Easy,
        Core => FfiDifficulty::Core,
        Practical => FfiDifficulty::Practical,
        Advanced => FfiDifficulty::Advanced,
        Specialized => FfiDifficulty::Specialized,
        Extreme => FfiDifficulty::Extreme,
        Theoretical => FfiDifficulty::Theoretical,
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiDomain {
    pub id: String,
    pub name: String,
    pub description: String,
}

fn map_domain(d: &bit_knowledge::Domain) -> FfiDomain {
    FfiDomain { id: d.id.clone(), name: d.name.clone(), description: d.description.clone() }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiConceptSummary {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub difficulty: FfiDifficulty,
}

fn concept_summary(c: &bit_knowledge::Concept) -> FfiConceptSummary {
    FfiConceptSummary {
        id: c.id.clone(),
        title: c.title.clone(),
        domain: c.domain.clone(),
        difficulty: map_difficulty(c.difficulty),
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiSearchHit {
    pub concept_id: String,
    pub score: u32,
}

// ---------------------------------------------------------------------
// Progression
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum FfiRoundStage {
    Demonstration,
    Fiddle,
    GuidedPractice,
    TimedChallenge,
}

fn map_round_stage(s: bit_progression::RoundStage) -> FfiRoundStage {
    use bit_progression::RoundStage::*;
    match s {
        Demonstration => FfiRoundStage::Demonstration,
        Fiddle => FfiRoundStage::Fiddle,
        GuidedPractice => FfiRoundStage::GuidedPractice,
        TimedChallenge => FfiRoundStage::TimedChallenge,
    }
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum FfiMasteryStage {
    Discovered,
    Practicing,
    Proficient,
    Mastered,
}

fn map_mastery_stage(s: bit_progression::MasteryStage) -> FfiMasteryStage {
    use bit_progression::MasteryStage::*;
    match s {
        Discovered => FfiMasteryStage::Discovered,
        Practicing => FfiMasteryStage::Practicing,
        Proficient => FfiMasteryStage::Proficient,
        Mastered => FfiMasteryStage::Mastered,
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiMastery {
    pub discovered: bool,
    pub practiced_count: u32,
    pub correct_count: u32,
    pub stage: FfiMasteryStage,
    pub streak: u32,
    pub accuracy: f32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiDayActivity {
    pub day: String,
    pub count: u32,
}

fn map_mastery(m: &bit_progression::MasteryState) -> FfiMastery {
    FfiMastery {
        discovered: m.discovered,
        practiced_count: m.practiced_count,
        correct_count: m.correct_count,
        stage: map_mastery_stage(m.stage),
        streak: m.streak,
        accuracy: m.accuracy(),
    }
}

// ---------------------------------------------------------------------
// Builds
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum FfiBuildTier {
    Beginner,
    Core,
    Advanced,
    Extreme,
}

fn map_build_tier(t: bit_build::BuildTier) -> FfiBuildTier {
    use bit_build::BuildTier::*;
    match t {
        Beginner => FfiBuildTier::Beginner,
        Core => FfiBuildTier::Core,
        Advanced => FfiBuildTier::Advanced,
        Extreme => FfiBuildTier::Extreme,
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiBuildSummary {
    pub id: String,
    pub name: String,
    pub tier: FfiBuildTier,
    pub description: String,
    pub freeform: bool,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum FfiObjectiveStatus {
    InProgress,
    Met,
}

fn map_objective_status(s: ObjectiveStatus) -> FfiObjectiveStatus {
    match s {
        ObjectiveStatus::InProgress => FfiObjectiveStatus::InProgress,
        ObjectiveStatus::Met => FfiObjectiveStatus::Met,
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiStepResult {
    pub accepted: bool,
    pub message: Option<String>,
}

fn map_step_result(r: StepResult) -> FfiStepResult {
    FfiStepResult { accepted: r.accepted, message: r.message }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum FfiCircuitAction {
    Toggle { index: u16 },
    Set { index: u16, value: bool },
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiCircuitState {
    pub switches: Vec<bool>,
    pub value: u32,
    /// The same switches read as two's complement (see
    /// `binary_circuit::State::signed_value`). Always populated — it's
    /// free to compute — but only meaningful to show when the concept
    /// being demonstrated actually calls for a signed reading (e.g. Two's
    /// Complement's demo); Binary's demo just ignores this field.
    pub signed_value: i32,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum FfiPixelAction {
    Toggle { x: u16, y: u16 },
    Set { x: u16, y: u16, value: bool },
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiPixelState {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiGateKind {
    And,
    Or,
    Xor,
    Not,
}

fn map_gate_kind(k: FfiGateKind) -> logic_gate::GateKind {
    match k {
        FfiGateKind::And => logic_gate::GateKind::And,
        FfiGateKind::Or => logic_gate::GateKind::Or,
        FfiGateKind::Xor => logic_gate::GateKind::Xor,
        FfiGateKind::Not => logic_gate::GateKind::Not,
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum FfiGateAction {
    Toggle { index: u16 },
    Set { index: u16, value: bool },
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiGateState {
    pub inputs: Vec<bool>,
    pub output: bool,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum FfiArrayAction {
    Set { index: u16, value: u8 },
    Clear { index: u16 },
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiArrayState {
    pub slots: Vec<u8>,
    pub filled: Vec<bool>,
}

// ---------------------------------------------------------------------
// Challenges
// ---------------------------------------------------------------------

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiChallenge {
    pub prompt: String,
    /// Round-trip this back into `validate_challenge` alongside the same
    /// `(concept_id, difficulty)` — generation is deterministic, so no
    /// challenge state is kept on the Rust side between the two calls.
    pub seed: u64,
}

/// Today's Puzzle of the Day — same concept, same seed, on every install.
/// `concept_id` and `difficulty` round-trip into `validate_challenge`
/// alongside `challenge.seed`, exactly like a regular Play challenge.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiDailyChallenge {
    pub day_number: u32,
    pub concept_id: String,
    pub concept_title: String,
    pub difficulty: u8,
    pub challenge: FfiChallenge,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiValidationResult {
    pub correct: bool,
    pub explanation: Option<String>,
}

// ---------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------

struct EngineState {
    engine: BitEngine,
    sessions: HashMap<u64, LiveBuildSession>,
    next_session_id: u64,
}

#[derive(uniffi::Object)]
pub struct FfiEngine {
    state: Mutex<EngineState>,
}

#[uniffi::export]
impl FfiEngine {
    /// `content_dir` is an absolute path to the app's bundled `/content`
    /// directory. On iOS this will be a path inside the app bundle's
    /// resources — reading it through `FilesystemSource` either way is
    /// exactly the point of that trait (see /ARCHITECTURE.md §3).
    #[uniffi::constructor]
    pub fn new(content_dir: String) -> Result<Arc<Self>, FfiError> {
        let source = FilesystemSource::new(&content_dir);
        let engine = BitEngine::new(&source)
            .map_err(|e| FfiError::ContentLoad { path: content_dir.clone(), message: format!("{e:?}") })?;
        Ok(Arc::new(Self { state: Mutex::new(EngineState { engine, sessions: HashMap::new(), next_session_id: 1 }) }))
    }

    // -- Explore ---------------------------------------------------

    pub fn root_domains(&self) -> Vec<FfiDomain> {
        let state = self.state.lock().unwrap();
        state.engine.graph().root_domains().into_iter().map(map_domain).collect()
    }

    pub fn domain_children(&self, domain_id: String) -> Vec<FfiDomain> {
        let state = self.state.lock().unwrap();
        state.engine.graph().children(&domain_id).into_iter().map(map_domain).collect()
    }

    pub fn concepts_in_domain(&self, domain_id: String) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.graph().concepts_in(&domain_id).into_iter().map(concept_summary).collect()
    }

    /// Full concept detail as JSON — see the module doc comment for why
    /// this one payload is JSON rather than a mirrored `uniffi::Record`.
    pub fn concept_detail_json(&self, concept_id: String) -> Option<String> {
        let state = self.state.lock().unwrap();
        state.engine.graph().concept(&concept_id).map(|c| serde_json::to_string(c).expect("Concept always serializes"))
    }

    pub fn search(&self, query: String) -> Vec<FfiSearchHit> {
        let state = self.state.lock().unwrap();
        state.engine.graph().search(&query).into_iter().map(|h| FfiSearchHit { concept_id: h.concept_id, score: h.score }).collect()
    }

    pub fn prerequisites(&self, concept_id: String) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.graph().prerequisites(&concept_id).into_iter().map(concept_summary).collect()
    }

    pub fn related(&self, concept_id: String) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.graph().related(&concept_id).into_iter().map(concept_summary).collect()
    }

    pub fn used_by(&self, concept_id: String) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.graph().used_by(&concept_id).into_iter().map(concept_summary).collect()
    }

    /// Every concept in the graph — Profile uses this to summarize
    /// discovery/mastery across all of Explore in one pass.
    pub fn all_concepts(&self) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.graph().all_concepts().into_iter().map(concept_summary).collect()
    }

    /// Every concept Play can actually offer a round for — see
    /// `BitEngine::practicable_concepts`. Play picks from this list
    /// rather than being hardcoded to one concept.
    pub fn practicable_concepts(&self) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.practicable_concepts().into_iter().map(concept_summary).collect()
    }

    // -- Play's endless game mode ---------------------------------------

    /// The puzzle pool right now: practicable concepts the player has
    /// actually discovered. Empty until at least one discovered concept
    /// has a working generator.
    pub fn game_pool(&self) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.game_pool().into_iter().map(concept_summary).collect()
    }

    /// A concept that's genuinely ready to learn next (every prerequisite
    /// already discovered), or `None` if nothing new is unlocked yet.
    pub fn next_learnable_concept(&self) -> Option<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.next_learnable_concept().map(concept_summary)
    }

    /// The plain number shown at the top of the game screen — lifetime
    /// correct answers in game mode.
    pub fn game_total_correct(&self) -> u32 {
        let state = self.state.lock().unwrap();
        state.engine.game_total_correct()
    }

    /// Call after grading an answer in game mode, only when it was
    /// correct — persists automatically the same way discovery/attempt
    /// recording does (see `Engine.swift`).
    pub fn record_game_correct_answer(&self) {
        let mut state = self.state.lock().unwrap();
        state.engine.record_game_correct_answer();
    }

    pub fn should_suggest_new_topic(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.engine.should_suggest_new_topic()
    }

    pub fn acknowledge_new_topic_suggestion(&self) {
        let mut state = self.state.lock().unwrap();
        state.engine.acknowledge_new_topic_suggestion();
    }

    /// `today` is the caller's local calendar day as "YYYY-MM-DD" —
    /// Swift is the only side of this boundary that ever asks the system
    /// clock what day it is, keeping this crate's date logic pure and
    /// testable (see `BitEngine::record_daily_activity`).
    pub fn record_daily_activity(&self, today: String) {
        let mut state = self.state.lock().unwrap();
        state.engine.record_daily_activity(&today);
    }

    pub fn daily_streak(&self) -> u32 {
        let state = self.state.lock().unwrap();
        state.engine.daily_streak()
    }

    pub fn longest_daily_streak(&self) -> u32 {
        let state = self.state.lock().unwrap();
        state.engine.longest_daily_streak()
    }

    /// `None` only if the bundled content has zero practicable concepts
    /// or `today` fails to parse — neither should happen in practice.
    pub fn daily_challenge(&self, today: String) -> Option<FfiDailyChallenge> {
        let state = self.state.lock().unwrap();
        let d = state.engine.daily_challenge(&today)?;
        let concept_title = state.engine.graph().concept(&d.concept_id).map(|c| c.title.clone()).unwrap_or_default();
        Some(FfiDailyChallenge {
            day_number: d.day_number,
            concept_id: d.concept_id,
            concept_title,
            difficulty: d.difficulty,
            challenge: FfiChallenge { prompt: d.challenge.prompt, seed: d.challenge.seed },
        })
    }

    /// Also feeds the regular daily streak — see `BitEngine::record_
    /// daily_challenge_result`. Call once, right after validating the
    /// player's answer to `daily_challenge`.
    pub fn record_daily_challenge_result(&self, today: String, correct: bool) {
        let mut state = self.state.lock().unwrap();
        state.engine.record_daily_challenge_result(&today, correct);
    }

    /// `None` means not yet attempted today.
    pub fn daily_challenge_result(&self, today: String) -> Option<bool> {
        let state = self.state.lock().unwrap();
        state.engine.daily_challenge_result(&today)
    }

    /// Every day with recorded activity, sorted ascending by date —
    /// Profile's heatmap renders these directly, oldest first.
    pub fn activity_history(&self) -> Vec<FfiDayActivity> {
        let state = self.state.lock().unwrap();
        let mut days: Vec<FfiDayActivity> =
            state.engine.activity_by_day().iter().map(|(day, &count)| FfiDayActivity { day: day.clone(), count }).collect();
        days.sort_by(|a, b| a.day.cmp(&b.day));
        days
    }

    /// `id` is a caller-defined milestone id (e.g. "first_mastery",
    /// "100_correct", "first_build") — see `BitEngine::has_celebrated`.
    pub fn has_celebrated(&self, id: String) -> bool {
        let state = self.state.lock().unwrap();
        state.engine.has_celebrated(&id)
    }

    pub fn mark_celebrated(&self, id: String) {
        let mut state = self.state.lock().unwrap();
        state.engine.mark_celebrated(&id);
    }

    // -- Progression -------------------------------------------------

    pub fn round_stage(&self, concept_id: String) -> FfiRoundStage {
        let state = self.state.lock().unwrap();
        map_round_stage(state.engine.round_stage_for(&concept_id))
    }

    pub fn mastery(&self, concept_id: String) -> FfiMastery {
        let state = self.state.lock().unwrap();
        map_mastery(&state.engine.mastery_for(&concept_id))
    }

    pub fn discovered_concept_count(&self) -> u32 {
        let state = self.state.lock().unwrap();
        state.engine.discovered_concept_count() as u32
    }

    pub fn record_discovery(&self, concept_id: String) {
        let mut state = self.state.lock().unwrap();
        state.engine.record_discovery(&concept_id);
    }

    pub fn toggle_bookmark(&self, concept_id: String) {
        let mut state = self.state.lock().unwrap();
        state.engine.toggle_bookmark(&concept_id);
    }

    pub fn is_bookmarked(&self, concept_id: String) -> bool {
        let state = self.state.lock().unwrap();
        state.engine.is_bookmarked(&concept_id)
    }

    pub fn bookmarked_concepts(&self) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        state.engine.bookmarked_concepts().into_iter().map(concept_summary).collect()
    }

    /// An empty `text` clears the note entirely — see
    /// `BitEngine::set_note`'s doc comment.
    pub fn set_note(&self, concept_id: String, text: String) {
        let mut state = self.state.lock().unwrap();
        state.engine.set_note(&concept_id, &text);
    }

    pub fn note_for(&self, concept_id: String) -> Option<String> {
        let state = self.state.lock().unwrap();
        state.engine.note_for(&concept_id).map(str::to_string)
    }

    pub fn record_attempt(&self, concept_id: String, correct: bool, today: String) {
        let mut state = self.state.lock().unwrap();
        state.engine.record_attempt(&concept_id, correct, &today);
    }

    /// Whether `concept_id` is due for review — see `BitEngine::is_rusty`.
    /// Play reads this per pool candidate to weight puzzle selection
    /// toward what's going stale instead of a flat random pick.
    pub fn is_rusty(&self, concept_id: String, today: String) -> bool {
        let state = self.state.lock().unwrap();
        state.engine.is_rusty(&concept_id, &today)
    }

    // -- Persistence ---------------------------------------------------

    pub fn save_json(&self) -> String {
        let state = self.state.lock().unwrap();
        state.engine.save().to_json()
    }

    pub fn restore_json(&self, json: String) -> Result<(), FfiError> {
        let save = bit_persistence::SaveState::from_json(&json).map_err(|message| FfiError::SaveParse { message })?;
        let mut state = self.state.lock().unwrap();
        state.engine.restore(save);
        Ok(())
    }

    // -- Challenges ---------------------------------------------------

    pub fn generate_challenge(&self, concept_id: String, difficulty: u8, seed: u64) -> Option<FfiChallenge> {
        let state = self.state.lock().unwrap();
        state.engine.generate_challenge(&concept_id, difficulty, seed).map(|c| FfiChallenge { prompt: c.prompt, seed: c.seed })
    }

    pub fn validate_challenge(&self, concept_id: String, difficulty: u8, seed: u64, answer: String) -> Option<FfiValidationResult> {
        let state = self.state.lock().unwrap();
        state
            .engine
            .validate_challenge(&concept_id, difficulty, seed, &answer)
            .map(|r| FfiValidationResult { correct: r.correct, explanation: r.explanation })
    }

    // -- Builds ---------------------------------------------------------

    pub fn list_builds(&self) -> Vec<FfiBuildSummary> {
        let state = self.state.lock().unwrap();
        state
            .engine
            .builds()
            .iter()
            .map(|b| FfiBuildSummary {
                id: b.id.clone(),
                name: b.name.clone(),
                tier: map_build_tier(b.tier),
                description: b.description.clone(),
                freeform: b.freeform,
            })
            .collect()
    }

    /// The concepts a build's own `concept_ids` names, resolved to full
    /// summaries — Build's session screen uses this to show "what this
    /// project actually practices" and link straight back to Explore.
    /// A dangling id (content typo, or a concept removed after the
    /// build was authored) is silently skipped rather than surfaced as
    /// an error, same policy as `bookmarked_concepts`.
    pub fn build_related_concepts(&self, build_id: String) -> Vec<FfiConceptSummary> {
        let state = self.state.lock().unwrap();
        let Some(definition) = state.engine.builds().iter().find(|b| b.id == build_id) else {
            return Vec::new();
        };
        definition.concept_ids.iter().filter_map(|id| state.engine.graph().concept(id)).map(concept_summary).collect()
    }

    pub fn start_build(&self, build_id: String) -> Result<u64, FfiError> {
        let mut state = self.state.lock().unwrap();
        let session = state
            .engine
            .start_build(&build_id)
            .map_err(|e| build_start_error(&build_id, e))?;
        let id = state.next_session_id;
        state.next_session_id += 1;
        state.sessions.insert(id, session);
        Ok(id)
    }

    pub fn end_build(&self, session_id: u64) {
        let mut state = self.state.lock().unwrap();
        state.sessions.remove(&session_id);
    }

    pub fn objective_status(&self, session_id: u64) -> Result<FfiObjectiveStatus, FfiError> {
        let state = self.state.lock().unwrap();
        let session = state.sessions.get(&session_id).ok_or(FfiError::SessionNotFound { id: session_id })?;
        Ok(map_objective_status(match session {
            LiveBuildSession::BinaryCircuit(s) => s.objective_status(),
            LiveBuildSession::PixelGrid(s) => s.objective_status(),
            LiveBuildSession::LogicGate(s) => s.objective_status(),
            LiveBuildSession::Array(s) => s.objective_status(),
        }))
    }

    pub fn circuit_state(&self, session_id: u64) -> Result<FfiCircuitState, FfiError> {
        let state = self.state.lock().unwrap();
        let session = circuit_session(&state.sessions, session_id)?;
        let s = session.state();
        Ok(FfiCircuitState { switches: s.switches.clone(), value: s.value(), signed_value: s.signed_value() })
    }

    pub fn apply_circuit_action(&self, session_id: u64, action: FfiCircuitAction) -> Result<FfiStepResult, FfiError> {
        let mut state = self.state.lock().unwrap();
        let session = circuit_session_mut(&mut state.sessions, session_id)?;
        let action = match action {
            FfiCircuitAction::Toggle { index } => binary_circuit::Action::Toggle(index as usize),
            FfiCircuitAction::Set { index, value } => binary_circuit::Action::Set(index as usize, value),
        };
        Ok(map_step_result(session.apply_action(action)))
    }

    pub fn pixel_state(&self, session_id: u64) -> Result<FfiPixelState, FfiError> {
        let state = self.state.lock().unwrap();
        let session = pixel_session(&state.sessions, session_id)?;
        let s = session.state();
        Ok(FfiPixelState { width: s.width as u16, height: s.height as u16, pixels: s.pixels.clone() })
    }

    pub fn apply_pixel_action(&self, session_id: u64, action: FfiPixelAction) -> Result<FfiStepResult, FfiError> {
        let mut state = self.state.lock().unwrap();
        let session = pixel_session_mut(&mut state.sessions, session_id)?;
        let action = match action {
            FfiPixelAction::Toggle { x, y } => pixel_grid::Action::Toggle(x as usize, y as usize),
            FfiPixelAction::Set { x, y, value } => pixel_grid::Action::Set(x as usize, y as usize, value),
        };
        Ok(map_step_result(session.apply_action(action)))
    }

    pub fn array_state(&self, session_id: u64) -> Result<FfiArrayState, FfiError> {
        let state = self.state.lock().unwrap();
        let session = array_session(&state.sessions, session_id)?;
        let s = session.state();
        Ok(FfiArrayState { slots: s.slots.clone(), filled: s.filled.clone() })
    }

    pub fn apply_array_action(&self, session_id: u64, action: FfiArrayAction) -> Result<FfiStepResult, FfiError> {
        let mut state = self.state.lock().unwrap();
        let session = array_session_mut(&mut state.sessions, session_id)?;
        let action = match action {
            FfiArrayAction::Set { index, value } => array_seq::Action::Set(index as usize, value),
            FfiArrayAction::Clear { index } => array_seq::Action::Clear(index as usize),
        };
        Ok(map_step_result(session.apply_action(action)))
    }

    // -- Ad hoc demo sessions -------------------------------------------
    //
    // Every concept's `interactive_demo` (see bit-knowledge) needs a live
    // simulation to demonstrate against, but not every demo corresponds
    // to a content-authored Build — "here are 8 switches" for the Binary
    // concept page isn't Binary Counter's specific target-value objective,
    // it's the bare simulation. These two constructors bypass
    // `bit_build::catalog` (which only knows named, content-driven builds)
    // and construct a session directly, always freeform, always
    // succeeding — there's no id lookup that can fail.

    pub fn start_circuit_demo(&self, bit_width: u8) -> u64 {
        let mut state = self.state.lock().unwrap();
        let definition = demo_definition("circuit_demo", "Circuit Demo");
        let session = BuildSession::new(definition, binary_circuit::BinaryCircuitSim { bit_width }, vec![]);
        insert_session(&mut state, LiveBuildSession::BinaryCircuit(session))
    }

    pub fn start_gate_demo(&self, gate: FfiGateKind, input_count: u8) -> u64 {
        let mut state = self.state.lock().unwrap();
        let definition = demo_definition("gate_demo", "Gate Demo");
        let sim = logic_gate::LogicGateSim { kind: map_gate_kind(gate), input_count: input_count as usize };
        let session = BuildSession::new(definition, sim, vec![]);
        insert_session(&mut state, LiveBuildSession::LogicGate(session))
    }

    pub fn start_array_demo(&self, capacity: u8) -> u64 {
        let mut state = self.state.lock().unwrap();
        let definition = demo_definition("array_demo", "Array Demo");
        let session = BuildSession::new(definition, array_seq::ArraySim { capacity: capacity as usize }, vec![]);
        insert_session(&mut state, LiveBuildSession::Array(session))
    }

    pub fn gate_state(&self, session_id: u64) -> Result<FfiGateState, FfiError> {
        let state = self.state.lock().unwrap();
        let session = gate_session(&state.sessions, session_id)?;
        let kind = session.simulation().kind;
        let s = session.state();
        Ok(FfiGateState { inputs: s.inputs.clone(), output: s.output(kind) })
    }

    pub fn apply_gate_action(&self, session_id: u64, action: FfiGateAction) -> Result<FfiStepResult, FfiError> {
        let mut state = self.state.lock().unwrap();
        let session = gate_session_mut(&mut state.sessions, session_id)?;
        let action = match action {
            FfiGateAction::Toggle { index } => logic_gate::Action::Toggle(index as usize),
            FfiGateAction::Set { index, value } => logic_gate::Action::SetInput(index as usize, value),
        };
        Ok(map_step_result(session.apply_action(action)))
    }
}

fn demo_definition(id: &str, name: &str) -> BuildDefinition {
    BuildDefinition {
        id: id.to_string(),
        name: name.to_string(),
        tier: BuildTier::Beginner,
        description: String::new(),
        concept_ids: vec![],
        params: HashMap::new(),
        freeform: true,
    }
}

fn insert_session(state: &mut EngineState, session: LiveBuildSession) -> u64 {
    let id = state.next_session_id;
    state.next_session_id += 1;
    state.sessions.insert(id, session);
    id
}

fn build_start_error(build_id: &str, e: BuildFactoryError) -> FfiError {
    FfiError::BuildStart { id: build_id.to_string(), message: format!("{e:?}") }
}

fn session_kind_name(session: &LiveBuildSession) -> &'static str {
    match session {
        LiveBuildSession::BinaryCircuit(_) => "binary circuit",
        LiveBuildSession::PixelGrid(_) => "pixel grid",
        LiveBuildSession::LogicGate(_) => "logic gate",
        LiveBuildSession::Array(_) => "array",
    }
}

fn circuit_session(
    sessions: &HashMap<u64, LiveBuildSession>,
    id: u64,
) -> Result<&bit_build::BuildSession<binary_circuit::BinaryCircuitSim>, FfiError> {
    match sessions.get(&id).ok_or(FfiError::SessionNotFound { id })? {
        LiveBuildSession::BinaryCircuit(s) => Ok(s),
        other => Err(FfiError::WrongSessionKind { id, expected: "binary circuit".into(), actual: session_kind_name(other).into() }),
    }
}

fn circuit_session_mut(
    sessions: &mut HashMap<u64, LiveBuildSession>,
    id: u64,
) -> Result<&mut bit_build::BuildSession<binary_circuit::BinaryCircuitSim>, FfiError> {
    match sessions.get_mut(&id).ok_or(FfiError::SessionNotFound { id })? {
        LiveBuildSession::BinaryCircuit(s) => Ok(s),
        other => Err(FfiError::WrongSessionKind { id, expected: "binary circuit".into(), actual: session_kind_name(other).into() }),
    }
}

fn pixel_session(
    sessions: &HashMap<u64, LiveBuildSession>,
    id: u64,
) -> Result<&bit_build::BuildSession<pixel_grid::PixelGridSim>, FfiError> {
    match sessions.get(&id).ok_or(FfiError::SessionNotFound { id })? {
        LiveBuildSession::PixelGrid(s) => Ok(s),
        other => Err(FfiError::WrongSessionKind { id, expected: "pixel grid".into(), actual: session_kind_name(other).into() }),
    }
}

fn pixel_session_mut(
    sessions: &mut HashMap<u64, LiveBuildSession>,
    id: u64,
) -> Result<&mut bit_build::BuildSession<pixel_grid::PixelGridSim>, FfiError> {
    match sessions.get_mut(&id).ok_or(FfiError::SessionNotFound { id })? {
        LiveBuildSession::PixelGrid(s) => Ok(s),
        other => Err(FfiError::WrongSessionKind { id, expected: "pixel grid".into(), actual: session_kind_name(other).into() }),
    }
}

fn array_session(
    sessions: &HashMap<u64, LiveBuildSession>,
    id: u64,
) -> Result<&bit_build::BuildSession<array_seq::ArraySim>, FfiError> {
    match sessions.get(&id).ok_or(FfiError::SessionNotFound { id })? {
        LiveBuildSession::Array(s) => Ok(s),
        other => Err(FfiError::WrongSessionKind { id, expected: "array".into(), actual: session_kind_name(other).into() }),
    }
}

fn array_session_mut(
    sessions: &mut HashMap<u64, LiveBuildSession>,
    id: u64,
) -> Result<&mut bit_build::BuildSession<array_seq::ArraySim>, FfiError> {
    match sessions.get_mut(&id).ok_or(FfiError::SessionNotFound { id })? {
        LiveBuildSession::Array(s) => Ok(s),
        other => Err(FfiError::WrongSessionKind { id, expected: "array".into(), actual: session_kind_name(other).into() }),
    }
}

fn gate_session(
    sessions: &HashMap<u64, LiveBuildSession>,
    id: u64,
) -> Result<&bit_build::BuildSession<logic_gate::LogicGateSim>, FfiError> {
    match sessions.get(&id).ok_or(FfiError::SessionNotFound { id })? {
        LiveBuildSession::LogicGate(s) => Ok(s),
        other => Err(FfiError::WrongSessionKind { id, expected: "logic gate".into(), actual: session_kind_name(other).into() }),
    }
}

fn gate_session_mut(
    sessions: &mut HashMap<u64, LiveBuildSession>,
    id: u64,
) -> Result<&mut bit_build::BuildSession<logic_gate::LogicGateSim>, FfiError> {
    match sessions.get_mut(&id).ok_or(FfiError::SessionNotFound { id })? {
        LiveBuildSession::LogicGate(s) => Ok(s),
        other => Err(FfiError::WrongSessionKind { id, expected: "logic gate".into(), actual: session_kind_name(other).into() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises `FfiEngine` end-to-end against the real `/content`
    /// directory — the same object graph Swift will hold, driven through
    /// exactly the methods `#[uniffi::export]` generates bindings for.
    /// Catches anything a MemorySource-based unit test upstream can't:
    /// wrong content path, a method panicking across the FFI-shaped
    /// boundary, or a mismatch between what's on disk and what the
    /// FFI surface assumes exists.
    fn new_engine() -> Arc<FfiEngine> {
        let content_dir = format!("{}/../../content", env!("CARGO_MANIFEST_DIR"));
        FfiEngine::new(content_dir).expect("FfiEngine should load the real content directory")
    }

    #[test]
    fn explore_walks_the_real_domain_tree_and_finds_binary_by_search() {
        let engine = new_engine();
        let top = engine.domain_children("".to_string());
        assert!(top.is_empty()); // "" has no parent domain; real roots have no parent id "".

        let foundations_children = engine.domain_children("foundations".to_string());
        assert!(foundations_children.iter().any(|d| d.id == "foundations.bits_binary"));

        let hits = engine.search("binary".to_string());
        assert!(hits.iter().any(|h| h.concept_id == "binary"));

        let detail = engine.concept_detail_json("binary".to_string()).expect("binary concept should exist");
        assert!(detail.contains("\"id\":\"binary\""));
    }

    #[test]
    fn progression_round_trips_through_save_and_restore() {
        let engine = new_engine();
        assert!(matches!(engine.round_stage("binary".to_string()), FfiRoundStage::Demonstration));

        engine.record_discovery("binary".to_string());
        assert!(matches!(engine.round_stage("binary".to_string()), FfiRoundStage::Fiddle));

        let json = engine.save_json();

        let fresh = new_engine();
        fresh.restore_json(json).expect("restore should succeed");
        assert!(matches!(fresh.round_stage("binary".to_string()), FfiRoundStage::Fiddle));
    }

    #[test]
    fn challenge_generation_and_validation_round_trip() {
        let engine = new_engine();
        let challenge = engine
            .generate_challenge("binary".to_string(), 4, 42)
            .expect("binary should be practicable");
        assert!(!challenge.prompt.is_empty());

        // We don't get the expected answer back over FFI (see FfiChallenge's
        // doc comment) — but the same seed always re-derives the same
        // challenge, so re-requesting it is how a real client would get
        // the prompt back to display without ever seeing the answer.
        let same_challenge = engine.generate_challenge("binary".to_string(), 4, 42).unwrap();
        assert_eq!(challenge.prompt, same_challenge.prompt);

        assert!(engine.generate_challenge("bits".to_string(), 4, 1).is_none());
    }

    #[test]
    fn build_related_concepts_resolves_a_real_builds_concept_ids() {
        let engine = new_engine();
        // /content/builds/binary_counter.ron names concept_ids: ["binary"].
        let related = engine.build_related_concepts("binary_counter".to_string());
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].id, "binary");

        assert!(engine.build_related_concepts("does_not_exist".to_string()).is_empty());
    }

    #[test]
    fn builds_list_matches_content_and_binary_counter_is_playable_end_to_end() {
        let engine = new_engine();
        let builds = engine.list_builds();
        assert_eq!(builds.len(), 7);
        assert!(builds.iter().any(|b| b.id == "binary_counter"));
        assert!(builds.iter().any(|b| b.id == "array_builder"));

        let session_id = engine.start_build("binary_counter".to_string()).expect("should start");
        assert!(matches!(engine.objective_status(session_id).unwrap(), FfiObjectiveStatus::InProgress));

        // target_value is 10 = 0b00001010 on an 8-bit register (see
        // /content/builds/binary_counter.ron): switches[3] is the 16s
        // place... rather than hardcode indices twice, just drive it via
        // the actual bit positions for 8 (index 4) and 2 (index 6).
        engine.apply_circuit_action(session_id, FfiCircuitAction::Toggle { index: 4 }).unwrap();
        engine.apply_circuit_action(session_id, FfiCircuitAction::Toggle { index: 6 }).unwrap();
        let state = engine.circuit_state(session_id).unwrap();
        assert_eq!(state.value, 10);
        assert!(matches!(engine.objective_status(session_id).unwrap(), FfiObjectiveStatus::Met));

        // Wrong-kind access is rejected rather than panicking.
        assert!(engine.pixel_state(session_id).is_err());

        engine.end_build(session_id);
        assert!(engine.objective_status(session_id).is_err());
    }

    #[test]
    fn unknown_build_and_unknown_session_report_errors_not_panics() {
        let engine = new_engine();
        assert!(engine.start_build("does_not_exist".to_string()).is_err());
        assert!(engine.objective_status(9999).is_err());
    }

    #[test]
    fn daily_streak_round_trips_through_ffi_and_save_restore() {
        let engine = new_engine();
        assert_eq!(engine.daily_streak(), 0);

        engine.record_daily_activity("2026-09-01".to_string());
        engine.record_daily_activity("2026-09-02".to_string());
        assert_eq!(engine.daily_streak(), 2);
        assert_eq!(engine.longest_daily_streak(), 2);

        let json = engine.save_json();
        let fresh = new_engine();
        fresh.restore_json(json).unwrap();
        assert_eq!(fresh.daily_streak(), 2);
    }

    #[test]
    fn daily_challenge_is_answerable_through_validate_challenge_and_result_round_trips() {
        let engine = new_engine();
        assert_eq!(engine.daily_challenge_result("2026-09-01".to_string()), None);

        let daily = engine.daily_challenge("2026-09-01".to_string()).expect("real content always has a practicable concept");
        assert!(!daily.concept_title.is_empty(), "concept_title should resolve to a real concept, not an unknown id");

        // The challenge round-trips into the exact same validate_challenge
        // path a regular Play question uses — no special-cased validation.
        let concept = engine.concept_detail_json(daily.concept_id.clone()).expect("daily concept should exist in the graph");
        assert!(!concept.is_empty());

        let same_again = engine.daily_challenge("2026-09-01".to_string()).unwrap();
        assert_eq!(daily.challenge.prompt, same_again.challenge.prompt, "same date must produce the same puzzle for everyone");
        assert_eq!(daily.concept_id, same_again.concept_id);

        engine.record_daily_challenge_result("2026-09-01".to_string(), true);
        assert_eq!(engine.daily_challenge_result("2026-09-01".to_string()), Some(true));
        assert_eq!(engine.daily_streak(), 1, "the daily puzzle should also count toward the regular streak");

        let json = engine.save_json();
        let fresh = new_engine();
        fresh.restore_json(json).unwrap();
        assert_eq!(fresh.daily_challenge_result("2026-09-01".to_string()), Some(true));
    }

    #[test]
    fn activity_history_is_sorted_ascending_and_persists() {
        let engine = new_engine();
        engine.record_daily_activity("2026-09-03".to_string());
        engine.record_daily_activity("2026-09-01".to_string());
        engine.record_daily_activity("2026-09-01".to_string());

        let history = engine.activity_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].day, "2026-09-01");
        assert_eq!(history[0].count, 2);
        assert_eq!(history[1].day, "2026-09-03");
        assert_eq!(history[1].count, 1);

        let json = engine.save_json();
        let fresh = new_engine();
        fresh.restore_json(json).unwrap();
        assert_eq!(fresh.activity_history().len(), 2);
    }

    #[test]
    fn milestones_round_trip_through_ffi_and_save_restore() {
        let engine = new_engine();
        assert!(!engine.has_celebrated("first_mastery".to_string()));

        engine.mark_celebrated("first_mastery".to_string());
        assert!(engine.has_celebrated("first_mastery".to_string()));

        let json = engine.save_json();
        let fresh = new_engine();
        fresh.restore_json(json).unwrap();
        assert!(fresh.has_celebrated("first_mastery".to_string()));
        assert!(!fresh.has_celebrated("100_correct".to_string()));
    }

    #[test]
    fn notes_round_trip_through_ffi_and_save_restore() {
        let engine = new_engine();
        assert_eq!(engine.note_for("binary".to_string()), None);

        engine.set_note("binary".to_string(), "my own explanation".to_string());
        assert_eq!(engine.note_for("binary".to_string()), Some("my own explanation".to_string()));

        let json = engine.save_json();
        let fresh = new_engine();
        fresh.restore_json(json).unwrap();
        assert_eq!(fresh.note_for("binary".to_string()), Some("my own explanation".to_string()));

        engine.set_note("binary".to_string(), "".to_string());
        assert_eq!(engine.note_for("binary".to_string()), None);
    }

    #[test]
    fn bookmarks_round_trip_through_ffi_and_save_restore() {
        let engine = new_engine();
        assert!(!engine.is_bookmarked("binary".to_string()));

        engine.toggle_bookmark("binary".to_string());
        assert!(engine.is_bookmarked("binary".to_string()));
        assert_eq!(engine.bookmarked_concepts().len(), 1);
        assert_eq!(engine.bookmarked_concepts()[0].id, "binary");

        let json = engine.save_json();
        let fresh = new_engine();
        fresh.restore_json(json).unwrap();
        assert!(fresh.is_bookmarked("binary".to_string()));
    }

    #[test]
    fn discovered_concept_count_tracks_discoveries_over_ffi() {
        let engine = new_engine();
        assert_eq!(engine.discovered_concept_count(), 0);
        engine.record_discovery("bits".to_string());
        engine.record_discovery("binary".to_string());
        assert_eq!(engine.discovered_concept_count(), 2);
    }

    #[test]
    fn game_pool_tracks_discovery_and_next_learnable_concept_is_gated() {
        let engine = new_engine();

        assert!(engine.game_pool().is_empty());
        // "bits" has no prerequisites, so it's the very first thing ready
        // to learn on a totally fresh install.
        assert_eq!(engine.next_learnable_concept().map(|c| c.id), Some("bits".to_string()));

        engine.record_discovery("binary".to_string());
        let pool_ids: Vec<String> = engine.game_pool().into_iter().map(|c| c.id).collect();
        assert_eq!(pool_ids, vec!["binary".to_string()]);
    }

    #[test]
    fn game_progress_counts_and_suggests_a_new_topic_after_threshold() {
        let engine = new_engine();
        assert_eq!(engine.game_total_correct(), 0);

        for _ in 0..15 {
            engine.record_game_correct_answer();
        }
        assert_eq!(engine.game_total_correct(), 15);
        assert!(engine.should_suggest_new_topic(), "15 correct with 'bits' still learnable should trigger a suggestion");

        engine.acknowledge_new_topic_suggestion();
        assert!(!engine.should_suggest_new_topic());
        assert_eq!(engine.game_total_correct(), 15, "acknowledging resets the streak, not the lifetime total");
    }

    #[test]
    fn practicable_concepts_includes_binary_and_gate_concepts() {
        let engine = new_engine();
        let ids: Vec<String> = engine.practicable_concepts().into_iter().map(|c| c.id).collect();
        assert!(ids.contains(&"binary".to_string()));
        assert!(ids.contains(&"xor".to_string()));
        // "bits" has no default generator (see GeneratorKind::default_for_concept).
        assert!(!ids.contains(&"bits".to_string()));
    }

    #[test]
    fn circuit_demo_reports_both_unsigned_and_signed_readings() {
        let engine = new_engine();
        let session_id = engine.start_circuit_demo(8);

        // All-ones: 255 unsigned, -1 in two's complement — proves
        // signed_value isn't just a copy of value.
        for index in 0..8 {
            engine.apply_circuit_action(session_id, FfiCircuitAction::Toggle { index }).unwrap();
        }
        let state = engine.circuit_state(session_id).unwrap();
        assert_eq!(state.value, 255);
        assert_eq!(state.signed_value, -1);

        // Demo sessions are always freeform (no target to hit).
        assert!(matches!(engine.objective_status(session_id).unwrap(), FfiObjectiveStatus::Met));
    }

    #[test]
    fn gate_demo_and_toggles_compute_a_real_and_output() {
        let engine = new_engine();
        let session_id = engine.start_gate_demo(FfiGateKind::And, 2);

        let initial = engine.gate_state(session_id).unwrap();
        assert_eq!(initial.inputs, vec![false, false]);
        assert!(!initial.output);

        engine.apply_gate_action(session_id, FfiGateAction::Toggle { index: 0 }).unwrap();
        engine.apply_gate_action(session_id, FfiGateAction::Toggle { index: 1 }).unwrap();
        let both_on = engine.gate_state(session_id).unwrap();
        assert_eq!(both_on.inputs, vec![true, true]);
        assert!(both_on.output);

        // Wrong-kind access is rejected rather than panicking, same as
        // circuit/pixel sessions.
        assert!(engine.circuit_state(session_id).is_err());
    }

    #[test]
    fn array_builder_is_playable_end_to_end() {
        let engine = new_engine();
        let session_id = engine.start_build("array_builder".to_string()).expect("should start");
        assert!(matches!(engine.objective_status(session_id).unwrap(), FfiObjectiveStatus::InProgress));

        // /content/builds/array_builder.ron targets "12,45,7,88,3,61".
        for (index, value) in [12u8, 45, 7, 88, 3, 61].into_iter().enumerate() {
            engine.apply_array_action(session_id, FfiArrayAction::Set { index: index as u16, value }).unwrap();
        }
        let state = engine.array_state(session_id).unwrap();
        assert_eq!(state.slots, vec![12, 45, 7, 88, 3, 61]);
        assert!(matches!(engine.objective_status(session_id).unwrap(), FfiObjectiveStatus::Met));

        // Wrong-kind access is rejected rather than panicking.
        assert!(engine.circuit_state(session_id).is_err());
    }

    #[test]
    fn array_demo_tracks_sets_and_clears() {
        let engine = new_engine();
        let session_id = engine.start_array_demo(4);

        engine.apply_array_action(session_id, FfiArrayAction::Set { index: 1, value: 42 }).unwrap();
        let state = engine.array_state(session_id).unwrap();
        assert_eq!(state.slots, vec![0, 42, 0, 0]);
        assert_eq!(state.filled, vec![false, true, false, false]);

        engine.apply_array_action(session_id, FfiArrayAction::Clear { index: 1 }).unwrap();
        let cleared = engine.array_state(session_id).unwrap();
        assert_eq!(cleared.filled, vec![false, false, false, false]);

        // Demo sessions are always freeform.
        assert!(matches!(engine.objective_status(session_id).unwrap(), FfiObjectiveStatus::Met));
    }

    #[test]
    fn not_gate_demo_takes_a_single_input() {
        let engine = new_engine();
        let session_id = engine.start_gate_demo(FfiGateKind::Not, 1);
        let initial = engine.gate_state(session_id).unwrap();
        assert!(initial.output, "NOT of off should be on");

        engine.apply_gate_action(session_id, FfiGateAction::Toggle { index: 0 }).unwrap();
        let toggled = engine.gate_state(session_id).unwrap();
        assert!(!toggled.output);
    }
}
