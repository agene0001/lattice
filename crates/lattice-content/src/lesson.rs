//! AI lesson drafting — the "learn the concept" side, complementing
//! [`crate::generate`].
//!
//! Lessons are *data*: original Markdown+KaTeX stored at
//! `subjects/<id>/notes/<concept>.md`. This drafts one for a concept you
//! haven't written yet, using the same BYOK provider as diagnosis and problem
//! generation. It authors **original** prose on purpose — the prompt forbids
//! copying or closely paraphrasing any specific source, so Lattice never
//! reproduces copyrighted notes or textbooks; it teaches the concept from
//! scratch. (Adapting openly-licensed material — e.g. OpenStax, CC-BY — is a
//! separate, by-hand authoring path that carries its own attribution.)
//!
//! Unlike generated *problems*, a lesson has no single checkable answer, so
//! there's no automated re-solve gate here. The discipline is human-in-the-loop
//! instead: a draft is shown for review and edited before it's saved.

use lattice_llm::{complete, LlmError, ProviderConfig};

/// Draft an original Markdown lesson for `concept_label`.
///
/// `group` situates the concept in the curriculum, and `prereqs` (the labels of
/// its direct prerequisites) let the model assume prior knowledge instead of
/// re-teaching it. Returns raw Markdown with math as `$…$` (inline) and `$$…$$`
/// (display) — the convention the Learn view renders.
pub async fn draft_lesson(
    config: &ProviderConfig,
    concept_label: &str,
    group: &str,
    prereqs: &[String],
) -> Result<String, LlmError> {
    let system = "You are an expert tutor writing an ORIGINAL short lesson for a \
        learning app. Teach the concept from first principles in your own words. \
        Do NOT copy or closely paraphrase any specific textbook, website, course, \
        or author — write fresh explanations. Use Markdown with KaTeX math: \
        inline math as $...$ and display math as $$...$$. Structure the lesson as: \
        a one-sentence intuition; a clear definition; one fully worked example; \
        and a short '**Common pitfall**' note. Aim for roughly 200–400 words. \
        Output ONLY the Markdown lesson — no preamble, no commentary, and no \
        surrounding code fences.";

    let prereq_line = if prereqs.is_empty() {
        "nothing in particular".to_string()
    } else {
        prereqs.join(", ")
    };
    let user = format!(
        "Concept to teach: {concept_label}\n\
         Curriculum area: {group}\n\
         The learner already knows: {prereq_line}.\n\n\
         Write the lesson now.",
    );

    complete(config, system, &user).await
}
