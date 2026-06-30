//! Source adapters (spec §10.2) and the license vocabulary (§10.6).
//!
//! A [`ProblemSource`] yields [`RawProblem`]s — problems exactly as found, before
//! any structuring. The downstream tag → verify → emit pipeline is source-
//! agnostic, so `MathDatasetSource`, a future `OpenStaxSource`, and an `OcrSource`
//! all feed the same machinery.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Deserialize;

use crate::ImportError;

/// A license, tracked from ingestion so the personal-vs-distributed filter
/// (§10.6) is a predicate rather than a re-import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum License {
    /// Creative Commons Attribution — redistributable with attribution.
    CcBy,
    /// CC Attribution-NonCommercial-ShareAlike (e.g. MIT OCW).
    CcByNcSa,
    /// MIT (e.g. the MATH and GSM8K dataset packaging).
    Mit,
    /// Pulled from a copyrighted source for personal study only — NOT for a
    /// distributed build.
    CopyrightedPersonal,
    /// Anything else, kept verbatim.
    Other(String),
}

impl License {
    /// The human-readable label stored in `problems.json` / shown in the UI.
    pub fn label(&self) -> String {
        match self {
            License::CcBy => "CC BY 4.0".to_string(),
            License::CcByNcSa => "CC BY-NC-SA 4.0".to_string(),
            License::Mit => "MIT".to_string(),
            License::CopyrightedPersonal => "copyrighted (personal use)".to_string(),
            License::Other(s) => s.clone(),
        }
    }

    /// Whether bundling this content in a *distributed* build is defensible
    /// (§10.6). Personal/copyrighted material is import-able for private study
    /// but must be filtered out before release. (Not legal advice; the NC/SA
    /// terms of [`CcByNcSa`](License::CcByNcSa) still bind a distributed build.)
    pub fn redistributable(&self) -> bool {
        matches!(self, License::CcBy | License::CcByNcSa | License::Mit)
    }
}

/// A problem exactly as found in some source, before structuring.
#[derive(Debug, Clone)]
pub struct RawProblem {
    /// Statement, as found — LaTeX or plain text.
    pub content: String,
    /// The stated final answer, if the source carries one. Problems without a
    /// solution can't be verified and are skipped.
    pub solution: Option<String>,
    /// Where it came from, e.g. a dataset name or textbook title.
    pub source_label: String,
    pub license: License,
    /// Any pre-existing subject/difficulty labels from the source (e.g. MATH's
    /// `"Algebra"` / `"Level 3"`) — hints for tagging, not authoritative.
    pub hint_tags: Vec<String>,
}

/// A pluggable problem source (§10.2). Implementors only fetch + normalize;
/// tagging, verification, and provenance are handled downstream.
#[async_trait]
pub trait ProblemSource {
    async fn fetch(&self) -> Result<Vec<RawProblem>, ImportError>;
}

// --- MATH dataset adapter (Hendrycks et al.) ---

/// Reads the MATH dataset: a directory tree of per-problem JSON files, each
/// `{ "problem", "level", "type", "solution" }`. The final answer is pulled from
/// the `\boxed{…}` in the solution; `type` and `level` become `hint_tags`.
pub struct MathDatasetSource {
    dir: PathBuf,
}

#[derive(Deserialize)]
struct MathRecord {
    problem: String,
    #[serde(default)]
    level: String,
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    solution: String,
}

impl MathDatasetSource {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn record_to_raw(rec: MathRecord) -> RawProblem {
        let mut hint_tags = Vec::new();
        if !rec.kind.trim().is_empty() {
            hint_tags.push(rec.kind.trim().to_string());
        }
        if !rec.level.trim().is_empty() {
            hint_tags.push(rec.level.trim().to_string());
        }
        RawProblem {
            content: rec.problem,
            // The dataset's answer is the boxed value in its worked solution.
            solution: extract_boxed(&rec.solution),
            source_label: "MATH (Hendrycks et al.)".to_string(),
            license: License::Mit,
            hint_tags,
        }
    }
}

