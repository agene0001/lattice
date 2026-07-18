//! `lattice-storage` — persistence behind a transport-agnostic trait.
//!
//! The spec's eventual shape is Postgres (writes) + DuckDB (analytics reads),
//! spec §5/§6. For a single-learner V1 that's a lot of infrastructure to stand
//! up before the engine is even validated, so V1 ships a [`Storage`] trait with
//! a [`SqliteStorage`] implementation. Because everything upstream depends on the
//! *trait*, swapping in Postgres later is a new `impl`, not a rewrite (spec §9) —
//! and `sqlx` already speaks Postgres, so the queries mostly carry over.
//!
//! Division of labour: the *static* subject definition (concepts, templates)
//! lives in `subjects/<id>/` as data and is the source of truth for the graph.
//! This crate stores the *dynamic* learner state — mastery, generated problems,
//! and attempts.

use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lattice_core::{
    Attempt, AttemptId, Concept, ConceptId, ConceptMastery, Diagnosis, DiagnosisId, LearnerId,
    Problem, ProblemId, SubjectId,
};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("malformed stored data: {0}")]
    Decode(String),
}

/// The persistence boundary every other crate depends on. Intentionally small
/// and free of any SQLite specifics.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Idempotently ensure a learner row exists.
    async fn ensure_learner(&self, learner: LearnerId) -> Result<(), StorageError>;

    /// All mastery records for a learner, keyed by concept.
    async fn load_mastery(
        &self,
        learner: LearnerId,
    ) -> Result<HashMap<ConceptId, ConceptMastery>, StorageError>;

    /// Insert or update one mastery record.
    async fn upsert_mastery(
        &self,
        learner: LearnerId,
        mastery: &ConceptMastery,
    ) -> Result<(), StorageError>;

    /// Persist a generated problem and its concept tags.
    async fn save_problem(&self, problem: &Problem) -> Result<(), StorageError>;

    async fn get_problem(&self, id: ProblemId) -> Result<Option<Problem>, StorageError>;

    /// Append an attempt to the event log (`submitted_work` captured from V1,
    /// spec §6).
    async fn record_attempt(&self, attempt: &Attempt) -> Result<(), StorageError>;

    async fn attempts_for_learner(
        &self,
        learner: LearnerId,
    ) -> Result<Vec<Attempt>, StorageError>;

    /// Persist an AI misconception diagnosis (Phase 2).
    async fn save_diagnosis(&self, diagnosis: &Diagnosis) -> Result<(), StorageError>;

    async fn diagnoses_for_attempt(
        &self,
        attempt: AttemptId,
    ) -> Result<Vec<Diagnosis>, StorageError>;
}

