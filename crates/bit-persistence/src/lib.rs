//! Versioned local save state. See /ARCHITECTURE.md §8.
//!
//! `schema_version` and an explicit migration chain exist from the first
//! commit: "no fixed ending" means a save written today must still load
//! after years of content growth and app updates.

use std::collections::HashMap;

use bit_progression::MasteryState;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveState {
    pub schema_version: u32,
    pub mastery: HashMap<String, MasteryState>,
    pub bookmarks: Vec<String>,
    pub notes: HashMap<String, String>,
    /// Lifetime total of correct answers in Play's endless game mode —
    /// the plain number shown at the top of the game screen. Additive
    /// field within schema v1 (see `#[serde(default)]`): an existing save
    /// missing it just starts counting from 0, no migration function
    /// needed for a field that's safely zero-valued by default.
    #[serde(default)]
    pub game_total_correct: u32,
    /// Correct answers since the player last saw (and dismissed or
    /// accepted) a "ready for something new?" suggestion — reset on
    /// acknowledgment, drives when the next suggestion is offered.
    #[serde(default)]
    pub game_streak_since_suggestion: u32,
    /// "YYYY-MM-DD" of the last day any activity was recorded — the seam
    /// between this crate (pure storage) and `bit_core::BitEngine::
    /// record_daily_activity` (the pure date-math logic that reads/writes
    /// this field). Never computed here; Swift supplies "today" and Rust
    /// stays wall-clock-free, same as every other engine in this app.
    #[serde(default)]
    pub last_active_day: Option<String>,
    #[serde(default)]
    pub daily_streak: u32,
    #[serde(default)]
    pub longest_daily_streak: u32,
    /// "YYYY-MM-DD" -> number of engagement events that day (a correct
    /// answer, a discovery) — Profile's activity heatmap reads this
    /// directly. Small and unbounded-but-harmless: even years of daily
    /// use is a few thousand short string keys.
    #[serde(default)]
    pub activity_by_day: HashMap<String, u32>,
    /// One-time achievement ids already shown to the player (e.g.
    /// "first_mastery", "100_correct", "first_build") — a generic flag
    /// set, not a fixed enum, so a new milestone is just a new string
    /// constant on the Swift side, never a schema change here.
    #[serde(default)]
    pub celebrated_milestones: Vec<String>,
    /// "YYYY-MM-DD" -> whether that day's Puzzle of the Day (the single,
    /// date-seeded challenge every install gets, same as everyone else)
    /// was answered correctly. Presence of the key is "attempted today"
    /// regardless of the bool, so the UI can distinguish "not yet played"
    /// from "played and got it wrong" — both matter for the share card.
    #[serde(default)]
    pub daily_challenge_results: HashMap<String, bool>,
}

impl SaveState {
    pub fn new() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            mastery: HashMap::new(),
            bookmarks: Vec::new(),
            notes: HashMap::new(),
            game_total_correct: 0,
            game_streak_since_suggestion: 0,
            last_active_day: None,
            daily_streak: 0,
            longest_daily_streak: 0,
            activity_by_day: HashMap::new(),
            celebrated_milestones: Vec::new(),
            daily_challenge_results: HashMap::new(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("SaveState is always serializable")
    }

    /// Parses a save, running it through the migration chain first so
    /// callers never see anything but the current schema shape.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        migrate(&mut value);
        serde_json::from_value(value).map_err(|e| e.to_string())
    }
}

