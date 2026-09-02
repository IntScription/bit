//! Pure, deterministic, step-driven simulations with zero UI/timing
//! dependency. See /ARCHITECTURE.md §5.
//!
//! Every simulation implements `Simulation` and is driven by explicit
//! `apply(state, action)` calls — never a wall-clock tick — so the same
//! simulation backs Build, Lab, and Play, and is fully unit-testable.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub accepted: bool,
    pub message: Option<String>,
}

impl StepResult {
    pub fn ok() -> Self {
        Self { accepted: true, message: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveStatus {
    InProgress,
    Met,
}

/// A simulation owns its own `Action`, `State`, and `Objective` types —
/// there is deliberately no shared cross-domain predicate language (a
/// binary circuit's objectives look nothing like a network simulation's).
/// What's shared is the *shape*: apply is the only way state changes, and
/// objectives are evaluated against state, never against the action log.
pub trait Simulation {
    type Action;
    type State: Clone;
    type Objective;

    fn initial_state(&self) -> Self::State;
    fn apply(&self, state: &mut Self::State, action: Self::Action) -> StepResult;
    fn check_objectives(&self, state: &Self::State, objectives: &[Self::Objective]) -> ObjectiveStatus;
}

/// The simplest simulation in the vertical slice (§11): `n` toggleable
/// switches whose active bits sum to a value, exactly as demonstrated in
/// spec §3 Round 0 ("128 64 32 16 8 4 2 1").
pub mod binary_circuit {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct BinaryCircuitSim {
        pub bit_width: u8,
    }

    #[derive(Debug, Clone)]
    pub struct State {
        pub switches: Vec<bool>, // index 0 = most significant bit
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Action {
        Toggle(usize),
        Set(usize, bool),
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Objective {
        pub target_value: u32,
    }

    impl State {
        pub fn value(&self) -> u32 {
            self.switches
                .iter()
                .rev()
                .enumerate()
                .filter(|(_, &on)| on)
                .map(|(i, _)| 1u32 << i)
                .sum()
        }

        /// The same switches, read as two's complement instead of plain
        /// unsigned binary — the concept page for Two's Complement is
        /// literally "here's the switches you already know, interpreted
        /// differently," so this deliberately isn't a separate
        /// simulation. The sign bit (switches[0], the MSB) carries
        /// negative weight instead of positive.
        pub fn signed_value(&self) -> i32 {
            let unsigned = self.value() as i64;
            let bit_width = self.switches.len() as u32;
            if self.switches.first().copied().unwrap_or(false) {
                (unsigned - (1i64 << bit_width)) as i32
            } else {
                unsigned as i32
            }
        }
    }

    impl Simulation for BinaryCircuitSim {
        type Action = Action;
        type State = State;
        type Objective = Objective;

        fn initial_state(&self) -> State {
            State { switches: vec![false; self.bit_width as usize] }
        }

        fn apply(&self, state: &mut State, action: Action) -> StepResult {
            match action {
                Action::Toggle(i) => match state.switches.get_mut(i) {
                    Some(s) => {
                        *s = !*s;
                        StepResult::ok()
                    }
                    None => StepResult { accepted: false, message: Some("switch index out of range".into()) },
                },
                Action::Set(i, value) => match state.switches.get_mut(i) {
                    Some(s) => {
                        *s = value;
                        StepResult::ok()
                    }
                    None => StepResult { accepted: false, message: Some("switch index out of range".into()) },
                },
            }
        }

        fn check_objectives(&self, state: &State, objectives: &[Objective]) -> ObjectiveStatus {
            if objectives.iter().all(|o| o.target_value == state.value()) {
                ObjectiveStatus::Met
            } else {
                ObjectiveStatus::InProgress
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn toggling_switches_produces_positional_value() {
            let sim = BinaryCircuitSim { bit_width: 8 };
            let mut state = sim.initial_state();
            // 1011 = 8 + 2 + 1 = 11, in the low nibble of an 8-bit register.
            sim.apply(&mut state, Action::Toggle(4)); // bit 8
            sim.apply(&mut state, Action::Toggle(6)); // bit 2
            sim.apply(&mut state, Action::Toggle(7)); // bit 1
            assert_eq!(state.value(), 11);
        }

        #[test]
        fn objective_met_when_value_matches() {
            let sim = BinaryCircuitSim { bit_width: 8 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::Set(7, true));
            let objectives = [Objective { target_value: 1 }];
            assert_eq!(sim.check_objectives(&state, &objectives), ObjectiveStatus::Met);
        }

        #[test]
        fn rejects_out_of_range_toggle() {
            let sim = BinaryCircuitSim { bit_width: 4 };
            let mut state = sim.initial_state();
            let result = sim.apply(&mut state, Action::Toggle(10));
            assert!(!result.accepted);
        }

        #[test]
        fn signed_value_matches_unsigned_when_sign_bit_is_off() {
            let sim = BinaryCircuitSim { bit_width: 8 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::Toggle(7)); // +1
            assert_eq!(state.value(), 1);
            assert_eq!(state.signed_value(), 1);
        }

        #[test]
        fn signed_value_is_negative_when_sign_bit_is_on() {
            let sim = BinaryCircuitSim { bit_width: 8 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::Toggle(0)); // sign bit only: 10000000
            assert_eq!(state.value(), 128);
            assert_eq!(state.signed_value(), -128);
        }

        #[test]
        fn all_ones_is_signed_negative_one() {
            let sim = BinaryCircuitSim { bit_width: 8 };
            let mut state = sim.initial_state();
            for i in 0..8 {
                sim.apply(&mut state, Action::Toggle(i));
            }
            assert_eq!(state.value(), 255);
            assert_eq!(state.signed_value(), -1);
        }
    }
}

/// A single configurable logic gate: AND / OR / XOR / NOT over `n` boolean
/// inputs. Backs the AND/OR/XOR/NOT concept demos and the Lab's logic-gate
/// playground. Wiring several gates together (the CORE-tier "Logic Gate
/// Machine" build) is a later, separate simulation built on top of this
/// one — not this module's job.
pub mod logic_gate {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    pub enum GateKind {
        And,
        Or,
        Xor,
        Not,
    }

    impl GateKind {
        /// Matches this gate's canonical concept id in the content graph
        /// (see /content/concepts/foundations/logic/*.ron), so challenge
        /// generation can map a concept id straight to a gate kind without
        /// a separate lookup table.
        pub fn from_concept_id(id: &str) -> Option<Self> {
            match id {
                "and" => Some(GateKind::And),
                "or" => Some(GateKind::Or),
                "xor" => Some(GateKind::Xor),
                "not" => Some(GateKind::Not),
                _ => None,
            }
        }

        pub fn default_input_count(self) -> usize {
            match self {
                GateKind::Not => 1,
                _ => 2,
            }
        }
    }

    /// Pure evaluation, reused by both the simulation below and
    /// `bit-challenge`'s gate-evaluation generator so the two never drift.
    /// XOR generalizes to n inputs as odd parity, matching how a chain of
    /// binary XOR gates behaves.
    pub fn evaluate(kind: GateKind, inputs: &[bool]) -> bool {
        match kind {
            GateKind::And => inputs.iter().all(|&b| b),
            GateKind::Or => inputs.iter().any(|&b| b),
            GateKind::Xor => inputs.iter().filter(|&&b| b).count() % 2 == 1,
            GateKind::Not => !inputs.first().copied().unwrap_or(false),
        }
    }

    #[derive(Debug, Clone)]
    pub struct LogicGateSim {
        pub kind: GateKind,
        pub input_count: usize,
    }

    #[derive(Debug, Clone)]
    pub struct State {
        pub inputs: Vec<bool>,
    }

    impl State {
        pub fn output(&self, kind: GateKind) -> bool {
            evaluate(kind, &self.inputs)
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Action {
        Toggle(usize),
        SetInput(usize, bool),
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Objective {
        pub target_output: bool,
    }

    impl Simulation for LogicGateSim {
        type Action = Action;
        type State = State;
        type Objective = Objective;

        fn initial_state(&self) -> State {
            State { inputs: vec![false; self.input_count] }
        }

        fn apply(&self, state: &mut State, action: Action) -> StepResult {
            match action {
                Action::Toggle(i) => match state.inputs.get_mut(i) {
                    Some(b) => {
                        *b = !*b;
                        StepResult::ok()
                    }
                    None => StepResult { accepted: false, message: Some("input index out of range".into()) },
                },
                Action::SetInput(i, value) => match state.inputs.get_mut(i) {
                    Some(b) => {
                        *b = value;
                        StepResult::ok()
                    }
                    None => StepResult { accepted: false, message: Some("input index out of range".into()) },
                },
            }
        }

        fn check_objectives(&self, state: &State, objectives: &[Objective]) -> ObjectiveStatus {
            let output = state.output(self.kind);
            if objectives.iter().all(|o| o.target_output == output) {
                ObjectiveStatus::Met
            } else {
                ObjectiveStatus::InProgress
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn and_is_true_only_when_all_inputs_true() {
            assert!(evaluate(GateKind::And, &[true, true]));
            assert!(!evaluate(GateKind::And, &[true, false]));
        }

        #[test]
        fn or_is_true_when_any_input_true() {
            assert!(evaluate(GateKind::Or, &[false, true]));
            assert!(!evaluate(GateKind::Or, &[false, false]));
        }

        #[test]
        fn xor_is_true_on_odd_parity() {
            assert!(evaluate(GateKind::Xor, &[true, false]));
            assert!(!evaluate(GateKind::Xor, &[true, true]));
            assert!(evaluate(GateKind::Xor, &[true, true, true]));
        }

        #[test]
        fn not_inverts_its_single_input() {
            assert!(evaluate(GateKind::Not, &[false]));
            assert!(!evaluate(GateKind::Not, &[true]));
        }

        #[test]
        fn toggling_inputs_updates_gate_output() {
            let sim = LogicGateSim { kind: GateKind::And, input_count: 2 };
            let mut state = sim.initial_state();
            assert!(!state.output(sim.kind));

            sim.apply(&mut state, Action::Toggle(0));
            sim.apply(&mut state, Action::Toggle(1));
            assert!(state.output(sim.kind));
        }

        #[test]
        fn objective_met_when_output_matches_target() {
            let sim = LogicGateSim { kind: GateKind::Or, input_count: 2 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::SetInput(0, true));
            let objectives = [Objective { target_output: true }];
            assert_eq!(sim.check_objectives(&state, &objectives), ObjectiveStatus::Met);
        }

        #[test]
        fn rejects_out_of_range_input() {
            let sim = LogicGateSim { kind: GateKind::Not, input_count: 1 };
            let mut state = sim.initial_state();
            let result = sim.apply(&mut state, Action::Toggle(5));
            assert!(!result.accepted);
        }

        #[test]
        fn gate_kind_resolves_from_concept_id() {
            assert_eq!(GateKind::from_concept_id("xor"), Some(GateKind::Xor));
            assert_eq!(GateKind::from_concept_id("nand"), None);
        }
    }
}

/// A grid of on/off pixels — each pixel is, literally, a bit. Backs the
/// Pixel Builder build: recreating a target pattern is the same mechanic
/// as [`binary_circuit`], just arranged in two dimensions instead of one
/// row, which is why pixel art is a natural early Build rather than a
/// separate concept.
pub mod pixel_grid {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct PixelGridSim {
        pub width: usize,
        pub height: usize,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct State {
        pub width: usize,
        pub height: usize,
        pub pixels: Vec<bool>, // row-major, len == width * height
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Action {
        Toggle(usize, usize),
        Set(usize, usize, bool),
    }

    #[derive(Debug, Clone)]
    pub struct Objective {
        pub target: Vec<bool>, // row-major, same length as the grid
    }

    impl State {
        fn index(&self, x: usize, y: usize) -> Option<usize> {
            if x < self.width && y < self.height {
                Some(y * self.width + x)
            } else {
                None
            }
        }
    }

    impl Simulation for PixelGridSim {
        type Action = Action;
        type State = State;
        type Objective = Objective;

        fn initial_state(&self) -> State {
            State { width: self.width, height: self.height, pixels: vec![false; self.width * self.height] }
        }

        fn apply(&self, state: &mut State, action: Action) -> StepResult {
            let (x, y, new_value) = match action {
                Action::Toggle(x, y) => (x, y, None),
                Action::Set(x, y, value) => (x, y, Some(value)),
            };
            match state.index(x, y) {
                Some(i) => {
                    state.pixels[i] = new_value.unwrap_or(!state.pixels[i]);
                    StepResult::ok()
                }
                None => StepResult { accepted: false, message: Some("pixel coordinate out of range".into()) },
            }
        }

        fn check_objectives(&self, state: &State, objectives: &[Objective]) -> ObjectiveStatus {
            if objectives.iter().all(|o| o.target == state.pixels) {
                ObjectiveStatus::Met
            } else {
                ObjectiveStatus::InProgress
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn toggling_a_pixel_flips_only_that_pixel() {
            let sim = PixelGridSim { width: 2, height: 2 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::Toggle(1, 0));
            assert_eq!(state.pixels, vec![false, true, false, false]);
        }

        #[test]
        fn objective_met_when_grid_matches_target_pattern() {
            let sim = PixelGridSim { width: 2, height: 2 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::Set(0, 0, true));
            sim.apply(&mut state, Action::Set(1, 1, true));
            let objectives = [Objective { target: vec![true, false, false, true] }];
            assert_eq!(sim.check_objectives(&state, &objectives), ObjectiveStatus::Met);
        }

        #[test]
        fn rejects_out_of_range_coordinate() {
            let sim = PixelGridSim { width: 2, height: 2 };
            let mut state = sim.initial_state();
            let result = sim.apply(&mut state, Action::Toggle(5, 5));
            assert!(!result.accepted);
        }
    }
}

/// A fixed-capacity linear sequence of byte-sized cells. This is the one
/// simulation behind three Data Structures concepts — Arrays, Stacks, and
/// Queues are the same underlying "slots you can read and write" mechanic,
/// just with different access discipline (direct index vs. LIFO end vs.
/// FIFO ends). Rather than modeling that discipline in the simulation
/// itself, `ArraySim` stays a dumb indexed slot machine and each concept's
/// interactive demo / challenge generator enforces its own access pattern
/// on top — exactly how `binary_circuit::State::signed_value` reinterprets
/// the same switches instead of introducing a second simulation.
pub mod array_seq {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct ArraySim {
        pub capacity: usize,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct State {
        pub slots: Vec<u8>,
        pub filled: Vec<bool>, // tracks which slots hold a real value vs. empty
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Action {
        Set(usize, u8),
        Clear(usize),
    }

    #[derive(Debug, Clone)]
    pub struct Objective {
        pub target: Vec<u8>,
        pub target_filled: Vec<bool>,
    }

    impl Simulation for ArraySim {
        type Action = Action;
        type State = State;
        type Objective = Objective;

        fn initial_state(&self) -> State {
            State { slots: vec![0; self.capacity], filled: vec![false; self.capacity] }
        }

        fn apply(&self, state: &mut State, action: Action) -> StepResult {
            match action {
                Action::Set(i, value) => match (state.slots.get_mut(i), state.filled.get_mut(i)) {
                    (Some(slot), Some(filled)) => {
                        *slot = value;
                        *filled = true;
                        StepResult::ok()
                    }
                    _ => StepResult { accepted: false, message: Some("array index out of range".into()) },
                },
                Action::Clear(i) => match (state.slots.get_mut(i), state.filled.get_mut(i)) {
                    (Some(slot), Some(filled)) => {
                        *slot = 0;
                        *filled = false;
                        StepResult::ok()
                    }
                    _ => StepResult { accepted: false, message: Some("array index out of range".into()) },
                },
            }
        }

        fn check_objectives(&self, state: &State, objectives: &[Objective]) -> ObjectiveStatus {
            if objectives
                .iter()
                .all(|o| o.target == state.slots && o.target_filled == state.filled)
            {
                ObjectiveStatus::Met
            } else {
                ObjectiveStatus::InProgress
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn setting_a_slot_marks_it_filled() {
            let sim = ArraySim { capacity: 4 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::Set(2, 42));
            assert_eq!(state.slots, vec![0, 0, 42, 0]);
            assert_eq!(state.filled, vec![false, false, true, false]);
        }

        #[test]
        fn clearing_a_slot_resets_it() {
            let sim = ArraySim { capacity: 3 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::Set(0, 9));
            sim.apply(&mut state, Action::Clear(0));
            assert_eq!(state.slots, vec![0, 0, 0]);
            assert_eq!(state.filled, vec![false, false, false]);
        }

        #[test]
        fn objective_met_when_slots_and_filled_match_target() {
            let sim = ArraySim { capacity: 2 };
            let mut state = sim.initial_state();
            sim.apply(&mut state, Action::Set(0, 5));
            let objectives = [Objective { target: vec![5, 0], target_filled: vec![true, false] }];
            assert_eq!(sim.check_objectives(&state, &objectives), ObjectiveStatus::Met);
        }

        #[test]
        fn rejects_out_of_range_index() {
            let sim = ArraySim { capacity: 2 };
            let mut state = sim.initial_state();
            let result = sim.apply(&mut state, Action::Set(5, 1));
            assert!(!result.accepted);
        }
    }
}
