//! Wraps a `bit_simulation::Simulation` with metadata (tier, objectives,
//! freeform flag) into a driveable `BuildSession`. See /ARCHITECTURE.md §5.
//!
//! `BuildSession` is generic over any `Simulation` impl — Binary Counter
//! and Digital Lock both ride `BinaryCircuitSim`, a future CPU build would
//! ride `CpuSim`, without this crate changing.
//!
//! `BuildDefinition` is the content-authored, serializable half (loaded
//! from `/content/builds/*.ron` by `bit-content`); `catalog::instantiate`
//! below is the small, closed, code-owned half that knows how to turn one
//! into a live `BuildSession` — see that module's doc comment for why this
//! can't be fully data-driven in a statically-typed generic engine.

use std::collections::HashMap;

use bit_simulation::{ObjectiveStatus, Simulation, StepResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BuildTier {
    Beginner,
    Core,
    Advanced,
    Extreme,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildDefinition {
    pub id: String,
    pub name: String,
    pub tier: BuildTier,
    #[serde(default)]
    pub description: String,
    /// Concepts this build lets the player practice/apply — surfaced as
    /// `build_refs` on the reverse side, from each concept's page.
    #[serde(default)]
    pub concept_ids: Vec<String>,
    /// Build-specific starting parameters (target values, grid size,
    /// patterns, ...), interpreted by `catalog::instantiate`. Mirrors how
    /// `bit_knowledge::InteractiveDemo` carries params for concept demos.
    #[serde(default)]
    pub params: HashMap<String, String>,
    /// Free Build mode: objectives are advisory only, never block or fail.
    pub freeform: bool,
}

/// A live, in-progress build. Swift drives this via `apply_action` /
/// `state()` / `objective_status()` through the FFI facade in bit-core.
pub struct BuildSession<S: Simulation> {
    definition: BuildDefinition,
    simulation: S,
    state: S::State,
    objectives: Vec<S::Objective>,
}

impl<S: Simulation> BuildSession<S> {
    pub fn new(definition: BuildDefinition, simulation: S, objectives: Vec<S::Objective>) -> Self {
        let state = simulation.initial_state();
        Self { definition, simulation, state, objectives }
    }

    pub fn definition(&self) -> &BuildDefinition {
        &self.definition
    }

    pub fn state(&self) -> &S::State {
        &self.state
    }

    /// The underlying simulation. Needed by callers (e.g. bit-ffi's gate
    /// demo, which must read `LogicGateSim::kind` to know which gate
    /// `state().output(kind)` should evaluate) that need a fact about the
    /// simulation itself, not just its current state.
    pub fn simulation(&self) -> &S {
        &self.simulation
    }

    pub fn apply_action(&mut self, action: S::Action) -> StepResult {
        self.simulation.apply(&mut self.state, action)
    }

    pub fn objective_status(&self) -> ObjectiveStatus {
        if self.definition.freeform {
            return ObjectiveStatus::Met;
        }
        self.simulation.check_objectives(&self.state, &self.objectives)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_simulation::binary_circuit::{Action, BinaryCircuitSim, Objective};

    #[test]
    fn build_session_tracks_objective_completion() {
        let definition = BuildDefinition {
            id: "binary_counter".into(),
            name: "Binary Counter".into(),
            tier: BuildTier::Beginner,
            description: String::new(),
            concept_ids: vec![],
            params: HashMap::new(),
            freeform: false,
        };
        let mut session = BuildSession::new(
            definition,
            BinaryCircuitSim { bit_width: 8 },
            vec![Objective { target_value: 5 }],
        );

        assert_eq!(session.objective_status(), ObjectiveStatus::InProgress);

        session.apply_action(Action::Toggle(5)); // +4
        session.apply_action(Action::Toggle(7)); // +1
        assert_eq!(session.state().value(), 5);
        assert_eq!(session.objective_status(), ObjectiveStatus::Met);
    }

    #[test]
    fn freeform_builds_are_always_considered_met() {
        let definition = BuildDefinition {
            id: "free_build".into(),
            name: "Free Build".into(),
            tier: BuildTier::Beginner,
            description: String::new(),
            concept_ids: vec![],
            params: HashMap::new(),
            freeform: true,
        };
        let session = BuildSession::new(definition, BinaryCircuitSim { bit_width: 8 }, vec![]);
        assert_eq!(session.objective_status(), ObjectiveStatus::Met);
    }

    #[test]
    fn build_session_is_generic_over_any_simulation() {
        use bit_simulation::logic_gate::{Action as GateAction, GateKind, LogicGateSim, Objective as GateObjective};

        // Same BuildSession<S> works unchanged for a second, unrelated
        // simulation — this is the genericity §5 of ARCHITECTURE.md relies
        // on for adding e.g. a CPU build later without touching this crate.
        let definition = BuildDefinition {
            id: "and_gate_demo".into(),
            name: "AND Gate".into(),
            tier: BuildTier::Beginner,
            description: String::new(),
            concept_ids: vec![],
            params: HashMap::new(),
            freeform: false,
        };
        let mut session = BuildSession::new(
            definition,
            LogicGateSim { kind: GateKind::And, input_count: 2 },
            vec![GateObjective { target_output: true }],
        );

        assert_eq!(session.objective_status(), ObjectiveStatus::InProgress);
        session.apply_action(GateAction::SetInput(0, true));
        session.apply_action(GateAction::SetInput(1, true));
        assert_eq!(session.objective_status(), ObjectiveStatus::Met);
    }
}

/// Maps a content-authored `BuildDefinition` to a concrete, live
/// `BuildSession`. This catalog is intentionally a small, closed match on
/// build id rather than a fully generic "any build, any simulation"
/// interpreter: `BuildSession<S>` is generic over `S::Action`/`S::State`/
/// `S::Objective`, which are different concrete types per simulation, so a
/// runtime-dynamic version would need type erasure (trait objects over
/// serialized actions/state) for no real benefit at this catalog's size —
/// four builds. Adding a fifth build means adding one match arm here, not
/// redesigning the type system. See /ARCHITECTURE.md §5 and §13.
pub mod catalog {
    use super::*;
    use bit_simulation::array_seq::{ArraySim, Objective as ArrayObjective};
    use bit_simulation::binary_circuit::{BinaryCircuitSim, Objective as CircuitObjective};
    use bit_simulation::logic_gate::LogicGateSim;
    use bit_simulation::pixel_grid::{Objective as PixelObjective, PixelGridSim};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BuildFactoryError {
        UnknownBuildId(String),
        MissingParam { build: String, param: String },
        InvalidParam { build: String, param: String, message: String },
    }

    /// A live session, tagged by which simulation it rides. Callers (the
    /// FFI layer) match on the variant to know which `Action`/`State`
    /// shape to drive it with.
    ///
    /// `LogicGate` sessions are never produced by `instantiate` below —
    /// no content-authored build rides `LogicGateSim` yet — they're
    /// constructed directly by bit-ffi for ad hoc concept demos (see its
    /// `start_gate_demo`). The variant lives here anyway so every live
    /// session, content-driven or ad hoc, shares one id space and one
    /// `end_build`/`objective_status` code path in bit-ffi.
    pub enum LiveBuildSession {
        BinaryCircuit(BuildSession<BinaryCircuitSim>),
        PixelGrid(BuildSession<PixelGridSim>),
        LogicGate(BuildSession<LogicGateSim>),
        Array(BuildSession<ArraySim>),
    }

    pub fn instantiate(definition: &BuildDefinition) -> Result<LiveBuildSession, BuildFactoryError> {
        match definition.id.as_str() {
            "binary_counter" | "digital_lock" | "bit_flag_system" => build_circuit_with_target(definition),
            "pixel_builder" | "pixel_art_studio" => build_pixel_pattern(definition),
            "array_builder" | "memory_simulator" => build_array_pattern(definition),
            other => Err(BuildFactoryError::UnknownBuildId(other.to_string())),
        }
    }

    fn param<'a>(definition: &'a BuildDefinition, key: &str) -> Result<&'a str, BuildFactoryError> {
        definition
            .params
            .get(key)
            .map(|s| s.as_str())
            .ok_or_else(|| BuildFactoryError::MissingParam { build: definition.id.clone(), param: key.to_string() })
    }

    fn parse_param<T: std::str::FromStr>(definition: &BuildDefinition, key: &str) -> Result<T, BuildFactoryError> {
        let raw = param(definition, key)?;
        raw.parse::<T>().map_err(|_| BuildFactoryError::InvalidParam {
            build: definition.id.clone(),
            param: key.to_string(),
            message: format!("could not parse '{raw}'"),
        })
    }

    /// Binary Counter, Digital Lock, and Bit Flag System are mechanically
    /// identical: toggle switches until the register reads a target value.
    /// What changes between them is presentation only (a counter framed as
    /// counting, a lock framed as a combination, a flag set framed as
    /// named permission bits) — that framing lives in each build's content
    /// (`name`, `description`) and, for flags, an optional `flag_labels`
    /// param the UI reads directly. None of that changes what gets
    /// simulated, so one helper serves all three.
    fn build_circuit_with_target(definition: &BuildDefinition) -> Result<LiveBuildSession, BuildFactoryError> {
        let bit_width: u8 = parse_param(definition, "bit_width")?;
        let target_value: u32 = parse_param(definition, "target_value")?;
        let session = BuildSession::new(
            definition.clone(),
            BinaryCircuitSim { bit_width },
            vec![CircuitObjective { target_value }],
        );
        Ok(LiveBuildSession::BinaryCircuit(session))
    }

    fn build_pixel_pattern(definition: &BuildDefinition) -> Result<LiveBuildSession, BuildFactoryError> {
        let width: usize = parse_param(definition, "width")?;
        let height: usize = parse_param(definition, "height")?;
        let pattern_str = param(definition, "pattern")?;

        if pattern_str.len() != width * height {
            return Err(BuildFactoryError::InvalidParam {
                build: definition.id.clone(),
                param: "pattern".to_string(),
                message: format!(
                    "pattern length {} does not match width*height {}",
                    pattern_str.len(),
                    width * height
                ),
            });
        }
        let target: Vec<bool> = pattern_str.chars().map(|c| c == '1').collect();

        let session =
            BuildSession::new(definition.clone(), PixelGridSim { width, height }, vec![PixelObjective { target }]);
        Ok(LiveBuildSession::PixelGrid(session))
    }

    /// Array Builder: fill every slot of a fixed-capacity array to match a
    /// target byte sequence. `target` is a comma-separated list of byte
    /// values (e.g. "12,45,0,7"); capacity is implied by its length, and
    /// every slot must end up filled — there's no content-authored build
    /// yet that wants a partially-filled target, so `target_filled` is
    /// always all-true rather than a second parseable param.
    fn build_array_pattern(definition: &BuildDefinition) -> Result<LiveBuildSession, BuildFactoryError> {
        let target_str = param(definition, "target")?;
        let target: Vec<u8> = target_str
            .split(',')
            .map(|s| s.trim().parse::<u8>())
            .collect::<Result<_, _>>()
            .map_err(|_| BuildFactoryError::InvalidParam {
                build: definition.id.clone(),
                param: "target".to_string(),
                message: format!("could not parse '{target_str}' as a comma-separated byte list"),
            })?;
        let capacity = target.len();
        let target_filled = vec![true; capacity];

        let session = BuildSession::new(
            definition.clone(),
            ArraySim { capacity },
            vec![ArrayObjective { target, target_filled }],
        );
        Ok(LiveBuildSession::Array(session))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use bit_simulation::binary_circuit::Action as CircuitAction;
        use bit_simulation::pixel_grid::Action as PixelAction;

        fn definition(id: &str, params: &[(&str, &str)]) -> BuildDefinition {
            BuildDefinition {
                id: id.to_string(),
                name: id.to_string(),
                tier: BuildTier::Beginner,
                description: String::new(),
                concept_ids: vec![],
                params: params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                freeform: false,
            }
        }

        #[test]
        fn binary_counter_is_winnable() {
            let def = definition("binary_counter", &[("bit_width", "8"), ("target_value", "10")]);
            let session = instantiate(&def).expect("binary_counter should instantiate");
            let LiveBuildSession::BinaryCircuit(mut session) = session else {
                panic!("expected a BinaryCircuit session");
            };
            assert_eq!(session.objective_status(), ObjectiveStatus::InProgress);
            session.apply_action(CircuitAction::Toggle(4)); // +8
            session.apply_action(CircuitAction::Toggle(6)); // +2
            assert_eq!(session.objective_status(), ObjectiveStatus::Met);
        }

        #[test]
        fn digital_lock_and_bit_flag_system_reuse_the_same_mechanic() {
            for id in ["digital_lock", "bit_flag_system"] {
                let def = definition(id, &[("bit_width", "8"), ("target_value", "182")]);
                let session = instantiate(&def).expect("should instantiate");
                assert!(matches!(session, LiveBuildSession::BinaryCircuit(_)));
            }
        }

        #[test]
        fn pixel_builder_is_winnable() {
            let def = definition("pixel_builder", &[("width", "2"), ("height", "2"), ("pattern", "1001")]);
            let session = instantiate(&def).expect("pixel_builder should instantiate");
            let LiveBuildSession::PixelGrid(mut session) = session else {
                panic!("expected a PixelGrid session");
            };
            assert_eq!(session.objective_status(), ObjectiveStatus::InProgress);
            session.apply_action(PixelAction::Set(0, 0, true));
            session.apply_action(PixelAction::Set(1, 1, true));
            assert_eq!(session.objective_status(), ObjectiveStatus::Met);
        }

        #[test]
        fn array_builder_is_winnable() {
            let def = definition("array_builder", &[("target", "5,12,0,7")]);
            let session = instantiate(&def).expect("array_builder should instantiate");
            let LiveBuildSession::Array(mut session) = session else {
                panic!("expected an Array session");
            };
            assert_eq!(session.objective_status(), ObjectiveStatus::InProgress);
            session.apply_action(bit_simulation::array_seq::Action::Set(0, 5));
            session.apply_action(bit_simulation::array_seq::Action::Set(1, 12));
            session.apply_action(bit_simulation::array_seq::Action::Set(2, 0));
            session.apply_action(bit_simulation::array_seq::Action::Set(3, 7));
            assert_eq!(session.objective_status(), ObjectiveStatus::Met);
        }

        #[test]
        fn array_builder_target_must_parse_as_byte_list() {
            let def = definition("array_builder", &[("target", "5,not-a-byte,0")]);
            assert!(matches!(instantiate(&def), Err(BuildFactoryError::InvalidParam { .. })));
        }

        #[test]
        fn unknown_build_id_is_rejected() {
            let def = definition("teleporter", &[]);
            let err = instantiate(&def).err().expect("should fail to instantiate");
            assert_eq!(err, BuildFactoryError::UnknownBuildId("teleporter".into()));
        }

        #[test]
        fn missing_param_is_reported() {
            let def = definition("binary_counter", &[("bit_width", "8")]);
            let err = instantiate(&def).err().expect("should fail to instantiate");
            assert_eq!(err, BuildFactoryError::MissingParam { build: "binary_counter".into(), param: "target_value".into() });
        }

        #[test]
        fn malformed_param_is_reported() {
            let def = definition("binary_counter", &[("bit_width", "8"), ("target_value", "not-a-number")]);
            assert!(matches!(instantiate(&def), Err(BuildFactoryError::InvalidParam { .. })));
        }

        #[test]
        fn pixel_pattern_length_mismatch_is_reported() {
            let def = definition("pixel_builder", &[("width", "2"), ("height", "2"), ("pattern", "101")]);
            assert!(matches!(instantiate(&def), Err(BuildFactoryError::InvalidParam { .. })));
        }
    }
}
