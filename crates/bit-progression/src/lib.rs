//! Mastery tracking and round pacing. See /ARCHITECTURE.md §7 and spec §3.
//!
//! `round_stage_for` is the single place that decides whether a concept is
//! still in its "psychologically gentle" introduction — no screen is
//! allowed to show a timer, life, or streak penalty on its own judgment;
//! it must ask this function first.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MasteryStage {
    Discovered,
    Practicing,
    Proficient,
    Mastered,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MasteryState {
    pub discovered: bool,
    pub practiced_count: u32,
    pub correct_count: u32,
    pub stage: MasteryStage,
    pub streak: u32,
    /// "YYYY-MM-DD" of the last practice attempt — plain storage only,
    /// this crate never compares dates itself (see /ARCHITECTURE.md's
    /// wall-clock-free rule); `bit_core::BitEngine::is_rusty` is what
    /// turns this plus a concept's `decay_days` into a yes/no answer.
    /// `#[serde(default)]` so mastery recorded before this field existed
    /// deserializes as `None` instead of failing to load.
    #[serde(default)]
    pub last_practiced_day: Option<String>,
}

impl MasteryState {
    pub fn new() -> Self {
        Self {
            discovered: false,
            practiced_count: 0,
            correct_count: 0,
            stage: MasteryStage::Discovered,
            streak: 0,
            last_practiced_day: None,
        }
    }

    pub fn accuracy(&self) -> f32 {
        if self.practiced_count == 0 {
            0.0
        } else {
            self.correct_count as f32 / self.practiced_count as f32
        }
    }

    pub fn record_discovery(&mut self) {
        self.discovered = true;
    }

    pub fn record_attempt(&mut self, correct: bool) {
        self.practiced_count += 1;
        if correct {
            self.correct_count += 1;
            self.streak += 1;
        } else {
            self.streak = 0;
        }
    }

    /// Promotes `stage` based on a concept's own thresholds (from its
    /// content-authored `MasteryRule` — see bit-knowledge). Call this
    /// after every `record_attempt`; it never demotes, and it's the only
    /// thing that moves `stage` off `Discovered`, which is what actually
    /// unlocks `RoundStage::TimedChallenge` in `round_stage_for` below.
    /// Without this, a concept could be practiced forever and never
    /// leave Guided Practice — a real gap this closes, not a hypothetical.
    pub fn advance_stage(&mut self, thresholds: StageThresholds) {
        if self.practiced_count == 0 {
            return;
        }
        let meets_bar = self.practiced_count >= thresholds.min_practice_correct && self.accuracy() >= thresholds.min_accuracy;
        match self.stage {
            MasteryStage::Discovered if meets_bar => self.stage = MasteryStage::Proficient,
            MasteryStage::Discovered => self.stage = MasteryStage::Practicing,
            MasteryStage::Practicing if meets_bar => self.stage = MasteryStage::Proficient,
            // Mastered requires a clean streak, at proficiency, at least
            // as long as the concept's own practice-count bar — i.e.
            // "prove it again, in a row, without the practice attempts
            // that got you here."
            MasteryStage::Proficient if self.streak >= thresholds.min_practice_correct => {
                self.stage = MasteryStage::Mastered
            }
            _ => {}
        }
    }
}

/// Mirrors the fields of `bit_knowledge::MasteryRule` that stage
/// promotion needs. A small local copy rather than a dependency on
/// bit-knowledge — this crate stays dependency-free by design (see
/// /ARCHITECTURE.md §1), and the caller (bit-core, which already depends
/// on both) is responsible for translating one into the other.
#[derive(Debug, Clone, Copy)]
pub struct StageThresholds {
    pub min_practice_correct: u32,
    pub min_accuracy: f32,
}

impl Default for MasteryState {
    fn default() -> Self {
        Self::new()
    }
}

/// See spec §3: Round 0 (Demonstration) -> Round 1 (Fiddle) -> Round 2
/// (Guided Practice) -> only then may pressure appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundStage {
    Demonstration,
    Fiddle,
    GuidedPractice,
    TimedChallenge,
}

pub fn round_stage_for(state: &MasteryState) -> RoundStage {
    if !state.discovered {
        return RoundStage::Demonstration;
    }
    if state.practiced_count == 0 {
        return RoundStage::Fiddle;
    }
    match state.stage {
        MasteryStage::Discovered | MasteryStage::Practicing => RoundStage::GuidedPractice,
        MasteryStage::Proficient | MasteryStage::Mastered => RoundStage::TimedChallenge,
    }
}