#[async_trait]
impl ProblemSource for MathDatasetSource {
    async fn fetch(&self) -> Result<Vec<RawProblem>, ImportError> {
        let mut files = Vec::new();
        collect_json_files(&self.dir, &mut files)?;
        files.sort();

        let mut out = Vec::new();
        for path in files {
            let raw = std::fs::read_to_string(&path).map_err(|source| ImportError::Io {
                path: path.display().to_string(),
                source,
            })?;
            let rec: MathRecord = serde_json::from_str(&raw).map_err(|source| ImportError::Parse {
                what: path.display().to_string(),
                source,
            })?;
            out.push(Self::record_to_raw(rec));
        }
        Ok(out)
    }
}

/// Recursively gather every `*.json` file under `dir` (the MATH dataset nests
/// problems in per-topic subdirectories).
fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ImportError> {
    let entries = std::fs::read_dir(dir).map_err(|source| ImportError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| ImportError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
    Ok(())
}

/// Extract the contents of the first `\boxed{…}`, handling nested braces.
/// Returns `None` if there's no boxed answer.
pub fn extract_boxed(s: &str) -> Option<String> {
    let start = s.find("\\boxed{")?;
    let rest = &s[start + "\\boxed{".len()..];
    let mut depth = 1usize;
    let mut out = String::new();
    for ch in rest.chars() {
        match ch {
            '{' => {
                depth += 1;
                out.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let trimmed = out.trim();
                    return (!trimmed.is_empty()).then(|| trimmed.to_string());
                }
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    None // unbalanced braces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_boxed_answer_with_nested_braces() {
        assert_eq!(extract_boxed(r"the answer is $\boxed{42}$.").as_deref(), Some("42"));
        assert_eq!(
            extract_boxed(r"$\boxed{\frac{1}{2}}$").as_deref(),
            Some(r"\frac{1}{2}")
        );
        assert_eq!(extract_boxed("no box here").as_deref(), None);
        // Unbalanced — don't return a half-answer.
        assert_eq!(extract_boxed(r"\boxed{1 + 2").as_deref(), None);
    }

    #[test]
    fn math_record_maps_to_raw_with_hint_tags() {
        let rec = MathRecord {
            problem: "What is 2+2?".into(),
            level: "Level 1".into(),
            kind: "Algebra".into(),
            solution: r"We compute $2+2=\boxed{4}$.".into(),
        };
        let raw = MathDatasetSource::record_to_raw(rec);
        assert_eq!(raw.content, "What is 2+2?");
        assert_eq!(raw.solution.as_deref(), Some("4"));
        assert_eq!(raw.hint_tags, vec!["Algebra", "Level 1"]);
        assert_eq!(raw.license, License::Mit);
    }

    #[tokio::test]
    async fn fetch_walks_nested_dirs_and_skips_non_json() {
        let root = std::env::temp_dir().join(format!("lattice-mathds-{}", std::process::id()));
        let sub = root.join("algebra");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join("1.json"),
            r#"{"problem":"What is 2+2?","level":"Level 2","type":"Algebra","solution":"$2+2=\\boxed{4}$"}"#,
        )
        .unwrap();
        std::fs::write(root.join("README.txt"), "not a problem").unwrap();

        let raws = MathDatasetSource::new(&root).fetch().await.unwrap();
        assert_eq!(raws.len(), 1, "should find the nested json, skip the txt");
        assert_eq!(raws[0].content, "What is 2+2?");
        assert_eq!(raws[0].solution.as_deref(), Some("4"));
        assert_eq!(raws[0].hint_tags, vec!["Algebra", "Level 2"]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn license_redistributability() {
        assert!(License::CcBy.redistributable());
        assert!(License::Mit.redistributable());
        assert!(!License::CopyrightedPersonal.redistributable());
        assert_eq!(License::CcByNcSa.label(), "CC BY-NC-SA 4.0");
    }
}
