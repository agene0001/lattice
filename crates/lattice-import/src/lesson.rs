//! Lesson import (spec §10.5) — **Lane A: grounded generation**.
//!
//! Lessons aren't imported by copying (prose is the most copyrightable thing in a
//! book) and there's no CAS to verify them. So the default is *grounded*
//! generation: feed the model an openly-licensed source passage as context and
//! have it produce an original lesson **from** that — a derivative in the
//! platform's own voice/format, redistributable when the grounding is CC-BY,
//! attribution carried in frontmatter. (Lane B, direct import of CC-licensed
//! lessons, and Lane C, arbitrary PDF/HTML→markdown, layer on later.)
//!
//! The grounding adapter is local-dir, mirroring [`MathDatasetSource`]: you drop
//! converted OpenStax/LibreTexts section text into a directory (one file per
//! concept), rather than scraping live — same offline, inspectable pattern.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use lattice_content::Subject;
use lattice_core::ConceptId;
use lattice_llm::{complete, ProviderConfig};

use crate::source::License;
use crate::ImportError;

/// A source passage to ground a generated lesson on.
#[derive(Debug, Clone)]
pub struct RawLesson {
    /// The source text/markdown the lesson is generated *from*.
    pub grounding: String,
    /// Which concept this grounds, if the source says so (filename stem here).
    pub concept_hint: Option<ConceptId>,
    pub source_label: String,
    pub license: License,
}

/// A pluggable grounding source, paralleling [`ProblemSource`](crate::ProblemSource).
#[async_trait]
pub trait LessonSource {
    async fn fetch(&self) -> Result<Vec<RawLesson>, ImportError>;
}

/// Reads a directory of grounding passages (`*.md` / `*.txt`), one per concept,
/// with the **filename stem as the concept id** (e.g. `gradients.md` →
/// `gradients`). The whole directory shares one source label + license (it's one
/// book/site); convert OpenStax/LibreTexts sections into it.
pub struct LocalGroundingSource {
    dir: PathBuf,
    source_label: String,
    license: License,
}

impl LocalGroundingSource {
    pub fn new(dir: impl Into<PathBuf>, source_label: impl Into<String>, license: License) -> Self {
        Self {
            dir: dir.into(),
            source_label: source_label.into(),
            license,
        }
    }
}

#[async_trait]
impl LessonSource for LocalGroundingSource {
    async fn fetch(&self) -> Result<Vec<RawLesson>, ImportError> {
        let entries = std::fs::read_dir(&self.dir).map_err(|source| ImportError::Io {
            path: self.dir.display().to_string(),
            source,
        })?;
        let mut out = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|source| ImportError::Io {
                    path: self.dir.display().to_string(),
                    source,
                })?
                .path();
            let is_text = path
                .extension()
                .is_some_and(|e| e == "md" || e == "txt");
            if !is_text {
                continue;
            }
            let grounding = std::fs::read_to_string(&path).map_err(|source| ImportError::Io {
                path: path.display().to_string(),
                source,
            })?;
            let concept_hint = path
                .file_stem()
                .map(|s| ConceptId::new(s.to_string_lossy().to_string()));
            out.push(RawLesson {
                grounding,
                concept_hint,
                source_label: self.source_label.clone(),
                license: self.license.clone(),
            });
        }
        out.sort_by(|a, b| a.concept_hint.cmp(&b.concept_hint));
        Ok(out)
    }
}

/// Generate an original lesson body (Markdown + KaTeX) *from* a grounding passage
/// — Lane A. Returns the body only; [`compose_lesson_file`] wraps it with
/// attribution frontmatter for writing to `notes/<id>.md`.
pub async fn generate_grounded_lesson(
    config: &ProviderConfig,
    concept_label: &str,
    group: &str,
    prereqs: &[String],
    grounding: &str,
) -> Result<String, ImportError> {
    let system = "You are writing an ORIGINAL short lesson for a tutoring app, \
        GROUNDED in a provided source passage. Restructure and re-explain the \
        material in your OWN words and the app's format — do NOT copy or closely \
        paraphrase the source's sentences. Use ONLY facts supported by the \
        passage; do not add unsupported claims. Markdown with KaTeX math: inline \
        $...$ and display $$...$$. Structure: a one-sentence intuition; a clear \
        definition; one fully worked example; and a short '**Common pitfall**' \
        note. ~200–400 words. Output ONLY the Markdown lesson — no preamble, no \
        code fences.";
    let prereq_line = if prereqs.is_empty() {
        "nothing in particular".to_string()
    } else {
        prereqs.join(", ")
    };
    let user = format!(
        "Concept to teach: {concept_label}\n\
         Curriculum area: {group}\n\
         The learner already knows: {prereq_line}.\n\n\
         Source passage to ground the lesson on:\n\"\"\"\n{grounding}\n\"\"\"\n\n\
         Write the grounded lesson now."
    );
    Ok(complete(config, system, &user).await?)
}

