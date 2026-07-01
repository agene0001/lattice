//! `lattice-core` — the domain vocabulary for the whole workspace.
//!
//! Pure data types: no I/O, no transport concerns (spec §5). Every other crate
//! — the graph engine, content generation, storage, the Tauri service — is
//! defined in terms of what lives here. Keeping this crate dependency-light and
//! side-effect-free is what lets `lattice-service` stay transport-agnostic
//! (spec §9) and what lets subjects be expressed as *data* rather than code
//! (Pillar 6, spec §2.6).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod units;

/// Human-readable identifier for a concept node, e.g. `"difference_of_squares"`.
///
/// Concepts are hand-authored in subject data files, so their ids are stable
/// slugs rather than UUIDs — this keeps the JSON/YAML graph readable and
/// diffable (spec §5, `subjects/math/`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConceptId(pub String);

impl ConceptId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ConceptId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for ConceptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identifier for a subject, e.g. `"math"`. A real type from day one even with a
/// single subject populated, so going multi-subject (Pillar 6) is a data change,
/// not a schema migration (spec §9).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SubjectId(pub String);

impl SubjectId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SubjectId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for SubjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Generates a UUID-backed newtype id with the usual conveniences. These ids are
/// minted by the system (problems, attempts, …) rather than authored by hand, so
/// a random UUID is the right shape — unlike [`ConceptId`]/[`SubjectId`].
macro_rules! uuid_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Mint a fresh random id.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

uuid_id!(
    /// Identifies a single learner. Real type from day one even with one learner
    /// (you, dogfooding) — see spec §9.
    LearnerId
);
uuid_id!(
    /// Identifies a problem, whether template-generated or AI-generated.
    ProblemId
);
uuid_id!(
    /// Identifies one submitted attempt at a problem.
    AttemptId
);
uuid_id!(
    /// Identifies one AI misconception diagnosis of an attempt (Phase 2).
    DiagnosisId
);

/// Mastery is not binary — it's a spectrum that degrades over time (spec §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MasteryState {
    Mastered,
    Familiar,
    Rusty,
    Forgotten,
}

/// Difficulty band for a problem. Used to keep template generation inside a
/// requested band (spec §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

/// How a problem was produced — mirrors `problems.generated_by` in the schema
/// (spec §6). `Template` is the only V1 source; `Ai` arrives in Phase 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemSource {
    Template,
    Ai,
    /// A hand-authored or curated problem served verbatim from a subject's
    /// `problems.json` — no generator, no solver. The data path for dropping in
    /// problems from your own notes or openly-licensed sets (spec §2.6).
    Static,
}

/// Where a piece of content came from, kept so adapted openly-licensed material
/// (e.g. OpenStax CC-BY, MIT OCW CC BY-NC-SA) carries its required attribution.
/// Applies to lessons (via Markdown frontmatter) and static problems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribution {
    /// Human-readable origin, e.g. `"MIT OCW 18.01"` or `"OpenStax Calculus Vol 1"`.
    pub source: String,
    /// License identifier, e.g. `"CC BY-NC-SA 4.0"`. Optional but strongly
    /// encouraged for anything adapted from an external source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// A node in the prerequisite DAG (spec §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Concept {
    pub id: ConceptId,
    pub subject_id: SubjectId,
    pub label: String,
    /// Curriculum grouping for display, e.g. `"Linear Algebra"`. Presentation
    /// only — the prerequisite DAG, not the group, defines learning order.
    pub group: String,
    /// Optional teaching notes (Markdown + KaTeX), loaded from
    /// `subjects/<id>/notes/<concept>.md` when present. The "learn the concept"
    /// content shown before/alongside practice.
    #[serde(default)]
    pub notes: Option<String>,
    /// Direct prerequisites *within this subject* — the transitive closure is
    /// computed by `lattice-graph`, not stored here.
    pub prerequisites: Vec<ConceptId>,
    /// Prerequisites in *other* subjects (e.g. a Physics concept building on a
    /// Math calculus node). Kept separate from [`prerequisites`](Self::prerequisites)
    /// so the per-subject DAG stays local and unchanged; cross-subject edges are
    /// an additive concern resolved at the orchestration layer.
    #[serde(default)]
    pub external_prerequisites: Vec<ConceptRef>,
}

/// A reference to a concept in a specific subject — how a cross-subject
/// prerequisite is expressed (authored as `"subject:concept"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptRef {
    pub subject: SubjectId,
    pub concept: ConceptId,
}

/// A learner's mastery of one concept at a point in time (spec §5).
///
/// `confidence` and `last_practiced_at` together with `decay_rate` are what the
/// mastery model in `lattice-graph` turns into a *current estimated* mastery —
/// the stored `state` is the last observed label, not the live value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConceptMastery {
    pub concept_id: ConceptId,
    pub state: MasteryState,
    /// Mastery at `last_practiced_at`, in `[0.0, 1.0]`.
    pub confidence: f32,
    pub last_practiced_at: DateTime<Utc>,
    /// Per-day forgetting rate. Fixed per concept-type in V1; a candidate to be
    /// learned from data later (spec open Q4).
    pub decay_rate: f32,
}

