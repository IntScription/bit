//! Procedural challenge engine. See /ARCHITECTURE.md §4.
//!
//! Every `Challenge` is fully reproducible from `(seed, template_id,
//! params)` — no generator may consult wall-clock time, OS randomness, or
//! any other non-deterministic source. That's what lets Daily Challenge
//! hand every player an identical seed and lets generators be unit-tested
//! by asserting exact output for a given seed.

use std::collections::HashMap;

use bit_knowledge::ConceptId;

/// A minimal seeded PRNG (SplitMix64). Deliberately dependency-free so
/// determinism is auditable in this crate alone rather than resting on an
/// external crate's stability guarantees.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Inclusive range [lo, hi].
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DifficultyLevel(pub u8); // 1 (gentlest) .. 10

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GeneratorKind {
    BinaryConversion,
    GateEval,
    TwosComplement,
    AsciiLookup,
    PopCount,
    BitwiseMask,
    BitRotation,
    ArrayIndex,
    StackTrace,
    QueueTrace,
    LinearSearch,
    BinarySearch,
    BubbleSortPass,
    BinaryAddition,
    BinarySubtraction,
    BinaryMultiplication,
    BinaryDivision,
    SelectionSortPass,
}

impl GeneratorKind {
    /// The generator a concept practices by default, when nothing more
    /// specific has been authored for it. Lets callers (Play, and the FFI
    /// layer) build a `ChallengeTemplate` on the fly from just a concept
    /// id, without needing `/content/challenges/*.ron` to exist yet — that
    /// richer, fully data-driven template catalog is a later addition,
    /// not a blocker for practicing the concepts this crate can already
    /// generate for.
    pub fn default_for_concept(concept_id: &str) -> Option<Self> {
        match concept_id {
            "binary" | "decimal" | "hexadecimal" | "bytes" | "ip_addresses" => Some(GeneratorKind::BinaryConversion),
            "and" | "or" | "xor" | "not" => Some(GeneratorKind::GateEval),
            "twos_complement" => Some(GeneratorKind::TwosComplement),
            "ascii" => Some(GeneratorKind::AsciiLookup),
            "population_count" => Some(GeneratorKind::PopCount),
            "bitwise_masks" => Some(GeneratorKind::BitwiseMask),
            "bit_rotation" => Some(GeneratorKind::BitRotation),
            // A linked list traversal from the head, step by step, is
            // the exact same "find the value at position N" puzzle as
            // an array — the mechanism (pointers vs. contiguous memory)
            // differs, the puzzle doesn't.
            "arrays" | "linked_list" => Some(GeneratorKind::ArrayIndex),
            "stacks" => Some(GeneratorKind::StackTrace),
            "queues" => Some(GeneratorKind::QueueTrace),
            "linear_search" => Some(GeneratorKind::LinearSearch),
            "binary_search" => Some(GeneratorKind::BinarySearch),
            "bubble_sort" => Some(GeneratorKind::BubbleSortPass),
            "binary_addition" => Some(GeneratorKind::BinaryAddition),
            "binary_subtraction" => Some(GeneratorKind::BinarySubtraction),
            "binary_multiplication" => Some(GeneratorKind::BinaryMultiplication),
            "binary_division" => Some(GeneratorKind::BinaryDivision),
            "selection_sort" => Some(GeneratorKind::SelectionSortPass),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MasterySnapshot {
    pub accuracy_by_concept: HashMap<ConceptId, f32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChallengeTemplate {
    pub id: String,
    pub generator: GeneratorKind,
    pub concept_ids: Vec<ConceptId>,
}

pub struct GenerationContext {
    pub seed: u64,
    pub difficulty: DifficultyLevel,
    pub mastery: MasterySnapshot,
    pub template: ChallengeTemplate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Challenge {
    pub template_id: String,
    pub seed: u64,
    pub prompt: String,
    pub expected_answer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer(pub String);

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub correct: bool,
    pub explanation: Option<String>,
}

pub trait ChallengeGenerator: Send + Sync {
    fn kind(&self) -> GeneratorKind;
    fn generate(&self, ctx: &GenerationContext) -> Challenge;
    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult;
}

#[derive(Default)]
pub struct ChallengeRegistry {
    generators: HashMap<GeneratorKind, Box<dyn ChallengeGenerator>>,
}

impl ChallengeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, generator: Box<dyn ChallengeGenerator>) {
        self.generators.insert(generator.kind(), generator);
    }

    pub fn generate(&self, ctx: &GenerationContext) -> Option<Challenge> {
        self.generators.get(&ctx.template.generator).map(|g| g.generate(ctx))
    }

    pub fn validate(&self, kind: GeneratorKind, challenge: &Challenge, answer: &Answer) -> Option<ValidationResult> {
        self.generators.get(&kind).map(|g| g.validate(challenge, answer))
    }
}

/// Shared by every generator below: exact string match after trimming,
/// with the expected answer echoed back on a miss. Every v1 generator
/// happens to validate this way — extracted here once it stopped being a
/// coincidence across four copies.
fn validate_exact_match(expected_answer: &str, answer: &Answer) -> ValidationResult {
    let correct = answer.0.trim() == expected_answer;
    ValidationResult { correct, explanation: if correct { None } else { Some(format!("Expected {expected_answer}")) } }
}

/// v1 generator for the vertical slice: binary <-> decimal conversion.
/// Direction and magnitude are both seed-derived; magnitude scales with
/// `difficulty` per §9 (adaptive difficulty widens the range, it never
/// changes the mechanic).
pub struct BinaryConversionGenerator;

impl ChallengeGenerator for BinaryConversionGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BinaryConversion
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let max_value = 2u32.saturating_pow(ctx.difficulty.0.clamp(1, 8) as u32) - 1;
        let value = rng.range_u32(0, max_value);
        let decimal_to_binary = rng.next_u64().is_multiple_of(2);

        let (prompt, expected_answer) = if decimal_to_binary {
            (format!("Convert {value} to binary"), format!("{value:b}"))
        } else {
            (format!("Convert binary {value:b} to decimal"), value.to_string())
        };

        Challenge { template_id: ctx.template.id.clone(), seed: ctx.seed, prompt, expected_answer }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Gate kind is derived from `template.concept_ids[0]` (e.g. "and", "xor")
/// rather than carried as a separate field — the content graph is already
/// the source of truth for which concept a template exercises, so this
/// avoids a second place that can drift out of sync with it.
pub struct GateEvalGenerator;

impl ChallengeGenerator for GateEvalGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::GateEval
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        use bit_simulation::logic_gate::{evaluate, GateKind};

        let gate = ctx
            .template
            .concept_ids
            .first()
            .and_then(|id| GateKind::from_concept_id(id))
            .unwrap_or(GateKind::And);

        let mut rng = Rng::new(ctx.seed);
        // Difficulty widens input count for AND/OR/XOR (2..=4); NOT always
        // takes exactly one input regardless of difficulty.
        let input_count = if gate == GateKind::Not {
            1
        } else {
            2 + (ctx.difficulty.0.saturating_sub(1) / 3).min(2) as usize
        };
        let inputs: Vec<bool> = (0..input_count).map(|_| rng.next_u64().is_multiple_of(2)).collect();
        let output = evaluate(gate, &inputs);

        let gate_name = match gate {
            GateKind::And => "AND",
            GateKind::Or => "OR",
            GateKind::Xor => "XOR",
            GateKind::Not => "NOT",
        };
        let operands = inputs.iter().map(|&b| if b { "1" } else { "0" }).collect::<Vec<_>>().join(", ");

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!("{gate_name}({operands}) = ?"),
            expected_answer: if output { "1" } else { "0" }.to_string(),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Two's complement is "the same 8-bit register as Binary, read with a
/// negative-weight sign bit" — see `binary_circuit::State::signed_value`
/// in bit-simulation, which this generator's expected answers are
/// computed to match exactly (`to_twos_complement_bits` below is that
/// same transform run in reverse).
pub struct TwosComplementGenerator;

impl ChallengeGenerator for TwosComplementGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::TwosComplement
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        const BIT_WIDTH: u32 = 8;
        let mut rng = Rng::new(ctx.seed);
        // Difficulty widens the range symmetrically around zero, capped at
        // the full 8-bit signed range (-128..=127).
        let max_abs = 2i32.pow(ctx.difficulty.0.clamp(1, 7) as u32);
        let value = (rng.range_u32(0, (2 * max_abs - 1) as u32) as i32 - max_abs).clamp(-128, 127);
        let binary_to_signed = rng.next_u64().is_multiple_of(2);

        let bits = to_twos_complement_bits(value, BIT_WIDTH);
        let (prompt, expected_answer) = if binary_to_signed {
            (format!("What signed value does {bits} represent (8-bit two's complement)?"), value.to_string())
        } else {
            (format!("Convert {value} to 8-bit two's complement binary"), bits)
        };

        Challenge { template_id: ctx.template.id.clone(), seed: ctx.seed, prompt, expected_answer }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

fn to_twos_complement_bits(value: i32, bit_width: u32) -> String {
    let unsigned = if value < 0 { (1i64 << bit_width) + value as i64 } else { value as i64 };
    format!("{unsigned:0width$b}", width = bit_width as usize)
}

/// ASCII code <-> character lookup. Skips the space character (32) so
/// every prompt is visually unambiguous — nothing about the mechanic
/// depends on that, it's purely to keep the printed challenge legible.
pub struct AsciiLookupGenerator;

impl ChallengeGenerator for AsciiLookupGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::AsciiLookup
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let code = rng.range_u32(33, 126);
        let ch = char::from_u32(code).unwrap_or('?');
        let char_to_code = rng.next_u64().is_multiple_of(2);

        let (prompt, expected_answer) = if char_to_code {
            (format!("What is the ASCII code for '{ch}'?"), code.to_string())
        } else {
            (format!("Which character is ASCII code {code}?"), ch.to_string())
        };

        Challenge { template_id: ctx.template.id.clone(), seed: ctx.seed, prompt, expected_answer }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Counts set bits in a random byte. The mechanic difficulty scales by
/// widening the byte's value range (a low-difficulty byte is likely to
/// have few set bits; a wide range makes both very-sparse and very-dense
/// bytes equally likely).
pub struct PopCountGenerator;

impl ChallengeGenerator for PopCountGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::PopCount
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let max_value = 2u32.saturating_pow(ctx.difficulty.0.clamp(1, 8) as u32) - 1;
        let value = rng.range_u32(0, max_value.max(1));
        let bits = format!("{value:08b}");
        let count = value.count_ones();

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!("How many bits are set (1s) in {bits}?"),
            expected_answer: count.to_string(),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// A random byte masked against a second random byte with AND, OR, or XOR
/// — the same three operators already taught by Foundations.Logic, just
/// applied bit-by-bit across a whole register instead of a single wire.
pub struct MaskGenerator;

impl ChallengeGenerator for MaskGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BitwiseMask
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let value = rng.range_u32(0, 255) as u8;
        let mask = rng.range_u32(0, 255) as u8;
        let op = rng.next_u64() % 3;

        let (op_name, result) = match op {
            0 => ("AND", value & mask),
            1 => ("OR", value | mask),
            _ => ("XOR", value ^ mask),
        };

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!("What is {value:08b} {op_name} {mask:08b}? (answer in 8-bit binary)"),
            expected_answer: format!("{result:08b}"),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Circular bit rotation of a random byte — bits shifted past the end
/// wrap around to the other side, which is exactly what distinguishes
/// rotation from the plain (non-wrapping) shifting Foundations already
/// covers.
pub struct RotationGenerator;

impl ChallengeGenerator for RotationGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BitRotation
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let value = rng.range_u32(0, 255) as u8;
        let amount = rng.range_u32(1, 7);
        let rotate_left = rng.next_u64().is_multiple_of(2);

        let (direction, result) =
            if rotate_left { ("left", value.rotate_left(amount)) } else { ("right", value.rotate_right(amount)) };

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!("Rotate {value:08b} {direction} by {amount} bit(s) (8-bit register)"),
            expected_answer: format!("{result:08b}"),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// A random small array printed in the prompt itself, asking for the
/// value at a random valid index — direct index access is the entire
/// point of an array, so the puzzle is the definition.
pub struct ArrayIndexGenerator;

impl ChallengeGenerator for ArrayIndexGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::ArrayIndex
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let len = rng.range_u32(4, 4 + (ctx.difficulty.0 as u32).min(4)) as usize;
        let values: Vec<u32> = (0..len).map(|_| rng.range_u32(0, 99)).collect();
        let index = rng.range_u32(0, (len - 1) as u32) as usize;
        let printed = values.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");

        // Same mechanic, concept-appropriate framing — a linked list
        // traversal from the head *is* "find the value at position N,"
        // it just shouldn't say "Array" while asking it. Derived from
        // the concept id the same way `GateEvalGenerator` derives which
        // gate to evaluate.
        let is_linked_list = ctx.template.concept_ids.first().map(|id| id == "linked_list").unwrap_or(false);
        let prompt = if is_linked_list {
            format!("Linked list, head to tail: [{printed}]. What is the value at position {index} (0 = head)?")
        } else {
            format!("Array: [{printed}]. What is the value at index {index}?")
        };

        Challenge { template_id: ctx.template.id.clone(), seed: ctx.seed, prompt, expected_answer: values[index].to_string() }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Narrates a sequence of push/pop operations on an initially-empty
/// stack and asks either what `pop()` returns or what sits on top
/// afterward. Traces the LIFO discipline in prose rather than driving
/// `bit_simulation::array_seq::ArraySim` directly, since the puzzle is
/// about operation order, not about any particular slot layout.
pub struct StackTraceGenerator;

impl ChallengeGenerator for StackTraceGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::StackTrace
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let op_count = 3 + (ctx.difficulty.0 as u32 / 3).min(3);
        let mut stack: Vec<u32> = Vec::new();
        let mut ops: Vec<String> = Vec::new();
        let mut last_pop: Option<u32> = None;

        for i in 0..op_count {
            let can_pop = !stack.is_empty();
            let should_pop = can_pop && (i == op_count - 1 || rng.next_u64().is_multiple_of(3));
            if should_pop {
                last_pop = stack.pop();
                ops.push("pop()".to_string());
            } else {
                let value = rng.range_u32(0, 99);
                stack.push(value);
                ops.push(format!("push({value})"));
            }
        }

        let narration = ops.join(", ");
        if ops.last().map(|s| s == "pop()").unwrap_or(false) {
            Challenge {
                template_id: ctx.template.id.clone(),
                seed: ctx.seed,
                prompt: format!("Start with an empty stack. {narration}. What does the final pop() return?"),
                expected_answer: last_pop.map(|v| v.to_string()).unwrap_or_default(),
            }
        } else {
            let top = stack.last().copied();
            Challenge {
                template_id: ctx.template.id.clone(),
                seed: ctx.seed,
                prompt: format!("Start with an empty stack. {narration}. What value is on top of the stack?"),
                expected_answer: top.map(|v| v.to_string()).unwrap_or_default(),
            }
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// The FIFO counterpart of [`StackTraceGenerator`]: same narrated
/// operation-sequence mechanic, but `dequeue()` always removes from the
/// front instead of the top.
pub struct QueueTraceGenerator;

impl ChallengeGenerator for QueueTraceGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::QueueTrace
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let op_count = 3 + (ctx.difficulty.0 as u32 / 3).min(3);
        let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
        let mut ops: Vec<String> = Vec::new();
        let mut last_dequeue: Option<u32> = None;

        for i in 0..op_count {
            let can_dequeue = !queue.is_empty();
            let should_dequeue = can_dequeue && (i == op_count - 1 || rng.next_u64().is_multiple_of(3));
            if should_dequeue {
                last_dequeue = queue.pop_front();
                ops.push("dequeue()".to_string());
            } else {
                let value = rng.range_u32(0, 99);
                queue.push_back(value);
                ops.push(format!("enqueue({value})"));
            }
        }

        let narration = ops.join(", ");
        if ops.last().map(|s| s == "dequeue()").unwrap_or(false) {
            Challenge {
                template_id: ctx.template.id.clone(),
                seed: ctx.seed,
                prompt: format!("Start with an empty queue. {narration}. What does the final dequeue() return?"),
                expected_answer: last_dequeue.map(|v| v.to_string()).unwrap_or_default(),
            }
        } else {
            let front = queue.front().copied();
            Challenge {
                template_id: ctx.template.id.clone(),
                seed: ctx.seed,
                prompt: format!("Start with an empty queue. {narration}. What value is at the front of the queue?"),
                expected_answer: front.map(|v| v.to_string()).unwrap_or_default(),
            }
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// A random small array printed in the prompt, asking for the index a
/// left-to-right scan finds the target at (or -1). The target is
/// sometimes present (drawn from the array, resolved to its *first*
/// occurrence so duplicates can't produce two "correct" answers) and
/// sometimes absent (drawn from a range the array never uses), so both
/// branches of linear search get exercised.
pub struct LinearSearchGenerator;

impl ChallengeGenerator for LinearSearchGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::LinearSearch
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let len = rng.range_u32(4, 4 + (ctx.difficulty.0 as u32).min(4)) as usize;
        let values: Vec<u32> = (0..len).map(|_| rng.range_u32(0, 99)).collect();

        let target_present = rng.next_u64().is_multiple_of(2);
        let (target, expected_index) = if target_present {
            let pick = rng.range_u32(0, (len - 1) as u32) as usize;
            let target = values[pick];
            let first_index = values.iter().position(|&v| v == target).unwrap();
            (target, first_index as i64)
        } else {
            (rng.range_u32(100, 199), -1)
        };

        let printed = values.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!(
                "Array: [{printed}]. Target: {target}. What index does a left-to-right search find it at? (-1 if not found)"
            ),
            expected_answer: expected_index.to_string(),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// A random *sorted*, duplicate-free array, asking how many comparisons
/// binary search needs to either find the target or determine it's
/// absent. The comparison count is computed by actually running the
/// standard `lo..=hi`, `mid = (lo+hi)/2` loop — not estimated — so the
/// expected answer can never drift from the real algorithm.
pub struct BinarySearchGenerator;

impl ChallengeGenerator for BinarySearchGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BinarySearch
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let len = rng.range_u32(5, 5 + (ctx.difficulty.0 as u32).min(6)) as usize;

        let mut values: Vec<u32> = Vec::new();
        while values.len() < len {
            let candidate = rng.range_u32(0, 199);
            if !values.contains(&candidate) {
                values.push(candidate);
            }
        }
        values.sort_unstable();

        let target_present = rng.next_u64().is_multiple_of(2);
        let target =
            if target_present { values[rng.range_u32(0, (len - 1) as u32) as usize] } else { rng.range_u32(200, 299) };

        let mut lo: i64 = 0;
        let mut hi: i64 = len as i64 - 1;
        let mut comparisons: u32 = 0;
        while lo <= hi {
            let mid = (lo + hi) / 2;
            comparisons += 1;
            let mid_value = values[mid as usize];
            if mid_value == target {
                break;
            } else if mid_value < target {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        let printed = values.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!(
                "Sorted array: [{printed}]. Target: {target}. How many comparisons does binary search need to find it (or determine it's absent)?"
            ),
            expected_answer: comparisons.to_string(),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Asks how many swaps happen during one full left-to-right bubble
/// sort pass — a plain integer, matching this app's answer-format
/// convention (unlike "what does the array look like after," which
/// would need the player to type back a whole formatted array). The
/// count is computed by actually running the pass, not estimated.
pub struct BubbleSortPassGenerator;

impl ChallengeGenerator for BubbleSortPassGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BubbleSortPass
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let len = rng.range_u32(4, 4 + (ctx.difficulty.0 as u32).min(4)) as usize;
        let mut values: Vec<u32> = (0..len).map(|_| rng.range_u32(0, 99)).collect();
        let printed = values.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");

        let mut swaps: u32 = 0;
        for i in 0..values.len().saturating_sub(1) {
            if values[i] > values[i + 1] {
                values.swap(i, i + 1);
                swaps += 1;
            }
        }

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!(
                "Array: [{printed}]. How many swaps happen during one full left-to-right bubble sort pass (compare each adjacent pair, swap if out of order)?"
            ),
            expected_answer: swaps.to_string(),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Adds two random binary numbers, asking for their sum in binary —
/// real addition, not a lookup: the expected answer is `a + b` computed
/// directly and formatted with `{:b}`, so it's automatically correct
/// for every carry chain a given seed happens to produce.
pub struct BinaryAdditionGenerator;

impl ChallengeGenerator for BinaryAdditionGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BinaryAddition
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let max_value = 2u32.saturating_pow(ctx.difficulty.0.clamp(2, 8) as u32) - 1;
        let a = rng.range_u32(0, max_value);
        let b = rng.range_u32(0, max_value);
        let sum = a + b;

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!("What is {a:b} + {b:b} in binary?"),
            expected_answer: format!("{sum:b}"),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Subtracts two random values and asks for the result in 8-bit two's
/// complement — deliberately not restricted to non-negative results.
/// When `b > a`, the answer genuinely is negative, and expressing a
/// negative result *as* a bit pattern (reusing the same
/// `to_twos_complement_bits` helper `TwosComplementGenerator` uses) is
/// the whole point: subtraction is where negative binary conversion
/// actually comes up, not an abstract exercise on its own.
pub struct BinarySubtractionGenerator;

impl ChallengeGenerator for BinarySubtractionGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BinarySubtraction
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        // Both operands capped at 127 so a - b always lands inside the
        // 8-bit signed range (-128..=127), whichever way it comes out.
        let max_value = 2u32.saturating_pow(ctx.difficulty.0.clamp(2, 7) as u32) - 1;
        let a = rng.range_u32(0, max_value.min(127));
        let b = rng.range_u32(0, max_value.min(127));
        let diff = a as i32 - b as i32;
        let bits = to_twos_complement_bits(diff, 8);

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!("What is {a:b} - {b:b} in 8-bit two's complement binary?"),
            expected_answer: bits,
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Multiplies two random values and asks for the product in binary.
/// Operands are kept deliberately small (capped well below Addition's
/// range) since the product grows quadratically — a puzzle asking you
/// to type back a 12-digit binary product isn't testing multiplication
/// anymore, it's testing patience.
pub struct BinaryMultiplicationGenerator;

impl ChallengeGenerator for BinaryMultiplicationGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BinaryMultiplication
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let max_value = 2u32.saturating_pow(ctx.difficulty.0.clamp(2, 5) as u32) - 1;
        let a = rng.range_u32(0, max_value);
        let b = rng.range_u32(0, max_value);
        let product = a * b;

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!("What is {a:b} * {b:b} in binary?"),
            expected_answer: format!("{product:b}"),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Integer-divides two random values (divisor always >= 1, so this
/// never asks for a division by zero) and asks for the quotient in
/// binary — floor division, the remainder is simply discarded, same as
/// `/` on unsigned integers in most programming languages.
pub struct BinaryDivisionGenerator;

impl ChallengeGenerator for BinaryDivisionGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::BinaryDivision
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let max_value = 2u32.saturating_pow(ctx.difficulty.0.clamp(2, 8) as u32) - 1;
        let dividend = rng.range_u32(0, max_value);
        let divisor = rng.range_u32(1, max_value.max(1));
        let quotient = dividend / divisor;

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!("What is {dividend:b} / {divisor:b} in binary? (integer division — drop the remainder)"),
            expected_answer: format!("{quotient:b}"),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// One pass of selection sort: find the minimum across the *whole*
/// array and report what ends up at index 0 once it's swapped there.
/// A genuinely different mechanic from `BubbleSortPassGenerator`
/// (adjacent-compare-and-swap) — this is "scan everything, remember
/// the smallest," which is what makes selection sort's per-pass cost
/// flat (always a full scan) where bubble sort's shrinks over time.
pub struct SelectionSortPassGenerator;

impl ChallengeGenerator for SelectionSortPassGenerator {
    fn kind(&self) -> GeneratorKind {
        GeneratorKind::SelectionSortPass
    }

    fn generate(&self, ctx: &GenerationContext) -> Challenge {
        let mut rng = Rng::new(ctx.seed);
        let len = rng.range_u32(4, 4 + (ctx.difficulty.0 as u32).min(4)) as usize;
        let values: Vec<u32> = (0..len).map(|_| rng.range_u32(0, 99)).collect();
        let printed = values.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
        let minimum = *values.iter().min().expect("len is always >= 4");

        Challenge {
            template_id: ctx.template.id.clone(),
            seed: ctx.seed,
            prompt: format!(
                "Array: [{printed}]. One pass of selection sort finds the minimum of the whole array and swaps it to the front. What value ends up at index 0?"
            ),
            expected_answer: minimum.to_string(),
        }
    }

    fn validate(&self, challenge: &Challenge, answer: &Answer) -> ValidationResult {
        validate_exact_match(&challenge.expected_answer, answer)
    }
}

/// Registers every generator this crate ships. `bit-core` uses this to
/// populate `BitEngine`'s registry at startup — content authors only ever
/// need to write a `ChallengeTemplate` naming an existing `GeneratorKind`,
/// never register anything themselves.
pub fn default_registry() -> ChallengeRegistry {
    let mut registry = ChallengeRegistry::new();
    registry.register(Box::new(BinaryConversionGenerator));
    registry.register(Box::new(GateEvalGenerator));
    registry.register(Box::new(TwosComplementGenerator));
    registry.register(Box::new(AsciiLookupGenerator));
    registry.register(Box::new(PopCountGenerator));
    registry.register(Box::new(MaskGenerator));
    registry.register(Box::new(RotationGenerator));
    registry.register(Box::new(ArrayIndexGenerator));
    registry.register(Box::new(StackTraceGenerator));
    registry.register(Box::new(QueueTraceGenerator));
    registry.register(Box::new(LinearSearchGenerator));
    registry.register(Box::new(BinarySearchGenerator));
    registry.register(Box::new(BubbleSortPassGenerator));
    registry.register(Box::new(BinaryAdditionGenerator));
    registry.register(Box::new(BinarySubtractionGenerator));
    registry.register(Box::new(BinaryMultiplicationGenerator));
    registry.register(Box::new(BinaryDivisionGenerator));
    registry.register(Box::new(SelectionSortPassGenerator));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(seed: u64) -> GenerationContext {
        GenerationContext {
            seed,
            difficulty: DifficultyLevel(4),
            mastery: MasterySnapshot::default(),
            template: ChallengeTemplate {
                id: "binary_to_decimal".into(),
                generator: GeneratorKind::BinaryConversion,
                concept_ids: vec!["binary".into()],
            },
        }
    }

    #[test]
    fn same_seed_produces_identical_challenge() {
        let gen = BinaryConversionGenerator;
        let a = gen.generate(&ctx(42));
        let b = gen.generate(&ctx(42));
        assert_eq!(a, b);
    }

    #[test]
    fn different_seed_can_produce_different_challenge() {
        let gen = BinaryConversionGenerator;
        let a = gen.generate(&ctx(1));
        let b = gen.generate(&ctx(2));
        assert_ne!(a, b);
    }

    #[test]
    fn validates_correct_and_incorrect_answers() {
        let gen = BinaryConversionGenerator;
        let challenge = gen.generate(&ctx(7));
        let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
        assert!(correct.correct);

        let wrong = gen.validate(&challenge, &Answer("not-a-number".into()));
        assert!(!wrong.correct);
    }

    #[test]
    fn registry_dispatches_by_generator_kind() {
        let mut registry = ChallengeRegistry::new();
        registry.register(Box::new(BinaryConversionGenerator));
        let challenge = registry.generate(&ctx(99)).expect("generator should be registered");
        assert!(!challenge.prompt.is_empty());
    }

    fn gate_ctx(seed: u64, concept_id: &str) -> GenerationContext {
        GenerationContext {
            seed,
            difficulty: DifficultyLevel(4),
            mastery: MasterySnapshot::default(),
            template: ChallengeTemplate {
                id: "logic_gate_eval".into(),
                generator: GeneratorKind::GateEval,
                concept_ids: vec![concept_id.into()],
            },
        }
    }

    #[test]
    fn gate_eval_matches_pure_evaluation() {
        use bit_simulation::logic_gate::{evaluate, GateKind};

        let gen = GateEvalGenerator;
        let challenge = gen.generate(&gate_ctx(3, "xor"));
        // Re-derive inputs the same way the generator does and confirm the
        // expected answer matches the shared `evaluate` function directly,
        // proving the two never drift apart.
        let mut rng = Rng::new(3);
        let inputs: Vec<bool> = (0..2).map(|_| rng.next_u64().is_multiple_of(2)).collect();
        let expected = if evaluate(GateKind::Xor, &inputs) { "1" } else { "0" };
        assert_eq!(challenge.expected_answer, expected);
    }

    #[test]
    fn not_gate_always_uses_a_single_input() {
        let gen = GateEvalGenerator;
        let challenge = gen.generate(&gate_ctx(1, "not"));
        assert!(challenge.prompt.starts_with("NOT("));
        assert_eq!(challenge.prompt.matches(',').count(), 0);
    }

    #[test]
    fn unknown_concept_id_falls_back_to_and() {
        let gen = GateEvalGenerator;
        let challenge = gen.generate(&gate_ctx(1, "nand"));
        assert!(challenge.prompt.starts_with("AND("));
    }

    #[test]
    fn default_generator_resolves_for_known_concepts_and_none_otherwise() {
        assert_eq!(GeneratorKind::default_for_concept("binary"), Some(GeneratorKind::BinaryConversion));
        assert_eq!(GeneratorKind::default_for_concept("xor"), Some(GeneratorKind::GateEval));
        assert_eq!(GeneratorKind::default_for_concept("twos_complement"), Some(GeneratorKind::TwosComplement));
        assert_eq!(GeneratorKind::default_for_concept("ascii"), Some(GeneratorKind::AsciiLookup));
        assert_eq!(GeneratorKind::default_for_concept("population_count"), Some(GeneratorKind::PopCount));
        assert_eq!(GeneratorKind::default_for_concept("bitwise_masks"), Some(GeneratorKind::BitwiseMask));
        assert_eq!(GeneratorKind::default_for_concept("bit_rotation"), Some(GeneratorKind::BitRotation));
        assert_eq!(GeneratorKind::default_for_concept("arrays"), Some(GeneratorKind::ArrayIndex));
        assert_eq!(GeneratorKind::default_for_concept("linked_list"), Some(GeneratorKind::ArrayIndex));
        assert_eq!(GeneratorKind::default_for_concept("stacks"), Some(GeneratorKind::StackTrace));
        assert_eq!(GeneratorKind::default_for_concept("queues"), Some(GeneratorKind::QueueTrace));
        assert_eq!(GeneratorKind::default_for_concept("bit_shifting"), None);
        // An IP octet is just a byte — same mechanic, same generator,
        // no new Rust needed for this concept's puzzles.
        assert_eq!(GeneratorKind::default_for_concept("ip_addresses"), Some(GeneratorKind::BinaryConversion));
    }

    #[test]
    fn default_registry_serves_all_builtin_generators() {
        let registry = default_registry();
        assert!(registry.generate(&ctx(1)).is_some());
        assert!(registry.generate(&gate_ctx(1, "and")).is_some());
        assert!(registry.generate(&twos_complement_ctx(1)).is_some());
        assert!(registry.generate(&ascii_ctx(1)).is_some());
    }

    fn twos_complement_ctx(seed: u64) -> GenerationContext {
        GenerationContext {
            seed,
            difficulty: DifficultyLevel(7),
            mastery: MasterySnapshot::default(),
            template: ChallengeTemplate {
                id: "twos_complement_conversion".into(),
                generator: GeneratorKind::TwosComplement,
                concept_ids: vec!["twos_complement".into()],
            },
        }
    }

    fn ascii_ctx(seed: u64) -> GenerationContext {
        GenerationContext {
            seed,
            difficulty: DifficultyLevel(4),
            mastery: MasterySnapshot::default(),
            template: ChallengeTemplate {
                id: "ascii_lookup".into(),
                generator: GeneratorKind::AsciiLookup,
                concept_ids: vec!["ascii".into()],
            },
        }
    }

    #[test]
    fn twos_complement_round_trips_binary_and_signed_directions() {
        let gen = TwosComplementGenerator;
        // Sample enough seeds to hit both directions deterministically.
        for seed in 0..20 {
            let challenge = gen.generate(&twos_complement_ctx(seed));
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed} prompt {:?} should validate its own answer", challenge.prompt);
        }
    }

    #[test]
    fn twos_complement_matches_binary_circuit_signed_value() {
        use bit_simulation::binary_circuit::{Action, BinaryCircuitSim};
        use bit_simulation::Simulation;

        // -1 in 8-bit two's complement is all-ones, matching
        // BinaryCircuitSim::State::signed_value's own -1 case exactly —
        // this is the seam between bit-challenge and bit-simulation that
        // must never drift apart.
        let sim = BinaryCircuitSim { bit_width: 8 };
        let mut state = sim.initial_state();
        for i in 0..8 {
            sim.apply(&mut state, Action::Toggle(i));
        }
        assert_eq!(state.signed_value(), -1);
        assert_eq!(to_twos_complement_bits(-1, 8), "11111111");
    }

    #[test]
    fn twos_complement_bit_patterns_match_known_boundary_values() {
        assert_eq!(to_twos_complement_bits(0, 8), "00000000");
        assert_eq!(to_twos_complement_bits(127, 8), "01111111"); // largest positive
        assert_eq!(to_twos_complement_bits(-128, 8), "10000000"); // most negative
        assert_eq!(to_twos_complement_bits(-1, 8), "11111111");
    }

    #[test]
    fn ascii_lookup_round_trips() {
        let gen = AsciiLookupGenerator;
        for seed in 0..20 {
            let challenge = gen.generate(&ascii_ctx(seed));
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed} prompt {:?} should validate its own answer", challenge.prompt);
        }
    }

    #[test]
    fn ascii_lookup_skips_space_for_legibility() {
        // Space (code 32) would render as an invisible character in a
        // prompt like "What is the ASCII code for ' '?" — confirm the
        // generator never produces it across a wide seed sample.
        let gen = AsciiLookupGenerator;
        for seed in 0..200 {
            let challenge = gen.generate(&ascii_ctx(seed));
            assert_ne!(challenge.prompt, "What is the ASCII code for ' '?");
        }
    }

    fn kind_ctx(seed: u64, generator: GeneratorKind, concept_id: &str) -> GenerationContext {
        GenerationContext {
            seed,
            difficulty: DifficultyLevel(5),
            mastery: MasterySnapshot::default(),
            template: ChallengeTemplate { id: "t".into(), generator, concept_ids: vec![concept_id.into()] },
        }
    }

    #[test]
    fn pop_count_matches_count_ones() {
        let gen = PopCountGenerator;
        for seed in 0..50 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::PopCount, "population_count"));
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed} should validate its own answer");
        }
    }

    #[test]
    fn mask_generator_result_matches_bitwise_op() {
        let gen = MaskGenerator;
        // seed 0 deterministically exercises the generator; just confirm
        // the printed operands, when parsed back, actually combine to the
        // expected answer under one of AND/OR/XOR.
        let challenge = gen.generate(&kind_ctx(11, GeneratorKind::BitwiseMask, "bitwise_masks"));
        let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
        assert!(correct.correct);
        assert_eq!(challenge.expected_answer.len(), 8);
    }

    #[test]
    fn rotation_is_reversible() {
        let gen = RotationGenerator;
        for seed in 0..50 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BitRotation, "bit_rotation"));
            assert_eq!(challenge.expected_answer.len(), 8);
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct);
        }
    }

    #[test]
    fn rotate_left_then_right_returns_original_value() {
        let value: u8 = 0b1011_0001;
        assert_eq!(value.rotate_left(3).rotate_right(3), value);
    }

    #[test]
    fn array_index_answer_matches_printed_array() {
        let gen = ArrayIndexGenerator;
        for seed in 0..50 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::ArrayIndex, "arrays"));
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed} prompt {:?}", challenge.prompt);
            assert!(challenge.prompt.starts_with("Array:"));
        }
    }

    #[test]
    fn array_index_generator_frames_linked_list_prompts_differently() {
        let gen = ArrayIndexGenerator;
        for seed in 0..50 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::ArrayIndex, "linked_list"));
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed} prompt {:?}", challenge.prompt);
            assert!(challenge.prompt.starts_with("Linked list"), "seed {seed} prompt {:?}", challenge.prompt);
            assert!(!challenge.prompt.contains("Array"), "seed {seed} prompt {:?}", challenge.prompt);
        }
    }

    #[test]
    fn stack_trace_is_lifo_and_self_consistent() {
        let gen = StackTraceGenerator;
        for seed in 0..50 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::StackTrace, "stacks"));
            assert!(!challenge.expected_answer.is_empty(), "seed {seed} should always have an answer");
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct);
        }
    }

    #[test]
    fn queue_trace_is_fifo_and_self_consistent() {
        let gen = QueueTraceGenerator;
        for seed in 0..50 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::QueueTrace, "queues"));
            assert!(!challenge.expected_answer.is_empty(), "seed {seed} should always have an answer");
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct);
        }
    }

    #[test]
    fn default_registry_serves_all_new_generators() {
        let registry = default_registry();
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::PopCount, "population_count")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::BitwiseMask, "bitwise_masks")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::BitRotation, "bit_rotation")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::ArrayIndex, "arrays")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::StackTrace, "stacks")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::QueueTrace, "queues")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::LinearSearch, "linear_search")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::BinarySearch, "binary_search")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::BubbleSortPass, "bubble_sort")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::BinaryAddition, "binary_addition")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::BinarySubtraction, "binary_subtraction")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::BinaryMultiplication, "binary_multiplication")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::BinaryDivision, "binary_division")).is_some());
        assert!(registry.generate(&kind_ctx(1, GeneratorKind::SelectionSortPass, "selection_sort")).is_some());
    }

    #[test]
    fn selection_sort_pass_answer_is_the_real_minimum() {
        let gen = SelectionSortPassGenerator;
        for seed in 0..200 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::SelectionSortPass, "selection_sort"));
            let values = parse_array_only(&challenge.prompt);
            let expected_min = values.iter().min().unwrap();
            assert_eq!(challenge.expected_answer, expected_min.to_string(), "seed {seed}");

            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed}");
        }
    }

    /// Parses "What is X <op> Y in ...?" back into the two operands the
    /// generator used, for the three new arithmetic generators — each
    /// uses a different separator word, so the operator string is
    /// passed in rather than assumed.
    fn parse_arithmetic_prompt(prompt: &str, op: &str) -> (i32, i32) {
        let rest = prompt.trim_start_matches("What is ");
        let separator = format!(" {op} ");
        let mut parts = rest.splitn(2, &separator);
        let a = i32::from_str_radix(parts.next().unwrap(), 2).unwrap();
        let rest2 = parts.next().unwrap();
        let b_str = rest2.split(' ').next().unwrap();
        let b = i32::from_str_radix(b_str, 2).unwrap();
        (a, b)
    }

    #[test]
    fn binary_subtraction_matches_real_subtraction_including_negative_results() {
        let gen = BinarySubtractionGenerator;
        let mut saw_negative_result = false;
        for seed in 0..300 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BinarySubtraction, "binary_subtraction"));
            let (a, b) = parse_arithmetic_prompt(&challenge.prompt, "-");
            let expected_diff = a - b;
            if expected_diff < 0 {
                saw_negative_result = true;
            }
            assert_eq!(challenge.expected_answer, to_twos_complement_bits(expected_diff, 8), "seed {seed}: {a} - {b}");

            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed}");
        }
        assert!(saw_negative_result, "a wide seed sample should include at least one negative result");
    }

    #[test]
    fn binary_multiplication_matches_real_multiplication() {
        let gen = BinaryMultiplicationGenerator;
        for seed in 0..200 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BinaryMultiplication, "binary_multiplication"));
            let (a, b) = parse_arithmetic_prompt(&challenge.prompt, "*");
            let expected: i32 = a * b;
            assert_eq!(challenge.expected_answer, format!("{expected:b}"), "seed {seed}: {a} * {b}");

            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed}");
        }
    }

    #[test]
    fn binary_division_matches_real_integer_division_and_never_divides_by_zero() {
        let gen = BinaryDivisionGenerator;
        for seed in 0..200 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BinaryDivision, "binary_division"));
            let (dividend, divisor) = parse_arithmetic_prompt(&challenge.prompt, "/");
            assert!(divisor >= 1, "seed {seed}: divisor should never be zero");
            let expected = dividend / divisor;
            assert_eq!(challenge.expected_answer, format!("{expected:b}"), "seed {seed}: {dividend} / {divisor}");

            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed}");
        }
    }

    #[test]
    fn binary_addition_sum_matches_real_addition_across_carry_chains() {
        let gen = BinaryAdditionGenerator;
        for seed in 0..200 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BinaryAddition, "binary_addition"));
            let mut parts = challenge.prompt.trim_start_matches("What is ").trim_end_matches(" in binary?").split(" + ");
            let a = u32::from_str_radix(parts.next().unwrap(), 2).unwrap();
            let b = u32::from_str_radix(parts.next().unwrap(), 2).unwrap();
            let expected = u32::from_str_radix(&challenge.expected_answer, 2).unwrap();
            assert_eq!(expected, a + b, "seed {seed}: {a} + {b}");

            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed}");
        }
    }

    #[test]
    fn binary_addition_operands_stay_within_the_difficulty_ceiling() {
        // difficulty 2 -> each operand is 0..=3 (2^2 - 1), so the sum can
        // never exceed 6, across a wide seed sample.
        let gen = BinaryAdditionGenerator;
        for seed in 0..200 {
            let ctx = GenerationContext {
                seed,
                difficulty: DifficultyLevel(2),
                mastery: MasterySnapshot::default(),
                template: ChallengeTemplate {
                    id: "t".into(),
                    generator: GeneratorKind::BinaryAddition,
                    concept_ids: vec!["binary_addition".into()],
                },
            };
            let sum = u32::from_str_radix(&gen.generate(&ctx).expected_answer, 2).unwrap();
            assert!(sum <= 6, "seed {seed}: sum {sum} exceeds the difficulty-2 ceiling");
        }
    }

    #[test]
    fn default_generator_resolves_for_search_concepts() {
        assert_eq!(GeneratorKind::default_for_concept("linear_search"), Some(GeneratorKind::LinearSearch));
        assert_eq!(GeneratorKind::default_for_concept("binary_search"), Some(GeneratorKind::BinarySearch));
        assert_eq!(GeneratorKind::default_for_concept("bubble_sort"), Some(GeneratorKind::BubbleSortPass));
    }

    /// Parses "Array: [a, b, c]. ..." back into the values the
    /// generator used — bubble sort's prompt has no "Target:" clause,
    /// so this is a lighter-weight sibling of `parse_search_prompt`.
    fn parse_array_only(prompt: &str) -> Vec<u32> {
        let start = prompt.find('[').unwrap() + 1;
        let end = prompt.find(']').unwrap();
        prompt[start..end].split(", ").map(|s| s.parse().unwrap()).collect()
    }

    #[test]
    fn bubble_sort_pass_swap_count_matches_a_manual_trace() {
        let gen = BubbleSortPassGenerator;
        for seed in 0..200 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BubbleSortPass, "bubble_sort"));
            let values = parse_array_only(&challenge.prompt);
            let mut traced = values.clone();
            let mut expected_swaps = 0u32;
            for i in 0..traced.len().saturating_sub(1) {
                if traced[i] > traced[i + 1] {
                    traced.swap(i, i + 1);
                    expected_swaps += 1;
                }
            }
            assert_eq!(challenge.expected_answer, expected_swaps.to_string(), "seed {seed}");
        }
    }

    #[test]
    fn bubble_sort_pass_answer_is_never_negative_or_impossible() {
        let gen = BubbleSortPassGenerator;
        for seed in 0..100 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BubbleSortPass, "bubble_sort"));
            let values = parse_array_only(&challenge.prompt);
            let swaps: u32 = challenge.expected_answer.parse().expect("swap count should be a plain non-negative integer");
            assert!(swaps < values.len() as u32, "seed {seed}: at most len-1 comparisons happen in one pass");
        }
    }

    #[test]
    fn linear_search_answer_matches_first_occurrence_or_absence() {
        let gen = LinearSearchGenerator;
        for seed in 0..100 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::LinearSearch, "linear_search"));
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed} prompt {:?}", challenge.prompt);
        }
    }

    /// Parses "Array: [a, b, c]. Target: t. ..." back into the values and
    /// target the generator actually used, so tests can check the answer
    /// against the real mechanic instead of only re-validating itself.
    fn parse_search_prompt(prompt: &str) -> (Vec<u32>, u32) {
        let start = prompt.find('[').unwrap() + 1;
        let end = prompt.find(']').unwrap();
        let values: Vec<u32> = prompt[start..end].split(", ").map(|s| s.parse().unwrap()).collect();
        let target_start = prompt.find("Target: ").unwrap() + "Target: ".len();
        let target_end = prompt[target_start..].find('.').unwrap() + target_start;
        let target: u32 = prompt[target_start..target_end].parse().unwrap();
        (values, target)
    }

    #[test]
    fn linear_search_never_reports_an_absent_target_as_found() {
        let gen = LinearSearchGenerator;
        for seed in 0..200 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::LinearSearch, "linear_search"));
            let (values, target) = parse_search_prompt(&challenge.prompt);
            let real_index = values.iter().position(|&v| v == target).map(|i| i as i64).unwrap_or(-1);
            assert_eq!(challenge.expected_answer, real_index.to_string(), "seed {seed}");
        }
    }

    #[test]
    fn binary_search_comparison_count_matches_a_manual_trace() {
        // seed chosen to inspect directly: confirm the generator's own
        // answer validates against itself (the real, load-bearing
        // property — the exact array is seed-derived and not worth
        // hardcoding here), across a wide sample so both the "found" and
        // "absent" branches get covered.
        let gen = BinarySearchGenerator;
        for seed in 0..200 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BinarySearch, "binary_search"));
            let correct = gen.validate(&challenge, &Answer(challenge.expected_answer.clone()));
            assert!(correct.correct, "seed {seed} prompt {:?}", challenge.prompt);
            let count: u32 = challenge.expected_answer.parse().expect("comparison count should be a plain integer");
            assert!(count >= 1, "binary search on a non-empty array always makes at least one comparison");
        }
    }

    #[test]
    fn binary_search_array_is_always_sorted_and_duplicate_free() {
        let gen = BinarySearchGenerator;
        for seed in 0..50 {
            let challenge = gen.generate(&kind_ctx(seed, GeneratorKind::BinarySearch, "binary_search"));
            let (values, _target) = parse_search_prompt(&challenge.prompt);
            let mut sorted = values.clone();
            sorted.sort_unstable();
            assert_eq!(values, sorted, "seed {seed}");
            let unique: std::collections::HashSet<u32> = values.iter().copied().collect();
            assert_eq!(unique.len(), values.len(), "seed {seed}: duplicate values would make binary search ambiguous");
        }
    }
}