/// SQLite-backed [`Storage`].
#[derive(Clone)]
pub struct SqliteStorage {
    pool: SqlitePool,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS learners (
    id          TEXT PRIMARY KEY,
    created_at  TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS problems (
    id           TEXT PRIMARY KEY,
    subject_id   TEXT NOT NULL,
    content      TEXT NOT NULL,
    solution     TEXT NOT NULL,
    difficulty   TEXT NOT NULL,
    generated_by TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS problem_concepts (
    problem_id TEXT NOT NULL,
    concept_id TEXT NOT NULL,
    PRIMARY KEY (problem_id, concept_id)
);
CREATE TABLE IF NOT EXISTS attempts (
    id             TEXT PRIMARY KEY,
    learner_id     TEXT NOT NULL,
    problem_id     TEXT NOT NULL,
    submitted_work TEXT NOT NULL,
    is_correct     INTEGER NOT NULL,
    created_at     TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS learner_concept_mastery (
    learner_id        TEXT NOT NULL,
    concept_id        TEXT NOT NULL,
    state             TEXT NOT NULL,
    confidence        REAL NOT NULL,
    last_practiced_at TEXT NOT NULL,
    decay_rate        REAL NOT NULL,
    PRIMARY KEY (learner_id, concept_id)
);
CREATE TABLE IF NOT EXISTS diagnoses (
    id                  TEXT PRIMARY KEY,
    attempt_id          TEXT NOT NULL,
    diagnosed_concept   TEXT NOT NULL,
    misconception_label TEXT NOT NULL,
    explanation         TEXT NOT NULL,
    confidence          REAL NOT NULL,
    created_at          TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_attempts_learner ON attempts (learner_id, created_at);
CREATE INDEX IF NOT EXISTS idx_diagnoses_attempt ON diagnoses (attempt_id);
"#;

impl SqliteStorage {
    /// Open (creating if needed) a file-backed database, e.g.
    /// `sqlite://lattice.db` or an absolute `sqlite:///path/to/lattice.db`.
    pub async fn connect(url: &str) -> Result<Self, StorageError> {
        let opts = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    /// Open (creating if needed) a SQLite database at a filesystem path. Preferred
    /// over [`Self::connect`] when the path may contain spaces (e.g. macOS
    /// `Application Support`), since it sidesteps URL parsing.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    /// An ephemeral in-memory database — used by tests. `max_connections(1)`
    /// keeps the single connection (and thus the data) alive for the pool's life.
    pub async fn connect_in_memory() -> Result<Self, StorageError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> Result<(), StorageError> {
        sqlx::raw_sql(SCHEMA).execute(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn ensure_learner(&self, learner: LearnerId) -> Result<(), StorageError> {
        sqlx::query("INSERT OR IGNORE INTO learners (id, created_at) VALUES (?, ?)")
            .bind(learner.to_string())
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn load_mastery(
        &self,
        learner: LearnerId,
    ) -> Result<HashMap<ConceptId, ConceptMastery>, StorageError> {
        let rows = sqlx::query(
            "SELECT concept_id, state, confidence, last_practiced_at, decay_rate \
             FROM learner_concept_mastery WHERE learner_id = ?",
        )
        .bind(learner.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut out = HashMap::with_capacity(rows.len());
        for row in &rows {
            let mastery = ConceptMastery {
                concept_id: ConceptId::new(row.try_get::<String, _>("concept_id")?),
                state: enum_from_db(&row.try_get::<String, _>("state")?)?,
                confidence: row.try_get::<f64, _>("confidence")? as f32,
                last_practiced_at: parse_dt(&row.try_get::<String, _>("last_practiced_at")?)?,
                decay_rate: row.try_get::<f64, _>("decay_rate")? as f32,
            };
            out.insert(mastery.concept_id.clone(), mastery);
        }
        Ok(out)
    }

    async fn upsert_mastery(
        &self,
        learner: LearnerId,
        mastery: &ConceptMastery,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO learner_concept_mastery \
                (learner_id, concept_id, state, confidence, last_practiced_at, decay_rate) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(learner_id, concept_id) DO UPDATE SET \
                state = excluded.state, \
                confidence = excluded.confidence, \
                last_practiced_at = excluded.last_practiced_at, \
                decay_rate = excluded.decay_rate",
        )
        .bind(learner.to_string())
        .bind(mastery.concept_id.as_str())
        .bind(enum_to_db(&mastery.state))
        .bind(mastery.confidence as f64)
        .bind(mastery.last_practiced_at.to_rfc3339())
        .bind(mastery.decay_rate as f64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn save_problem(&self, problem: &Problem) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT OR REPLACE INTO problems \
                (id, subject_id, content, solution, difficulty, generated_by) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(problem.id.to_string())
        .bind(problem.subject_id.as_str())
        .bind(&problem.content)
        .bind(&problem.solution)
        .bind(enum_to_db(&problem.difficulty))
        .bind(enum_to_db(&problem.generated_by))
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM problem_concepts WHERE problem_id = ?")
            .bind(problem.id.to_string())
            .execute(&mut *tx)
            .await?;

        for concept in &problem.concepts {
            sqlx::query(
                "INSERT OR IGNORE INTO problem_concepts (problem_id, concept_id) VALUES (?, ?)",
            )
            .bind(problem.id.to_string())
            .bind(concept.as_str())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_problem(&self, id: ProblemId) -> Result<Option<Problem>, StorageError> {
        let Some(row) = sqlx::query(
            "SELECT subject_id, content, solution, difficulty, generated_by \
             FROM problems WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        let concept_rows = sqlx::query(
            "SELECT concept_id FROM problem_concepts WHERE problem_id = ? ORDER BY concept_id",
        )
        .bind(id.to_string())
        .fetch_all(&self.pool)
        .await?;
        let concepts = concept_rows
            .iter()
            .map(|r| Ok(ConceptId::new(r.try_get::<String, _>("concept_id")?)))
            .collect::<Result<Vec<_>, StorageError>>()?;

        Ok(Some(Problem {
            id,
            subject_id: SubjectId::new(row.try_get::<String, _>("subject_id")?),
            concepts,
            difficulty: enum_from_db(&row.try_get::<String, _>("difficulty")?)?,
            content: row.try_get("content")?,
            solution: row.try_get("solution")?,
            generated_by: enum_from_db(&row.try_get::<String, _>("generated_by")?)?,
            // Attribution, hints and steps are serve-time display metadata, not
            // persisted; a reloaded problem carries only what grading needs.
            attribution: None,
            hints: Vec::new(),
            steps: Vec::new(),
        }))
    }

    async fn record_attempt(&self, attempt: &Attempt) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO attempts \
                (id, learner_id, problem_id, submitted_work, is_correct, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(attempt.id.to_string())
        .bind(attempt.learner_id.to_string())
        .bind(attempt.problem_id.to_string())
        .bind(&attempt.submitted_work)
        .bind(attempt.is_correct)
        .bind(attempt.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn attempts_for_learner(
        &self,
        learner: LearnerId,
    ) -> Result<Vec<Attempt>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, learner_id, problem_id, submitted_work, is_correct, created_at \
             FROM attempts WHERE learner_id = ? ORDER BY created_at",
        )
        .bind(learner.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                Ok(Attempt {
                    id: AttemptId(parse_uuid(&row.try_get::<String, _>("id")?)?),
                    learner_id: LearnerId(parse_uuid(&row.try_get::<String, _>("learner_id")?)?),
                    problem_id: ProblemId(parse_uuid(&row.try_get::<String, _>("problem_id")?)?),
                    submitted_work: row.try_get("submitted_work")?,
                    is_correct: row.try_get("is_correct")?,
                    created_at: parse_dt(&row.try_get::<String, _>("created_at")?)?,
                })
            })
            .collect()
    }

    async fn save_diagnosis(&self, diagnosis: &Diagnosis) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT OR REPLACE INTO diagnoses \
                (id, attempt_id, diagnosed_concept, misconception_label, explanation, confidence, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(diagnosis.id.to_string())
        .bind(diagnosis.attempt_id.to_string())
        .bind(diagnosis.diagnosed_concept.as_str())
        .bind(&diagnosis.misconception_label)
        .bind(&diagnosis.explanation)
        .bind(diagnosis.confidence as f64)
        .bind(diagnosis.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn diagnoses_for_attempt(
        &self,
        attempt: AttemptId,
    ) -> Result<Vec<Diagnosis>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, attempt_id, diagnosed_concept, misconception_label, explanation, confidence, created_at \
             FROM diagnoses WHERE attempt_id = ? ORDER BY created_at",
        )
        .bind(attempt.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                Ok(Diagnosis {
                    id: DiagnosisId(parse_uuid(&row.try_get::<String, _>("id")?)?),
                    attempt_id: AttemptId(parse_uuid(&row.try_get::<String, _>("attempt_id")?)?),
                    diagnosed_concept: ConceptId::new(
                        row.try_get::<String, _>("diagnosed_concept")?,
                    ),
                    misconception_label: row.try_get("misconception_label")?,
                    explanation: row.try_get("explanation")?,
                    confidence: row.try_get::<f64, _>("confidence")? as f32,
                    created_at: parse_dt(&row.try_get::<String, _>("created_at")?)?,
                })
            })
            .collect()
    }
}

// --- decode helpers: the DB stores everything as TEXT/REAL/INTEGER ---

/// Serialize a unit enum (e.g. [`lattice_core::Difficulty`]) to its snake_case
/// string using its existing `serde` derive — no hand-kept match table.
fn enum_to_db<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        other => unreachable!("expected a unit enum to serialize to a string, got {other:?}"),
    }
}

fn enum_from_db<T: DeserializeOwned>(s: &str) -> Result<T, StorageError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| StorageError::Decode(e.to_string()))
}

fn parse_uuid(s: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(s).map_err(|e| StorageError::Decode(e.to_string()))
}

fn parse_dt(s: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| StorageError::Decode(e.to_string()))
}

// Keep `Concept` referenced so a future denormalized cache of the graph here
// doesn't surprise anyone; currently the graph's source of truth is the JSON.
#[allow(dead_code)]
fn _concept_type_anchor(_c: &Concept) {}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::{Difficulty, MasteryState, ProblemSource};

    #[tokio::test]
    async fn roundtrips_mastery_problem_and_attempt() {
        let store = SqliteStorage::connect_in_memory().await.unwrap();
        let learner = LearnerId::new();
        store.ensure_learner(learner).await.unwrap();
        store.ensure_learner(learner).await.unwrap(); // idempotent

        // upsert twice — the second must overwrite (ON CONFLICT).
        let mut mastery = ConceptMastery {
            concept_id: ConceptId::new("factoring"),
            state: MasteryState::Rusty,
            confidence: 0.42,
            last_practiced_at: Utc::now(),
            decay_rate: 0.01,
        };
        store.upsert_mastery(learner, &mastery).await.unwrap();
        mastery.state = MasteryState::Mastered;
        mastery.confidence = 0.9;
        store.upsert_mastery(learner, &mastery).await.unwrap();

        let loaded = store.load_mastery(learner).await.unwrap();
        assert_eq!(loaded.len(), 1);
        let m = &loaded[&ConceptId::new("factoring")];
        assert_eq!(m.state, MasteryState::Mastered);
        assert!((m.confidence - 0.9).abs() < 1e-6);

        // concepts in sorted order so the struct compares equal after reload.
        let problem = Problem {
            id: ProblemId::new(),
            subject_id: SubjectId::new("math"),
            concepts: vec![
                ConceptId::new("algebraic_manipulation"),
                ConceptId::new("factoring"),
            ],
            difficulty: Difficulty::Medium,
            content: "factor x^2 - 1".into(),
            solution: "(x-1)(x+1)".into(),
            generated_by: ProblemSource::Template,
            attribution: None,
            hints: Vec::new(),
            steps: Vec::new(),
        };
        store.save_problem(&problem).await.unwrap();
        assert_eq!(store.get_problem(problem.id).await.unwrap().unwrap(), problem);
        assert!(store.get_problem(ProblemId::new()).await.unwrap().is_none());

        let attempt = Attempt {
            id: AttemptId::new(),
            learner_id: learner,
            problem_id: problem.id,
            submitted_work: "(x-1)^2".into(),
            is_correct: false,
            created_at: Utc::now(),
        };
        store.record_attempt(&attempt).await.unwrap();
        let attempts = store.attempts_for_learner(learner).await.unwrap();
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].submitted_work, "(x-1)^2");
        assert!(!attempts[0].is_correct);
    }
}