/// A problem, template- or AI-generated (spec §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    pub id: ProblemId,
    pub subject_id: SubjectId,
    /// Many-to-many concept tagging — a problem can exercise several concepts.
    pub concepts: Vec<ConceptId>,
    pub difficulty: Difficulty,
    /// LaTeX or plain text.
    pub content: String,
    pub solution: String,
    pub generated_by: ProblemSource,
    /// Attribution for the source material, when adapted from an external set.
    /// Present only on static problems and only at serve time — it's display
    /// metadata, not persisted (grading re-fetches by id without needing it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<Attribution>,
}

/// A learner's submitted attempt (spec §5).
///
/// `submitted_work` is the full worked solution, not just a final answer — this
/// is the load-bearing design constraint for Pillar 4 (spec §2.4), captured from
/// V1 so the diagnosis pillar has history to work with when it ships (§6, §9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    pub id: AttemptId,
    pub learner_id: LearnerId,
    pub problem_id: ProblemId,
    pub submitted_work: String,
    pub is_correct: bool,
    pub created_at: DateTime<Utc>,
}

/// An AI misconception diagnosis of a wrong attempt (Phase 2, spec §5, §7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnosis {
    pub id: DiagnosisId,
    pub attempt_id: AttemptId,
    pub diagnosed_concept: ConceptId,
    pub misconception_label: String,
    pub explanation: String,
    /// The model's self-reported confidence in `[0, 1]` (spec §7).
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
}

/// Whether `candidate` answers a problem whose expected answer is `expected`.
///
/// Numeric/fraction answers compare by value (`1/2` = `2/4` = `0.5`, and
/// `x = 3` = `3`); physical quantities compare by value **and** dimension
/// (`9.8 m/s^2` = `9.80 m/s²`, but `5 J` ≠ `5 N`); everything else falls back to
/// a normalized substring match. Shared by grading (`lattice-service`) and
/// AI-generation verification (`lattice-content`) so both agree on what
/// "equivalent" means (spec open Q6).
pub fn answers_match(expected: &str, candidate: &str) -> bool {
    let expected_n = normalize_answer(expected);
    if expected_n.is_empty() {
        return false;
    }
    // Units-aware first: when *both* sides carry a recognizable unit, compare as
    // physical quantities. Unitless answers return `None` here and fall through
    // to the numeric/substring logic below, so math grading is unchanged.
    let expected_tail = expected.rsplit('=').next().unwrap_or(expected);
    let candidate_tail = candidate
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(candidate);
    if let (Some(want), Some(got)) = (
        units::parse_quantity(expected_tail),
        units::parse_quantity(candidate_tail),
    ) {
        return units::quantities_match(&want, &got, units::DEFAULT_REL_TOL);
    }

    if let Some(want) = to_number(after_last_eq(&expected_n)) {
        // Numeric: compare the candidate's final number, not a substring.
        let last_line = candidate
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or(candidate);
        let got = last_number(&normalize_answer(last_line))
            .or_else(|| last_number(&normalize_answer(candidate)));
        return matches!(got, Some(g) if (g - want).abs() < 1e-9);
    }
    normalize_answer(candidate).contains(&expected_n)
}

fn normalize_answer(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '\\' | '{' | '}' | '$'))
        .collect::<String>()
        .to_lowercase()
}

/// The part after the last `=` (so `x=3` → `3`); the whole string if there's none.
fn after_last_eq(s: &str) -> &str {
    s.rsplit('=').next().unwrap_or(s)
}

/// Parse a plain number or a `p/q` fraction into an `f64`.
fn to_number(s: &str) -> Option<f64> {
    if let Some((p, q)) = s.split_once('/') {
        let p: f64 = p.parse().ok()?;
        let q: f64 = q.parse().ok()?;
        return (q != 0.0).then_some(p / q);
    }
    s.parse::<f64>().ok()
}

/// The right-most number-or-fraction token in `s` (already normalized).
fn last_number(s: &str) -> Option<f64> {
    let is_num = |c: char| c.is_ascii_digit() || matches!(c, '.' | '/' | '-');
    let chars: Vec<char> = s.chars().collect();
    let mut end = chars.len();
    while end > 0 && !is_num(chars[end - 1]) {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0 && is_num(chars[start - 1]) {
        start -= 1;
    }
    to_number(&chars[start..end].iter().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::answers_match;

    #[test]
    fn numeric_and_fraction_equivalence() {
        assert!(answers_match("x = 3", "3"));
        assert!(answers_match("x = 3", "2x = 6\nx = 3"));
        assert!(answers_match("1/2", "2/4"));
        assert!(answers_match("1/2", "0.5"));
        assert!(!answers_match("7", "70"));
        assert!(!answers_match("3", "the answer is 4"));
    }

    #[test]
    fn structural_substring_fallback() {
        assert!(answers_match("(x - 1)(x + 1)", "(x-1)(x+1)"));
        assert!(answers_match("3, 5", "(3, 5)"));
        assert!(!answers_match("2x + 3", "2x + 5"));
    }

    #[test]
    fn physical_quantities_compare_by_value_and_dimension() {
        // Formatting / sig-figs / unicode don't matter; dimension does.
        assert!(answers_match("9.8 m/s^2", "9.80 m/s²"));
        assert!(answers_match("v = 20 m/s", "the speed is 20 m/s"));
        assert!(answers_match("1 km", "1000 m"));
        assert!(!answers_match("5 J", "5 N")); // right number, wrong dimension
        assert!(!answers_match("9.8 m/s^2", "5 m/s^2")); // wrong value
        // A unitless answer still uses the numeric path unchanged.
        assert!(answers_match("x = 3", "3"));
    }
}