/// Wrap a generated body with `source`/`license` frontmatter — the exact shape
/// [`lattice_content::split_frontmatter`] parses back out in the Learn view.
pub fn compose_lesson_file(source_label: &str, license_label: &str, body: &str) -> String {
    format!(
        "---\nsource: {source}\nlicense: {license}\n---\n\n{body}\n",
        source = source_label,
        license = license_label,
        body = body.trim()
    )
}

/// Map a CLI license string onto a [`License`]. Forgiving about punctuation and
/// version suffixes (`"CC-BY"`, `"cc by 4.0"`, `"CC_BY"` all → [`License::CcBy`]).
pub fn parse_license(s: &str) -> License {
    // Normalize to bare lowercase alphanumerics: "cc by 4.0" -> "ccby40".
    let norm: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).flat_map(|c| c.to_lowercase()).collect();
    if norm.contains("ncsa") {
        License::CcByNcSa
    } else if norm.starts_with("ccby") {
        License::CcBy
    } else if norm == "mit" {
        License::Mit
    } else if norm.contains("personal") || norm.contains("copyright") {
        License::CopyrightedPersonal
    } else {
        License::Other(s.trim().to_string())
    }
}

/// What a lesson-import run produced.
#[derive(Default)]
pub struct LessonReport {
    pub written: Vec<ConceptId>,
    pub skipped_existing: usize,
    pub skipped_unmapped: usize,
    pub processed: usize,
}

/// Run Lane-A generation over a grounding source, writing `notes/<id>.md` for
/// concepts that don't already have a lesson. Existing lessons are left untouched
/// unless `force` — generation never silently clobbers hand-authored prose.
pub async fn run_lesson_import(
    config: &ProviderConfig,
    source: &dyn LessonSource,
    subject: &Subject,
    notes_dir: &Path,
    limit: Option<usize>,
    force: bool,
) -> Result<LessonReport, ImportError> {
    // concept id -> (label, group, prerequisite labels)
    let by_id: HashMap<&ConceptId, &lattice_core::Concept> =
        subject.concepts.iter().map(|c| (&c.id, c)).collect();

    let raws = source.fetch().await?;
    let mut report = LessonReport::default();

    for raw in &raws {
        let Some(concept) = raw.concept_hint.clone() else {
            report.skipped_unmapped += 1;
            continue;
        };
        let Some(c) = by_id.get(&concept) else {
            // Grounding for a concept the subject doesn't have — skip, don't guess.
            report.skipped_unmapped += 1;
            continue;
        };

        let path = notes_dir.join(format!("{concept}.md"));
        if path.exists() && !force {
            report.skipped_existing += 1;
            continue;
        }
        if limit.is_some_and(|lim| report.processed >= lim) {
            break;
        }
        report.processed += 1;

        let prereqs: Vec<String> = c
            .prerequisites
            .iter()
            .map(|p| by_id.get(p).map(|pc| pc.label.clone()).unwrap_or_else(|| p.to_string()))
            .collect();
        let body =
            generate_grounded_lesson(config, &c.label, &c.group, &prereqs, &raw.grounding).await?;
        let file = compose_lesson_file(&raw.source_label, &raw.license.label(), &body);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ImportError::Io {
                path: parent.display().to_string(),
                source,
            })?;
        }
        std::fs::write(&path, file).map_err(|source| ImportError::Io {
            path: path.display().to_string(),
            source,
        })?;
        report.written.push(concept);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn license_parses_common_forms() {
        assert_eq!(parse_license("CC-BY"), License::CcBy);
        assert_eq!(parse_license("cc by 4.0"), License::CcBy);
        assert_eq!(parse_license("CC-BY-NC-SA"), License::CcByNcSa);
        assert_eq!(parse_license("MIT"), License::Mit);
        assert_eq!(parse_license("personal"), License::CopyrightedPersonal);
        assert_eq!(parse_license("WTFPL"), License::Other("WTFPL".to_string()));
    }

    #[test]
    fn composed_file_round_trips_through_the_lesson_parser() {
        let file = compose_lesson_file("OpenStax Calculus Vol 1", "CC BY 4.0", "# Gradients\n\nBody $x$.");
        let (meta, body) = lattice_content::split_frontmatter(&file);
        assert_eq!(meta.source.as_deref(), Some("OpenStax Calculus Vol 1"));
        assert_eq!(meta.license.as_deref(), Some("CC BY 4.0"));
        // Body is recovered intact (modulo a trailing newline from file hygiene).
        assert_eq!(body.trim_end(), "# Gradients\n\nBody $x$.");
    }

    #[tokio::test]
    async fn grounding_source_reads_dir_with_concept_from_filename() {
        let dir = std::env::temp_dir().join(format!("lattice-ground-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("gradients.md"), "A gradient is a vector of partials.").unwrap();
        std::fs::write(dir.join("notes.pdf"), "binary-ish").unwrap(); // ignored

        let src = LocalGroundingSource::new(&dir, "OpenStax", License::CcBy);
        let raws = src.fetch().await.unwrap();
        assert_eq!(raws.len(), 1);
        assert_eq!(raws[0].concept_hint, Some(ConceptId::new("gradients")));
        assert_eq!(raws[0].license, License::CcBy);

        std::fs::remove_dir_all(&dir).ok();
    }
}
