//! `lattice-diagnosis` — AI misconception diagnosis (Phase 2, spec §2.4, §7).
//!
//! Given a student's *submitted work* (not just the answer), classify the
//! specific misconception behind a wrong attempt and map it to a concept —
//! reading the work is what lets it say "you used `(x−1)²` instead of
//! difference-of-squares", which a deterministic tag system can't.
//!
//! The LLM call (BYOK: Anthropic/OpenAI/Gemini) goes through [`lattice_llm`];
//! this crate owns the prompt and the structured-JSON parse (spec §7).

use serde::{Deserialize, Serialize};

// Re-exported so downstream crates keep a single import surface for the provider.
pub use lattice_llm::{Provider, ProviderConfig};

/// Everything the model needs to diagnose one wrong attempt.
#[derive(Debug, Clone)]
pub struct DiagnosisRequest<'a> {
    pub problem_content: &'a str,
    pub solution: &'a str,
    pub submitted_work: &'a str,
    pub concept_label: &'a str,
    pub concept_id: &'a str,
}

/// Structured diagnosis — the shape the model is asked to return (spec §7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisResult {
    pub diagnosed_concept: String,
    pub misconception_label: String,
    pub explanation: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_confidence() -> f32 {
    0.5
}

#[derive(Debug, thiserror::Error)]
pub enum DiagnosisError {
    #[error(transparent)]
    Llm(#[from] lattice_llm::LlmError),
    #[error("could not parse the model's response as JSON: {0}")]
    Parse(String),
}

const SYSTEM_PROMPT: &str = "\
You are a precise math tutor diagnosing a single student mistake. You are given a \
problem, its correct solution, and the student's submitted work. Identify the ONE \
specific misconception behind the error — not a generic 'they're weak at X', but \
the identifiable confusion (e.g. confusing the difference-of-squares identity with \
squaring a binomial).

Respond with ONLY a JSON object, no prose before or after, of exactly this shape:
{
  \"diagnosed_concept\": \"<a short concept slug the error maps to>\",
  \"misconception_label\": \"<a short label, <= 8 words>\",
  \"explanation\": \"<1-3 sentences addressed to the student>\",
  \"confidence\": <number between 0 and 1>
}";

fn build_user_prompt(request: &DiagnosisRequest) -> String {
    format!(
        "Concept being practiced: {concept} (id: {concept_id})\n\n\
         Problem (LaTeX):\n{problem}\n\n\
         Correct solution:\n{solution}\n\n\
         Student's submitted work:\n{work}\n\n\
         Diagnose the specific misconception.",
        concept = request.concept_label,
        concept_id = request.concept_id,
        problem = request.problem_content,
        solution = request.solution,
        work = request.submitted_work,
    )
}

/// Diagnose one wrong attempt by calling the configured provider.
pub async fn diagnose(
    config: &ProviderConfig,
    request: &DiagnosisRequest<'_>,
) -> Result<DiagnosisResult, DiagnosisError> {
    let user = build_user_prompt(request);
    let raw = lattice_llm::complete(config, SYSTEM_PROMPT, &user).await?;
    parse_result(&raw)
}

fn parse_result(raw: &str) -> Result<DiagnosisResult, DiagnosisError> {
    let json = lattice_llm::extract_json_object(raw)
        .ok_or_else(|| DiagnosisError::Parse(format!("no JSON object in response: {raw}")))?;
    serde_json::from_str(json).map_err(|e| DiagnosisError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_clean_object() {
        let raw = r#"{"diagnosed_concept":"difference_of_squares","misconception_label":"squared the binomial","explanation":"You wrote (x-1)^2 instead of (x-1)(x+1).","confidence":0.9}"#;
        let r = parse_result(raw).unwrap();
        assert_eq!(r.diagnosed_concept, "difference_of_squares");
        assert!((r.confidence - 0.9).abs() < 1e-6);
    }

    #[test]
    fn extracts_object_from_surrounding_prose() {
        let raw = "Here is the diagnosis:\n{\"diagnosed_concept\":\"factoring\",\"misconception_label\":\"sign error\",\"explanation\":\"Watch the signs.\"}\nHope that helps!";
        let r = parse_result(raw).unwrap();
        assert_eq!(r.diagnosed_concept, "factoring");
        assert!((r.confidence - 0.5).abs() < 1e-6, "confidence defaults to 0.5");
    }

    #[test]
    fn errors_on_non_json() {
        assert!(parse_result("I cannot help with that.").is_err());
    }
}