impl Default for SaveState {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs `value` forward to `CURRENT_SCHEMA_VERSION` in place. Each step is
/// a small, named function so a future migration is additive, not a
/// rewrite of this function.
fn migrate(value: &mut serde_json::Value) {
    let version = value.get("schema_version").and_then(|v| v.as_u64()).unwrap_or(0);
    if version < 1 {
        migrate_v0_to_v1(value);
    }
    // Next migration: `if version < 2 { migrate_v1_to_v2(value); }`
}

/// v0 saves predate `schema_version`/`bookmarks`/`notes` entirely.
fn migrate_v0_to_v1(value: &mut serde_json::Value) {
    let obj = value.as_object_mut().expect("save state is always a JSON object");
    obj.entry("bookmarks").or_insert_with(|| serde_json::Value::Array(vec![]));
    obj.entry("notes").or_insert_with(|| serde_json::Value::Object(Default::default()));
    obj.entry("mastery").or_insert_with(|| serde_json::Value::Object(Default::default()));
    obj.insert("schema_version".to_string(), serde_json::Value::from(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let mut state = SaveState::new();
        state.bookmarks.push("binary".into());
        state.mastery.insert("binary".into(), MasteryState::new());

        let json = state.to_json();
        let loaded = SaveState::from_json(&json).unwrap();
        assert_eq!(loaded.bookmarks, vec!["binary".to_string()]);
        assert!(loaded.mastery.contains_key("binary"));
    }

    #[test]
    fn migrates_a_pre_v1_save_missing_new_fields() {
        let legacy = r#"{"mastery": {}}"#;
        let loaded = SaveState::from_json(legacy).expect("legacy save should migrate cleanly");
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(loaded.bookmarks.is_empty());
    }

    #[test]
    fn game_counters_round_trip_and_default_to_zero_for_older_saves() {
        let mut state = SaveState::new();
        state.game_total_correct = 42;
        state.game_streak_since_suggestion = 7;
        let loaded = SaveState::from_json(&state.to_json()).unwrap();
        assert_eq!(loaded.game_total_correct, 42);
        assert_eq!(loaded.game_streak_since_suggestion, 7);

        // A save from before these fields existed shouldn't fail to load.
        let legacy = r#"{"schema_version": 1, "mastery": {}, "bookmarks": [], "notes": {}}"#;
        let loaded_legacy = SaveState::from_json(legacy).expect("save missing game counters should still load");
        assert_eq!(loaded_legacy.game_total_correct, 0);
        assert_eq!(loaded_legacy.game_streak_since_suggestion, 0);
    }

    #[test]
    fn daily_streak_fields_round_trip_and_default_for_older_saves() {
        let mut state = SaveState::new();
        state.last_active_day = Some("2026-09-01".into());
        state.daily_streak = 3;
        state.longest_daily_streak = 5;
        let loaded = SaveState::from_json(&state.to_json()).unwrap();
        assert_eq!(loaded.last_active_day, Some("2026-09-01".to_string()));
        assert_eq!(loaded.daily_streak, 3);
        assert_eq!(loaded.longest_daily_streak, 5);

        let legacy = r#"{"schema_version": 1, "mastery": {}, "bookmarks": [], "notes": {}}"#;
        let loaded_legacy = SaveState::from_json(legacy).expect("save missing daily streak fields should still load");
        assert_eq!(loaded_legacy.last_active_day, None);
        assert_eq!(loaded_legacy.daily_streak, 0);
        assert_eq!(loaded_legacy.longest_daily_streak, 0);
    }

    #[test]
    fn activity_by_day_round_trips_and_defaults_empty_for_older_saves() {
        let mut state = SaveState::new();
        state.activity_by_day.insert("2026-09-01".into(), 3);
        let loaded = SaveState::from_json(&state.to_json()).unwrap();
        assert_eq!(loaded.activity_by_day.get("2026-09-01"), Some(&3));

        let legacy = r#"{"schema_version": 1, "mastery": {}, "bookmarks": [], "notes": {}}"#;
        let loaded_legacy = SaveState::from_json(legacy).expect("save missing activity_by_day should still load");
        assert!(loaded_legacy.activity_by_day.is_empty());
    }

    #[test]
    fn celebrated_milestones_round_trip_and_default_empty_for_older_saves() {
        let mut state = SaveState::new();
        state.celebrated_milestones.push("first_mastery".into());
        let loaded = SaveState::from_json(&state.to_json()).unwrap();
        assert_eq!(loaded.celebrated_milestones, vec!["first_mastery".to_string()]);

        let legacy = r#"{"schema_version": 1, "mastery": {}, "bookmarks": [], "notes": {}}"#;
        let loaded_legacy = SaveState::from_json(legacy).expect("save missing celebrated_milestones should still load");
        assert!(loaded_legacy.celebrated_milestones.is_empty());
    }
}
