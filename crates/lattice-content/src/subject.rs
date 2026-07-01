//! Loading a subject's static definition from `subjects/<id>/` (spec §5, §2.6).
//!
//! A subject is *data*: a concept graph (`concepts.json`) plus problem templates
//! (`templates.json`). Adding a subject — Programming being the natural second
//! one (spec §2.6) — means adding a directory, not Rust code.

use std::path::Path;

use lattice_core::{Attribution, Concept, ConceptId, Difficulty, SubjectId};
use serde::Deserialize;

use crate::template::Template;

/// A fully-loaded subject, ready to hand to the graph engine and the generator.
#[derive(Debug, Clone)]
pub struct Subject {
    pub id: SubjectId,
    pub name: String,
    /// Curriculum groups in display order (e.g. Foundations → Algebra → …).
    pub groups: Vec<String>,
    pub concepts: Vec<Concept>,
    pub templates: Vec<Template>,
    /// Hand-authored/curated problems served verbatim (no generator). The data
    /// path for adding your own problems — see `subjects/<id>/problems.json`.
    pub static_problems: Vec<StaticProblem>,
}

/// One curated problem from `problems.json`: a literal statement + answer, with
/// optional attribution for adapted openly-licensed material.
#[derive(Debug, Clone)]
pub struct StaticProblem {
    pub id: String,
    pub concept: ConceptId,
    pub difficulty: Difficulty,
    pub content: String,
    pub solution: String,
    pub attribution: Option<Attribution>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("reading subject data at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parsing {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

/// Load the subject rooted at `dir` (e.g. `subjects/math`).
///
/// `concepts.json` is required; `templates.json` is optional (a subject can exist
/// with a graph but no generators yet).
pub fn load_subject(dir: impl AsRef<Path>) -> Result<Subject, LoadError> {
    let dir = dir.as_ref();

    let concepts_path = dir.join("concepts.json");
    let concepts_raw = read(&concepts_path)?;
    let concepts_file: ConceptsFile = parse(&concepts_path, &concepts_raw)?;
    let subject_id = SubjectId::new(concepts_file.subject.id);

    let concepts = concepts_file
        .concepts
        .into_iter()
        .map(|c| {
            let notes = read_notes(dir, c.id.as_str());
            Concept {
                id: c.id,
                subject_id: subject_id.clone(),
                label: c.label,
                group: c.group,
                notes,
                prerequisites: c.prerequisites,
            }
        })
        .collect();

    let templates_path = dir.join("templates.json");
    let templates = if templates_path.exists() {
        let raw = read(&templates_path)?;
        parse::<TemplatesFile>(&templates_path, &raw)?.templates
    } else {
        Vec::new()
    };

    // `problems.json` is optional — a subject can ship with only templates, only
    // curated problems, or both.
    let problems_path = dir.join("problems.json");
    let static_problems = if problems_path.exists() {
        let raw = read(&problems_path)?;
        parse::<ProblemsFile>(&problems_path, &raw)?
            .problems
            .into_iter()
            .map(StaticProblemDef::into_static)
            .collect()
    } else {
        Vec::new()
    };

    Ok(Subject {
        id: subject_id,
        name: concepts_file.subject.name,
        groups: concepts_file.groups,
        concepts,
        templates,
        static_problems,
    })
}

/// Load a concept's optional lesson from `<dir>/notes/<concept>.md` — the
/// "learn the concept" prose shown alongside practice. A subject can ship
/// exercises before any notes, so a missing or blank file is simply `None`,
/// never an error. Lessons are *original* Markdown+KaTeX (authored by hand or
/// AI-drafted), kept as data so they're offline, versionable, and editable.
fn read_notes(dir: &Path, concept_id: &str) -> Option<String> {
    let path = dir.join("notes").join(format!("{concept_id}.md"));
    let text = std::fs::read_to_string(path).ok()?;
    (!text.trim().is_empty()).then_some(text)
}

fn read(path: &Path) -> Result<String, LoadError> {
    std::fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn parse<T: for<'de> Deserialize<'de>>(path: &Path, raw: &str) -> Result<T, LoadError> {
    serde_json::from_str(raw).map_err(|source| LoadError::Parse {
        path: path.display().to_string(),
        source,
    })
}

// --- On-disk schema (kept private; unknown fields like `description` are ignored). ---

#[derive(Deserialize)]
struct ConceptsFile {
    subject: SubjectHeader,
    #[serde(default)]
    groups: Vec<String>,
    concepts: Vec<ConceptDef>,
}

#[derive(Deserialize)]
struct SubjectHeader {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct ConceptDef {
    id: ConceptId,
    label: String,
    #[serde(default)]
    group: String,
    #[serde(default)]
    prerequisites: Vec<ConceptId>,
}

#[derive(Deserialize)]
struct TemplatesFile {
    templates: Vec<Template>,
}

#[derive(Deserialize)]
struct ProblemsFile {
    #[serde(default)]
    problems: Vec<StaticProblemDef>,
}

#[derive(Deserialize)]
struct StaticProblemDef {
    id: String,
    concept: ConceptId,
    difficulty: Difficulty,
    content: String,
    solution: String,
    /// Flat `source`/`license` keys in JSON, folded into an [`Attribution`].
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    license: Option<String>,
}

impl StaticProblemDef {
    fn into_static(self) -> StaticProblem {
        let attribution = self
            .source
            .filter(|s| !s.trim().is_empty())
            .map(|source| Attribution {
                source,
                license: self.license.filter(|s| !s.trim().is_empty()),
            });
        StaticProblem {
            id: self.id,
            concept: self.concept,
            difficulty: self.difficulty,
            content: self.content,
            solution: self.solution,
            attribution,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn math_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../subjects/math")
    }

    #[test]
    fn loads_the_authored_math_subject() {
        let subject = load_subject(math_dir()).expect("math subject should load");
        assert_eq!(subject.id, SubjectId::new("math"));
        assert!(subject.concepts.len() >= 20, "expected a real graph");
        assert!(!subject.templates.is_empty());
    }

    /// The authored math graph must be a valid DAG with every prerequisite
    /// resolving — this is the test that catches an authoring typo.
    #[test]
    fn authored_math_graph_is_a_valid_dag() {
        let subject = load_subject(math_dir()).expect("math subject should load");
        let graph = lattice_graph::ConceptGraph::new(subject.concepts.clone());
        graph
            .validate()
            .expect("authored math graph must be a valid, acyclic DAG");
    }

    /// Coverage gate: every authored concept ships a "learn the concept" lesson
    /// under `notes/<id>.md`. A new concept without a lesson is an incomplete
    /// course — this fails loudly so the gap is filled (or the test relaxed on
    /// purpose). Also catches a filename/id typo, which would silently drop a
    /// lesson otherwise.
    #[test]
    fn every_concept_has_an_authored_lesson() {
        let subject = load_subject(math_dir()).expect("math subject should load");
        let missing: Vec<&str> = subject
            .concepts
            .iter()
            .filter(|c| c.notes.is_none())
            .map(|c| c.id.as_str())
            .collect();
        assert!(
            missing.is_empty(),
            "concepts without a lesson under subjects/math/notes/: {missing:?}"
        );
    }

    /// A blank or absent notes file leaves `notes` as `None`, never `Some("")` —
    /// so the UI's "no lesson yet" path keys off a real absence.
    #[test]
    fn blank_notes_are_treated_as_absent() {
        let dir = std::env::temp_dir().join(format!("lattice-blank-notes-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("notes")).unwrap();
        std::fs::write(
            dir.join("concepts.json"),
            r#"{"subject":{"id":"t","name":"T"},"groups":[],
                "concepts":[{"id":"a","label":"A","prerequisites":[]},
                            {"id":"b","label":"B","prerequisites":[]}]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("notes/a.md"), "   \n\t\n").unwrap(); // blank
        // `b` has no file at all.

        let subject = load_subject(&dir).expect("loads");
        for c in &subject.concepts {
            assert_eq!(c.notes, None, "concept {} should have no notes", c.id);
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Every authored subject under `subjects/` must load, be a valid DAG, and
    /// have every template + curated problem point at a real concept. This guards
    /// all subjects at once (math, physics, and any added later) against typos.
    #[test]
    fn all_authored_subjects_are_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../subjects");
        let mut checked = 0;
        for entry in std::fs::read_dir(&root).expect("subjects dir") {
            let dir = entry.unwrap().path();
            if !dir.join("concepts.json").is_file() {
                continue;
            }
            let subject = load_subject(&dir)
                .unwrap_or_else(|e| panic!("loading {}: {e}", dir.display()));
            let graph = lattice_graph::ConceptGraph::new(subject.concepts.clone());
            graph
                .validate()
                .unwrap_or_else(|e| panic!("{} is not a valid DAG: {e}", dir.display()));
            for t in &subject.templates {
                assert!(
                    graph.get(&t.concept).is_some(),
                    "{}: template `{}` targets unknown concept `{}`",
                    dir.display(),
                    t.id,
                    t.concept
                );
            }
            for p in &subject.static_problems {
                assert!(
                    graph.get(&p.concept).is_some(),
                    "{}: problem `{}` targets unknown concept `{}`",
                    dir.display(),
                    p.id,
                    p.concept
                );
            }
            checked += 1;
        }
        assert!(checked >= 2, "expected at least math + physics subjects");
    }

    /// Every template must target a concept that actually exists in the graph.
    #[test]
    fn every_template_targets_a_real_concept() {
        let subject = load_subject(math_dir()).expect("math subject should load");
        let graph = lattice_graph::ConceptGraph::new(subject.concepts.clone());
        for template in &subject.templates {
            assert!(
                graph.get(&template.concept).is_some(),
                "template `{}` targets unknown concept `{}`",
                template.id,
                template.concept
            );
        }
    }
}
