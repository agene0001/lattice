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

/// Attribution pulled from a lesson's optional frontmatter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LessonMeta {
    pub source: Option<String>,
    pub license: Option<String>,
}

/// Split optional `---`-delimited YAML-ish frontmatter off the top of a lesson,
/// returning the parsed `source`/`license` and the remaining Markdown body.
///
/// A lesson may begin with:
/// ```text
/// ---
/// source: OpenStax Calculus Vol 1
/// license: CC BY 4.0
/// ---
/// # Lesson title …
/// ```
/// With no frontmatter, the whole input is returned as the body unchanged — so
/// every hand-written lesson keeps working without ceremony.
pub fn split_frontmatter(raw: &str) -> (LessonMeta, &str) {
    let mut meta = LessonMeta::default();
    let Some(after) = raw
        .strip_prefix("---\n")
        .or_else(|| raw.strip_prefix("---\r\n"))
    else {
        return (meta, raw);
    };

    let mut consumed = 0usize;
    let mut closed = false;
    for line in after.split_inclusive('\n') {
        consumed += line.len();
        let trimmed = line.trim();
        if trimmed == "---" {
            closed = true;
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let value = value.trim();
            if !value.is_empty() {
                match key.trim() {
                    "source" => meta.source = Some(value.to_string()),
                    "license" => meta.license = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }

    if closed {
        (meta, after[consumed..].trim_start_matches(['\n', '\r']))
    } else {
        // No closing fence — treat the input as an ordinary body, untouched.
        (meta, raw)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_strips_it_from_the_body() {
        let raw = "---\nsource: OpenStax Calculus Vol 1\nlicense: CC BY 4.0\n---\n# Gradients\n\nBody text.";
        let (meta, body) = split_frontmatter(raw);
        assert_eq!(meta.source.as_deref(), Some("OpenStax Calculus Vol 1"));
        assert_eq!(meta.license.as_deref(), Some("CC BY 4.0"));
        assert_eq!(body, "# Gradients\n\nBody text.");
    }

    #[test]
    fn body_without_frontmatter_is_returned_verbatim() {
        let raw = "# Dot Product\n\nNo frontmatter here.";
        let (meta, body) = split_frontmatter(raw);
        assert_eq!(meta, LessonMeta::default());
        assert_eq!(body, raw);
    }

    #[test]
    fn an_unclosed_fence_is_treated_as_plain_body() {
        // A horizontal rule mid-lesson must not be mistaken for frontmatter.
        let raw = "Intro paragraph.\n\n---\n\nMore text.";
        let (meta, body) = split_frontmatter(raw);
        assert_eq!(meta, LessonMeta::default());
        assert_eq!(body, raw);
    }
}
