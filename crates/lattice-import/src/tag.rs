//! Structuring/tagging (spec §10.1): map a [`RawProblem`](crate::RawProblem)
//! onto the *existing* concept graph and a difficulty.
//!
//! This is the load-bearing reframe: a closed-vocabulary classification against a
//! fixed concept list, **with an explicit escape** — if nothing in the graph
//! fits (the dataset covers topics the subject doesn't, e.g. competition geometry
//! against an ML-math graph), the tagger returns [`TagOutcome::Unmapped`] and the
//! problem is skipped rather than force-fit onto the nearest node.

use std::collections::HashSet;

use lattice_core::{ConceptId, Difficulty};
use lattice_llm::{complete, extract_json_object, ProviderConfig};
use serde::Deserialize;

use crate::ImportError;

/// The allowed concepts to tag against — the subject's graph nodes.
pub struct ConceptVocab {
    entries: Vec<(ConceptId, String)>, // (id, label)
    ids: HashSet<ConceptId>,
}

impl ConceptVocab {
    pub fn new(entries: impl IntoIterator<Item = (ConceptId, String)>) -> Self {
        let entries: Vec<_> = entries.into_iter().collect();
        let ids = entries.iter().map(|(id, _)| id.clone()).collect();
        Self { entries, ids }
    }

    fn contains(&self, id: &ConceptId) -> bool {
        self.ids.contains(id)
    }

