//! The import pipeline (spec §10): fetch → tag → verify → emit.
//!
//! The V1 sink is `subjects/<id>/problems.json` (the static-problem format the
//! app already serves), so imports flow into the practice pool with attribution
//! and no new storage schema. Only **verified** problems reach the live file;
//! unverified ones are parked in a review file (§10.4) rather than discarded.

use std::collections::HashSet;
use std::path::Path;

use lattice_core::{ConceptId, Difficulty};
use lattice_llm::ProviderConfig;

use crate::source::{License, ProblemSource};
use crate::tag::{structure_and_tag, ConceptVocab, TagOutcome};
use crate::verify::verify_solution;
use crate::ImportError;

/// A fully structured, provenance-carrying problem ready to emit.
#[derive(Debug, Clone)]
pub struct ImportedProblem {
    pub id: String,
    /// Primary concept (the static `problems.json` schema is single-concept;
    /// multi-concept tagging is preserved upstream and is a follow-up here).
    pub concept: ConceptId,
    pub difficulty: Difficulty,
    pub content: String,
    pub solution: String,
    pub source: String,
    pub license: License,
    pub verified: bool,
}

/// What a run produced, for the CLI's summary.
#[derive(Default)]
pub struct ImportReport {
    pub imported: Vec<ImportedProblem>,
    pub needs_review: Vec<ImportedProblem>,
    pub skipped_unmapped: usize,
    pub skipped_no_solution: usize,
    pub processed: usize,
}

/// Run the pipeline over a source. `id_prefix` namespaces generated ids (stable
/// across re-runs so a repeated import de-duplicates on merge); `limit` caps how
/// many problems incur the (paid) tag+verify LLM calls — use it for trial runs.
pub async fn run_import(
    config: &ProviderConfig,
    source: &dyn ProblemSource,
    vocab: &ConceptVocab,
    id_prefix: &str,
    limit: Option<usize>,
) -> Result<ImportReport, ImportError> {
    let raws = source.fetch().await?;
    let mut report = ImportReport::default();

    for (i, raw) in raws.iter().enumerate() {
        let Some(solution) = raw.solution.clone() else {
            report.skipped_no_solution += 1;
            continue;
        };
        if limit.is_some_and(|lim| report.processed >= lim) {
            break;
        }
        report.processed += 1;

        let tagged = match structure_and_tag(config, &raw.content, &raw.hint_tags, vocab).await? {
            TagOutcome::Tagged(t) => t,
            TagOutcome::Unmapped { .. } => {
                report.skipped_unmapped += 1;
                continue;
            }
        };

        let verified = verify_solution(config, &raw.content, &solution).await?;
        let problem = ImportedProblem {
            id: format!("{id_prefix}_{i:05}"),
            concept: tagged.concepts[0].clone(),
            difficulty: tagged.difficulty,
            content: raw.content.clone(),
            solution,
            source: raw.source_label.clone(),
            license: raw.license.clone(),
            verified,
        };
        if verified {
            report.imported.push(problem);
        } else {
            report.needs_review.push(problem);
        }
    }
    Ok(report)
}

/// Merge `problems` into a `problems.json` file (creating it if absent),
/// de-duplicating by `id` so re-runs are idempotent. Existing entries and any
/// `_comment` are preserved. Returns how many new problems were added.
pub fn merge_into_problems_json(
    path: &Path,
    problems: &[ImportedProblem],
) -> Result<usize, ImportError> {
    let io = |source: std::io::Error| ImportError::Io {
        path: path.display().to_string(),
        source,
    };
    let parse = |source: serde_json::Error| ImportError::Parse {
        what: path.display().to_string(),
        source,
    };

    let mut root: serde_json::Value = if path.exists() {
        serde_json::from_str(&std::fs::read_to_string(path).map_err(io)?).map_err(parse)?
    } else {
        serde_json::json!({ "problems": [] })
    };

    let arr = root
        .get_mut("problems")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| ImportError::Dataset(format!("{}: no `problems` array", path.display())))?;

    let existing: HashSet<String> = arr
        .iter()
        .filter_map(|e| e.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .collect();

    let mut added = 0;
    for p in problems {
        if existing.contains(&p.id) {
            continue;
        }
        arr.push(serde_json::json!({
            "id": p.id,
            "concept": p.concept,        // ConceptId serializes as a bare string
            "difficulty": p.difficulty,  // "easy" | "medium" | "hard"
            "content": p.content,
            "solution": p.solution,
            "source": p.source,
            "license": p.license.label(),
        }));
        added += 1;
    }

    let pretty = serde_json::to_string_pretty(&root).map_err(parse)?;
    std::fs::write(path, pretty + "\n").map_err(io)?;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> ImportedProblem {
        ImportedProblem {
            id: id.to_string(),
            concept: ConceptId::new("gradients"),
            difficulty: Difficulty::Medium,
            content: "\\nabla f = ?".to_string(),
            solution: "(2x, 2y)".to_string(),
            source: "MATH (Hendrycks et al.)".to_string(),
            license: License::Mit,
            verified: true,
        }
    }

    #[test]
    fn merge_creates_dedups_and_preserves() {
        let path = std::env::temp_dir().join(format!("lattice-import-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        // First write: creates the file with one problem.
        let added = merge_into_problems_json(&path, &[sample("math_00001")]).unwrap();
        assert_eq!(added, 1);

        // Second write: one duplicate id + one new → only the new one lands.
        let added = merge_into_problems_json(
            &path,
            &[sample("math_00001"), sample("math_00002")],
        )
        .unwrap();
        assert_eq!(added, 1);

        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let arr = root["problems"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        // Emitted shape matches what the loader (StaticProblemDef) expects.
        assert_eq!(arr[0]["concept"], "gradients");
        assert_eq!(arr[0]["difficulty"], "medium");
        assert_eq!(arr[0]["license"], "MIT");

        std::fs::remove_file(&path).ok();
    }
}
