//! AI problem generation (Phase 3, spec §2.3).
//!
//! Templates give number-variety; this gives *form*-variety — unlimited, varied
//! problems (including word problems) for any concept and difficulty. The catch
//! with AI-generated math is a silently-wrong answer, so every generated problem
//! is **verified by an independent re-solve**: the model authors a problem +
//! answer, then a second call solves the problem from scratch, and we only keep
//! it if the two answers agree ([`lattice_core::answers_match`]). Deterministic
//! templates remain the default; this is the layered enhancement.

use lattice_core::{
    answers_match, ConceptId, Difficulty, Problem, ProblemId, ProblemSource, SubjectId,
};
use lattice_llm::{complete, extract_json_object, ProviderConfig};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum GenError {
    #[error(transparent)]
    Llm(#[from] lattice_llm::LlmError),
    #[error("could not parse the generated problem: {0}")]
    Parse(String),
    #[error("the model couldn't produce a problem it solves consistently (after {0} tries)")]
    Unverified(u32),
}

#[derive(Deserialize)]
struct Generated {
    content: String,
    solution: String,
}

const TRIES: u32 = 3;

/// Generate one AI problem for `concept`, verified by an independent re-solve.
pub async fn generate_problem(
    config: &ProviderConfig,
    subject_id: &SubjectId,
    concept_id: &ConceptId,
    concept_label: &str,
    difficulty: Difficulty,
) -> Result<Problem, GenError> {
    for _ in 0..TRIES {
        let candidate = author(config, concept_label, difficulty).await?;
        let check = solve(config, &candidate.content).await?;
        // Keep it only if an independent solve agrees with the stated answer.
        if answers_match(&candidate.solution, &check) {
            return Ok(Problem {
                id: ProblemId::new(),
                subject_id: subject_id.clone(),
                concepts: vec![concept_id.clone()],
                difficulty,
                content: candidate.content,
                solution: candidate.solution,
                generated_by: ProblemSource::Ai,
            });
        }
    }
    Err(GenError::Unverified(TRIES))
}

fn difficulty_word(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
    }
}

async fn author(
    config: &ProviderConfig,
    concept_label: &str,
    difficulty: Difficulty,
) -> Result<Generated, GenError> {
    let system = "You are a math problem author for a practice app. Produce ONE problem \
        with a single, unambiguous, concise final answer (a number, fraction, short \
        expression, or comma-separated list — avoid answers whose form is ambiguous). \
        The 'content' must be a KaTeX-renderable string; use \\text{...} for any words. \
        Return ONLY a JSON object: \
        {\"content\": \"<problem in KaTeX>\", \"solution\": \"<the final answer only>\"}.";
    let user = format!(
        "Concept: {concept}. Write one {diff} problem on this concept.",
        concept = concept_label,
        diff = difficulty_word(difficulty),
    );
    let raw = complete(config, system, &user).await?;
    let json = extract_json_object(&raw)
        .ok_or_else(|| GenError::Parse(format!("no JSON object in response: {raw}")))?;
    serde_json::from_str(json).map_err(|e| GenError::Parse(e.to_string()))
}

async fn solve(config: &ProviderConfig, content: &str) -> Result<String, GenError> {
    let system = "You are solving a math problem. Reply with ONLY the final answer — \
        no working, no prose.";
    Ok(complete(config, system, content).await?)
}