    /// `id — label` lines for the prompt, the exact vocabulary the model may use.
    fn prompt_list(&self) -> String {
        self.entries
            .iter()
            .map(|(id, label)| format!("{id} — {label}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A problem successfully mapped onto the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedProblem {
    /// 1–3 concepts the problem exercises, all guaranteed to be real graph ids.
    pub concepts: Vec<ConceptId>,
    pub difficulty: Difficulty,
}

/// The result of tagging: either mapped onto the graph, or rejected as out of
/// scope for this subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagOutcome {
    Tagged(TaggedProblem),
    Unmapped { reason: String },
}

/// Tag one problem against `vocab` using the BYOK provider. `hint_tags` (e.g.
/// MATH's `"Level 3"`) refine difficulty without overriding the model's concept
/// judgement.
pub async fn structure_and_tag(
    config: &ProviderConfig,
    content: &str,
    hint_tags: &[String],
    vocab: &ConceptVocab,
) -> Result<TagOutcome, ImportError> {
    let system = "You TAG an existing math problem against a FIXED concept list \
        for a tutoring app. You do NOT solve, rewrite, or invent anything. Return \
        ONLY a JSON object: {\"in_scope\": <bool>, \"concepts\": [\"<id>\", ...], \
        \"difficulty\": \"easy\"|\"medium\"|\"hard\"}. Rules: every id in \
        \"concepts\" MUST appear VERBATIM in the allowed list; pick the 1–3 \
        concepts the problem most directly exercises. If NONE of the allowed \
        concepts genuinely fit, set \"in_scope\": false and \"concepts\": []. \
        Never invent or approximate a concept id.";
    let user = format!(
        "Allowed concepts (id — label):\n{}\n\nProblem:\n{}",
        vocab.prompt_list(),
        content
    );
    let raw = complete(config, system, &user).await?;
    let json = extract_json_object(&raw)
        .ok_or_else(|| ImportError::Dataset(format!("tagger returned no JSON object: {raw}")))?;
    let mut outcome = parse_tag_response(json, vocab)?;

    // Prefer a difficulty derived from a reliable source label (MATH's level)
    // over the model's guess, when one is present.
    if let TagOutcome::Tagged(ref mut tagged) = outcome {
        if let Some(d) = difficulty_from_hint_tags(hint_tags) {
            tagged.difficulty = d;
        }
    }
    Ok(outcome)
}

#[derive(Deserialize)]
struct TagResponse {
    #[serde(default = "default_true")]
    in_scope: bool,
    #[serde(default)]
    concepts: Vec<String>,
    #[serde(default)]
    difficulty: String,
}

fn default_true() -> bool {
    true
}

/// Parse + **constrain** the tagger's JSON: drop any id not in the vocabulary,
/// and reject (as [`TagOutcome::Unmapped`]) when the model says out-of-scope or
/// when nothing maps. Pure — this is the part the tests pin down.
pub fn parse_tag_response(json: &str, vocab: &ConceptVocab) -> Result<TagOutcome, ImportError> {
    let resp: TagResponse = serde_json::from_str(json).map_err(|source| ImportError::Parse {
        what: "tagger response".to_string(),
        source,
    })?;

    if !resp.in_scope {
        return Ok(TagOutcome::Unmapped {
            reason: "model judged the problem out of scope for this subject".to_string(),
        });
    }

    // Keep only real graph ids, preserving order and de-duplicating.
    let mut concepts: Vec<ConceptId> = Vec::new();
    for raw_id in resp.concepts {
        let id = ConceptId::new(raw_id);
        if vocab.contains(&id) && !concepts.contains(&id) {
            concepts.push(id);
        }
    }
    if concepts.is_empty() {
        return Ok(TagOutcome::Unmapped {
            reason: "no tagged concept matched the graph".to_string(),
        });
    }
    concepts.truncate(3);

    Ok(TagOutcome::Tagged(TaggedProblem {
        difficulty: parse_difficulty(&resp.difficulty),
        concepts,
    }))
}

fn parse_difficulty(s: &str) -> Difficulty {
    match s.trim().to_ascii_lowercase().as_str() {
        "easy" => Difficulty::Easy,
        "hard" => Difficulty::Hard,
        _ => Difficulty::Medium,
    }
}

/// Map a source's own difficulty label onto Lattice's three tiers. Recognizes the
/// MATH dataset's `"Level 1".."Level 5"`: 1–2 → easy, 3 → medium, 4–5 → hard.
pub fn difficulty_from_hint_tags(hint_tags: &[String]) -> Option<Difficulty> {
    for tag in hint_tags {
        let lower = tag.to_ascii_lowercase();
        if let Some(n) = lower.strip_prefix("level ").and_then(|n| n.trim().parse::<u32>().ok()) {
            return Some(match n {
                0..=2 => Difficulty::Easy,
                3 => Difficulty::Medium,
                _ => Difficulty::Hard,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vocab() -> ConceptVocab {
        ConceptVocab::new([
            (ConceptId::new("gradients"), "Gradients".to_string()),
            (ConceptId::new("derivatives"), "Derivatives".to_string()),
        ])
    }

    #[test]
    fn keeps_only_real_ids_and_caps_at_three() {
        let json = r#"{"in_scope":true,"concepts":["derivatives","gradients","not_a_concept"],"difficulty":"hard"}"#;
        let out = parse_tag_response(json, &vocab()).unwrap();
        match out {
            TagOutcome::Tagged(t) => {
                assert_eq!(t.concepts, vec![ConceptId::new("derivatives"), ConceptId::new("gradients")]);
                assert_eq!(t.difficulty, Difficulty::Hard);
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
    }

    #[test]
    fn out_of_scope_is_rejected_not_forced() {
        let json = r#"{"in_scope":false,"concepts":[],"difficulty":"medium"}"#;
        assert!(matches!(parse_tag_response(json, &vocab()).unwrap(), TagOutcome::Unmapped { .. }));
    }

    #[test]
    fn all_invalid_ids_means_unmapped() {
        // The model hallucinated ids that aren't in the graph — don't force-fit.
        let json = r#"{"in_scope":true,"concepts":["geometry","number_theory"],"difficulty":"medium"}"#;
        assert!(matches!(parse_tag_response(json, &vocab()).unwrap(), TagOutcome::Unmapped { .. }));
    }

    #[test]
    fn level_label_overrides_to_difficulty() {
        assert_eq!(difficulty_from_hint_tags(&["Algebra".into(), "Level 1".into()]), Some(Difficulty::Easy));
        assert_eq!(difficulty_from_hint_tags(&["Level 3".into()]), Some(Difficulty::Medium));
        assert_eq!(difficulty_from_hint_tags(&["Level 5".into()]), Some(Difficulty::Hard));
        assert_eq!(difficulty_from_hint_tags(&["Algebra".into()]), None);
    }
}
