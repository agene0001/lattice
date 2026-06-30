//! `lattice-import` — the content import pipeline (spec §10).
//!
//! The core reframe (§10.1): the LLM is a **structurer/tagger, not an author**.
//! We ingest problems that already exist, then use the model to map each onto the
//! *existing* concept graph and a difficulty — a closed-vocabulary classification
//! task, reliable in a way cold generation is not. The pipeline is:
//!
//! ```text
//! RawProblem (from a source adapter)
//!   → tag:    structure into concepts (from the graph) + difficulty, or REJECT as unmapped
//!   → verify: independent LLM re-solve must agree with the stated answer
//!   → emit:   a `problems.json` entry carrying source + license provenance
//! ```
//!
//! V1 deliberately sinks into `subjects/<id>/problems.json` — the static-problem
//! format the app already serves — so imported problems flow straight into the
//! practice pool with attribution, without a new storage schema. A persisted
//! "problem bank" (spec §10.7) is the scale-up once a JSON file stops being
//! comfortable.
//!
//! Verification here reuses the proven generation pattern (re-solve, then
//! [`lattice_core::answers_match`]) rather than a CAS; SymPy-grade symbolic
//! checking (§10.4) is the stronger deterministic follow-up.

pub mod pipeline;
pub mod source;
pub mod tag;
pub mod verify;

pub use pipeline::{merge_into_problems_json, run_import, ImportReport, ImportedProblem};
pub use source::{License, MathDatasetSource, ProblemSource, RawProblem};
pub use tag::{difficulty_from_hint_tags, structure_and_tag, ConceptVocab, TagOutcome, TaggedProblem};
pub use verify::verify_solution;

/// Errors surfaced by the import pipeline.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("io error on {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parsing {what}: {source}")]
    Parse {
        what: String,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Llm(#[from] lattice_llm::LlmError),
    #[error("dataset error: {0}")]
    Dataset(String),
}