/// Pure gate: may the UI introduce timers/lives/streak pressure right now?
/// Matches spec §3's "do not immediately introduce timers, lives, streak
/// penalties, leaderboard pressure."
pub fn allows_time_pressure(stage: RoundStage) -> bool {
    matches!(stage, RoundStage::TimedChallenge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undiscovered_concept_starts_at_demonstration() {
        let state = MasteryState::new();
        assert_eq!(round_stage_for(&state), RoundStage::Demonstration);
        assert!(!allows_time_pressure(round_stage_for(&state)));
    }

    #[test]
    fn discovered_but_unpracticed_is_fiddle() {
        let mut state = MasteryState::new();
        state.record_discovery();
        assert_eq!(round_stage_for(&state), RoundStage::Fiddle);
    }

    #[test]
    fn practicing_stage_never_allows_time_pressure() {
        let mut state = MasteryState::new();
        state.record_discovery();
        state.record_attempt(true);
        state.stage = MasteryStage::Practicing;
        let stage = round_stage_for(&state);
        assert_eq!(stage, RoundStage::GuidedPractice);
        assert!(!allows_time_pressure(stage));
    }

    #[test]
    fn proficient_stage_allows_time_pressure() {
        let mut state = MasteryState::new();
        state.record_discovery();
        state.record_attempt(true);
        state.stage = MasteryStage::Proficient;
        let stage = round_stage_for(&state);
        assert_eq!(stage, RoundStage::TimedChallenge);
        assert!(allows_time_pressure(stage));
    }

    fn thresholds() -> StageThresholds {
        StageThresholds { min_practice_correct: 3, min_accuracy: 0.8 }
    }

    #[test]
    fn one_attempt_moves_off_discovered_into_practicing() {
        let mut state = MasteryState::new();
        state.record_discovery();
        state.record_attempt(false);
        state.advance_stage(thresholds());
        assert_eq!(state.stage, MasteryStage::Practicing);
    }

    #[test]
    fn meeting_the_bar_promotes_straight_to_proficient() {
        let mut state = MasteryState::new();
        state.record_discovery();
        for _ in 0..3 {
            state.record_attempt(true);
            state.advance_stage(thresholds());
        }
        assert_eq!(state.stage, MasteryStage::Proficient);
        assert_eq!(round_stage_for(&state), RoundStage::TimedChallenge);
    }

    #[test]
    fn a_miss_after_reaching_proficient_does_not_demote() {
        let mut state = MasteryState::new();
        state.record_discovery();
        for _ in 0..3 {
            state.record_attempt(true);
            state.advance_stage(thresholds());
        }
        assert_eq!(state.stage, MasteryStage::Proficient);
        state.record_attempt(false); // breaks the streak, doesn't meet the bar this round
        state.advance_stage(thresholds());
        assert_eq!(state.stage, MasteryStage::Proficient, "stage should never regress");
    }

    #[test]
    fn mastered_requires_a_fresh_streak_at_proficient() {
        let mut state = MasteryState::new();
        state.record_discovery();
        for _ in 0..3 {
            state.record_attempt(true);
            state.advance_stage(thresholds());
        }
        assert_eq!(state.stage, MasteryStage::Proficient);
        assert_ne!(state.stage, MasteryStage::Mastered, "3 correct alone shouldn't already be Mastered");

        for _ in 0..3 {
            state.record_attempt(true);
            state.advance_stage(thresholds());
        }
        assert_eq!(state.stage, MasteryStage::Mastered);
    }

    #[test]
    fn low_accuracy_stays_in_practicing_despite_high_attempt_count() {
        let mut state = MasteryState::new();
        state.record_discovery();
        for correct in [true, false, false, false, false] {
            state.record_attempt(correct);
            state.advance_stage(thresholds());
        }
        assert_eq!(state.stage, MasteryStage::Practicing);
        assert_eq!(round_stage_for(&state), RoundStage::GuidedPractice);
    }

    #[test]
    fn last_practiced_day_deserializes_as_none_for_pre_existing_saves() {
        let legacy = r#"{"discovered":true,"practiced_count":2,"correct_count":2,"stage":"Practicing","streak":2}"#;
        let state: MasteryState = serde_json::from_str(legacy).expect("mastery missing last_practiced_day should still deserialize");
        assert_eq!(state.last_practiced_day, None);
    }
}
