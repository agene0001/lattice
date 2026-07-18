//! `lattice-service` — the orchestration layer (spec §5, §9).
//!
//! Plain async methods that wrap the domain crates, with **zero** Tauri- or
//! HTTP-specific types. `src-tauri` registers these as IPC commands; a future
//! `lattice-api` (Axum) would call the exact same methods. The dependency arrows
//! all point inward to `lattice-core`.
//!
//! This is also where the deterministic V1 "tutoring loop" lives:
//! `next_problem` → `submit_attempt` → (on failure) trace the weak prerequisite
//! and hand back targeted practice — all without an LLM (spec §2.1, data-flow §4).

use std::collections::HashMap;

use chrono::Utc;
use lattice_content::{load_subject, split_frontmatter, StaticProblem, Subject, Template};
use lattice_core::{
    Attempt, AttemptId, ConceptId, ConceptRef, Diagnosis, DiagnosisId, Difficulty, LearnerId,
    MasteryState, Problem, ProblemId, ProblemSource, SubjectId,
};
use lattice_diagnosis::DiagnosisRequest;
pub use lattice_diagnosis::{Provider, ProviderConfig};
use lattice_graph::{
    find_weakest_prerequisite, ready_frontier, Bkt, BktParams, ConceptGraph, GraphError,
    MasteryModel, WeakLink,
};
use lattice_storage::{SqliteStorage, Storage, StorageError};
// rand 0.10 moved range sampling (`random_range`) onto the `RngExt` trait.
use rand::RngExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("invalid subject graph: {0}")]
    Graph(#[from] GraphError),
    #[error("loading subject: {0}")]
    Load(#[from] lattice_content::LoadError),
    #[error("no template available for concept `{0}`")]
    NoTemplate(ConceptId),
    #[error("unknown concept: {0}")]
    UnknownConcept(ConceptId),
    #[error("lessons can't be saved for this subject (no writable source directory)")]
    NotesUnavailable,
    #[error("writing lesson: {0}")]
    Io(String),
    #[error("problem not found: {0}")]
    ProblemNotFound(ProblemId),
    #[error("subject has no studyable concepts")]
    EmptySubject,
    #[error("diagnosis failed: {0}")]
    Diagnosis(String),
    #[error("problem generation failed: {0}")]
    Generation(String),
}

/// What the UI learns from a submitted attempt (spec data-flow §4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptOutcome {
    pub attempt_id: AttemptId,
    pub is_correct: bool,
    /// The deterministic root-cause diagnosis (present only on failure).
    pub weak_link: Option<WeakLink>,
    /// A freshly generated practice problem targeting the weak link.
    pub practice: Option<Problem>,
    /// Cross-subject prerequisites of the failed concept (e.g. the Math nodes a
    /// Physics concept builds on). The orchestration layer resolves each one's
    /// mastery in its own subject to decide which are worth surfacing.
    #[serde(default)]
    pub external_prerequisites: Vec<ConceptRef>,
}

/// One concept's status for the graph view (spec §5, frontend `Graph`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptStatus {
    pub id: ConceptId,
    pub label: String,
    pub group: String,
    pub prerequisites: Vec<ConceptId>,
    /// Prerequisites in other subjects (cross-subject edges), for display + jump.
    #[serde(default)]
    pub external_prerequisites: Vec<ConceptRef>,
    /// Current decay-adjusted mastery in `[0, 1]`.
    pub estimated_mastery: f32,
    /// Last observed label, if the learner has ever practiced this.
    pub state: Option<MasteryState>,
    /// Whether a problem can be generated for this concept (a template exists).
    /// The UI uses this to decide whether the node is clickable.
    pub practiceable: bool,
    /// Whether a "learn the concept" lesson has been authored for this concept.
    /// Lets the Learn view flag which concepts still need notes written.
    pub has_notes: bool,
}

/// A practised concept whose decay-adjusted mastery falls below this is "due for
/// review" — the `familiar` boundary, so anything that's slipped to rusty/weak
/// resurfaces in the daily queue.
const REVIEW_THRESHOLD: f32 = 0.6;

/// Why a concept is in today's practice queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueKind {
    /// Practised before but decayed below the review threshold — refresh it.
    Review,
    /// Prerequisites are solid and it's not started yet — ready to learn.
    Learn,
}

/// One item in the daily practice queue (spec §5, decay-driven review).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub concept_id: ConceptId,
    pub label: String,
    pub group: String,
    pub estimated_mastery: f32,
    pub practiceable: bool,
    pub kind: QueueKind,
}

/// The "learn the concept" content for one concept (spec §2.2 — teach, then
/// practice). Carries the lesson prose plus the context the Learn view needs to
/// situate it and link straight into practice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub concept_id: ConceptId,
    pub label: String,
    pub group: String,
    pub prerequisites: Vec<ConceptId>,
    /// Prerequisites in other subjects (cross-subject edges), for display + jump.
    #[serde(default)]
    pub external_prerequisites: Vec<ConceptRef>,
    /// Original Markdown+KaTeX lesson body (frontmatter stripped), for rendering.
    /// `None` if no lesson has been written yet.
    pub notes: Option<String>,
    /// The full on-disk file including any frontmatter — what the editor loads, so
    /// that saving an edit preserves (and can change) `source`/`license` instead
    /// of dropping them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    /// Attribution from the lesson's frontmatter, when adapted from a source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Whether practice exists for this concept (drives the "Practice" button).
    pub practiceable: bool,
}

/// One way to practise a concept: a generator template (fresh numbers each time)
/// or a curated static problem (served verbatim). The practice pool mixes both.
enum PracticeItem<'a> {
    Template(&'a Template),
    Static(&'a StaticProblem),
}

/// The orchestrator. Generic over the [`Storage`] backend and the
/// [`MasteryModel`] so both are swappable (SQLite→Postgres; decay→knowledge
/// tracing) without touching call sites.
pub struct LatticeService<S: Storage, M: MasteryModel = Bkt> {
    subject: Subject,
    graph: ConceptGraph,
    storage: S,
    model: M,
    /// The subject's root directory on disk, when known. Lets lessons be read
    /// fresh and saved at runtime (the Learn view's authoring loop). `None` for
    /// in-memory services (tests), which fall back to the notes loaded at boot.
    notes_root: Option<std::path::PathBuf>,
}

impl<S: Storage, M: MasteryModel> LatticeService<S, M> {
    /// Build a service from already-loaded parts. Validates the concept graph up
    /// front so malformed authored data fails fast, not mid-session.
    pub fn new(subject: Subject, storage: S, model: M) -> Result<Self, ServiceError> {
        let graph = ConceptGraph::new(subject.concepts.clone());
        graph.validate()?;
        Ok(Self {
            subject,
            graph,
            storage,
            model,
            notes_root: None,
        })
    }

    /// Point the service at the subject's on-disk root so lessons can be read
    /// fresh and saved at runtime. The builder form lets `bootstrap` set this
    /// without widening [`Self::new`], which tests call with no directory.
    pub fn with_notes_root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.notes_root = Some(root.into());
        self
    }

    /// Generate (and persist) the next problem for a learner: prefer a concept on
    /// the "ready to learn" frontier that we have a template for, else fall back
    /// to any template. Forward progress, not just remediation (spec §2.2).
    pub async fn next_problem(&self, learner: LearnerId) -> Result<Problem, ServiceError> {
        self.storage.ensure_learner(learner).await?;
        let masteries = self.storage.load_mastery(learner).await?;
        let now = Utc::now();

        let frontier = ready_frontier(&self.graph, &masteries, &self.model, now);
        let target = frontier
            .iter()
            .find(|c| self.has_practice(c))
            .cloned()
            .or_else(|| self.first_practiceable_concept())
            .ok_or(ServiceError::EmptySubject)?;

        self.generate_for(&target)
            .await?
            .ok_or(ServiceError::NoTemplate(target))
    }

    /// Generate (and persist) a problem for a *specific* concept the learner
    /// picked — from the concept map or a related-topic chip. Errors with
    /// [`ServiceError::NoTemplate`] if that concept has no exercises yet.
    pub async fn practice_concept(
        &self,
        learner: LearnerId,
        concept: ConceptId,
    ) -> Result<Problem, ServiceError> {
        self.storage.ensure_learner(learner).await?;
        self.generate_for(&concept)
            .await?
            .ok_or(ServiceError::NoTemplate(concept))
    }

    /// Phase 2: diagnose *why* a wrong attempt was wrong, using the BYOK provider
    /// the app supplies. Reads the submitted work (not just the final answer),
    /// calls the LLM, persists the structured diagnosis, and returns it
    /// (spec §2.4, §7). Keychain/settings live in the app layer — the service
    /// just receives a ready [`ProviderConfig`].
    pub async fn diagnose_attempt(
        &self,
        attempt_id: AttemptId,
        problem_id: ProblemId,
        submitted_work: &str,
        provider: &ProviderConfig,
    ) -> Result<Diagnosis, ServiceError> {
        let problem = self
            .storage
            .get_problem(problem_id)
            .await?
            .ok_or(ServiceError::ProblemNotFound(problem_id))?;

        let concept_id = problem
            .concepts
            .first()
            .cloned()
            .unwrap_or_else(|| ConceptId::new("unknown"));
        let concept_label = self
            .graph
            .get(&concept_id)
            .map(|c| c.label.clone())
            .unwrap_or_else(|| concept_id.to_string());

        let request = DiagnosisRequest {
            problem_content: &problem.content,
            solution: &problem.solution,
            submitted_work,
            concept_label: &concept_label,
            concept_id: concept_id.as_str(),
        };
        let result = lattice_diagnosis::diagnose(provider, &request)
            .await
            .map_err(|e| ServiceError::Diagnosis(e.to_string()))?;

        let diagnosis = Diagnosis {
            id: DiagnosisId::new(),
            attempt_id,
            diagnosed_concept: ConceptId::new(result.diagnosed_concept),
            misconception_label: result.misconception_label,
            explanation: result.explanation,
            confidence: result.confidence.clamp(0.0, 1.0),
            created_at: Utc::now(),
        };
        self.storage.save_diagnosis(&diagnosis).await?;
        Ok(diagnosis)
    }

    /// Phase 3: generate a fresh, varied problem for `concept` at `difficulty`
    /// using the BYOK provider, verified by an independent re-solve before it's
    /// shown (spec §2.3). Persists and returns it.
    pub async fn generate_ai_problem(
        &self,
        learner: LearnerId,
        concept: ConceptId,
        difficulty: Difficulty,
        provider: &ProviderConfig,
    ) -> Result<Problem, ServiceError> {
        self.storage.ensure_learner(learner).await?;
        let label = self
            .graph
            .get(&concept)
            .map(|c| c.label.clone())
            .unwrap_or_else(|| concept.to_string());
        let problem = lattice_content::generate_problem(
            provider,
            &self.subject.id,
            &concept,
            &label,
            difficulty,
        )
        .await
        .map_err(|e| ServiceError::Generation(e.to_string()))?;
        self.storage.save_problem(&problem).await?;
        Ok(problem)
    }

    /// Teach when stuck: a BYOK step-by-step worked solution for a problem,
    /// anchored to its known-correct answer (spec §2.5). Works for any problem —
    /// templated, static, or AI — that lacks authored [`steps`](Problem::steps).
    pub async fn explain_problem(
        &self,
        problem_id: ProblemId,
        provider: &ProviderConfig,
    ) -> Result<String, ServiceError> {
        let problem = self
            .storage
            .get_problem(problem_id)
            .await?
            .ok_or(ServiceError::ProblemNotFound(problem_id))?;
        lattice_content::explain_problem(provider, &problem.content, &problem.solution)
            .await
            .map_err(|e| ServiceError::Generation(e.to_string()))
    }

    /// Grade a submission, log it, update mastery, and on failure trace the weak
    /// prerequisite and offer targeted practice — the whole V1 loop.
    pub async fn submit_attempt(
        &self,
        learner: LearnerId,
        problem_id: ProblemId,
        submitted_work: String,
    ) -> Result<AttemptOutcome, ServiceError> {
        let problem = self
            .storage
            .get_problem(problem_id)
            .await?
            .ok_or(ServiceError::ProblemNotFound(problem_id))?;

        let is_correct = answer_is_correct(&problem, &submitted_work);
        let now = Utc::now();

        let attempt = Attempt {
            id: AttemptId::new(),
            learner_id: learner,
            problem_id,
            submitted_work,
            is_correct,
            created_at: now,
        };
        self.storage.record_attempt(&attempt).await?;

        // Snapshot mastery *before* updating, so the diagnosis reflects the state
        // that produced the failure.
        let masteries = self.storage.load_mastery(learner).await?;
        for concept in &problem.concepts {
            let updated = self.model.update(masteries.get(concept), concept, is_correct, now);
            self.storage.upsert_mastery(learner, &updated).await?;
        }

        let (weak_link, practice, external_prerequisites) = if is_correct {
            (None, None, Vec::new())
        } else {
            let weak =
                find_weakest_prerequisite(&self.graph, &problem, &masteries, &self.model, now);
            let practice = match &weak {
                Some(w) => self.generate_for(&w.concept_id).await?,
                None => None,
            };
            // Cross-subject prerequisites of the failed concept(s), for the
            // orchestration layer to weigh against other subjects' mastery.
            let mut external = Vec::new();
            for concept in &problem.concepts {
                if let Some(c) = self.graph.get(concept) {
                    for r in &c.external_prerequisites {
                        if !external.contains(r) {
                            external.push(r.clone());
                        }
                    }
                }
            }
            (weak, practice, external)
        };

        Ok(AttemptOutcome {
            attempt_id: attempt.id,
            is_correct,
            weak_link,
            practice,
            external_prerequisites,
        })
    }

    /// The learner's current (decay-adjusted) mastery of one concept, or `None`
    /// if never practiced. Used by the orchestration layer to weigh a
    /// cross-subject prerequisite that lives in this subject.
    pub async fn concept_mastery(
        &self,
        learner: LearnerId,
        concept: &ConceptId,
    ) -> Result<Option<f32>, ServiceError> {
        self.storage.ensure_learner(learner).await?;
        let masteries = self.storage.load_mastery(learner).await?;
        let now = Utc::now();
        Ok(masteries
            .get(concept)
            .map(|m| self.model.estimated_mastery(m, now)))
    }

    /// The whole concept graph annotated with the learner's current mastery —
    /// drives the visual prerequisite map (spec §5).
    pub async fn concept_map(&self, learner: LearnerId) -> Result<Vec<ConceptStatus>, ServiceError> {
        self.storage.ensure_learner(learner).await?;
        let masteries = self.storage.load_mastery(learner).await?;
        let now = Utc::now();

        let mut statuses: Vec<ConceptStatus> = self
            .graph
            .concepts()
            .map(|c| {
                let mastery = masteries.get(&c.id);
                ConceptStatus {
                    id: c.id.clone(),
                    label: c.label.clone(),
                    group: c.group.clone(),
                    prerequisites: c.prerequisites.clone(),
                    external_prerequisites: c.external_prerequisites.clone(),
                    estimated_mastery: mastery
                        .map_or(0.0, |m| self.model.estimated_mastery(m, now)),
                    state: mastery.map(|m| m.state),
                    practiceable: self.has_practice(&c.id),
                    has_notes: self.has_notes(&c.id),
                }
            })
            .collect();
        statuses.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(statuses)
    }

    /// The "learn the concept" lesson for `concept`: its authored notes plus the
    /// context the Learn view needs to situate it and link into practice. Notes
    /// are read fresh from disk (when a [`notes_root`](Self::with_notes_root) is
    /// set) so a just-saved or AI-drafted lesson shows without a restart.
    pub fn lesson(&self, concept: &ConceptId) -> Result<Lesson, ServiceError> {
        let c = self
            .graph
            .get(concept)
            .ok_or_else(|| ServiceError::UnknownConcept(concept.clone()))?;
        // Strip optional frontmatter so the body renders cleanly and the source
        // /license surface separately for attribution.
        let raw_file = self.read_notes(concept);
        let (meta, notes) = match &raw_file {
            Some(raw) => {
                let (meta, body) = split_frontmatter(raw);
                (meta, (!body.trim().is_empty()).then(|| body.to_string()))
            }
            None => (Default::default(), None),
        };
        Ok(Lesson {
            concept_id: c.id.clone(),
            label: c.label.clone(),
            group: c.group.clone(),
            prerequisites: c.prerequisites.clone(),
            external_prerequisites: c.external_prerequisites.clone(),
            notes,
            raw: raw_file,
            source: meta.source,
            license: meta.license,
            practiceable: self.has_practice(concept),
        })
    }

    /// Draft an original lesson for `concept` with the BYOK provider (spec §2.2).
    /// Returns the Markdown **without saving** — the caller previews and edits it
    /// before [`save_lesson`](Self::save_lesson). Authoring is original by design
    /// (the drafting prompt forbids copying any source), so no copyrighted text
    /// enters the corpus.
    pub async fn draft_lesson(
        &self,
        concept: &ConceptId,
        provider: &ProviderConfig,
    ) -> Result<String, ServiceError> {
        let c = self
            .graph
            .get(concept)
            .ok_or_else(|| ServiceError::UnknownConcept(concept.clone()))?;
        // Own everything before the await so the future stays `Send` and no
        // borrow of `self.graph` is held across it.
        let label = c.label.clone();
        let group = c.group.clone();
        let prereqs: Vec<String> = c
            .prerequisites
            .iter()
            .map(|p| {
                self.graph
                    .get(p)
                    .map(|pc| pc.label.clone())
                    .unwrap_or_else(|| p.to_string())
            })
            .collect();

        lattice_content::draft_lesson(provider, &label, &group, &prereqs)
            .await
            .map_err(|e| ServiceError::Generation(e.to_string()))
    }

    /// Persist an authored or edited lesson to `subjects/<id>/notes/<concept>.md`.
    /// Requires a known subject root (always true for the bootstrapped app).
    pub fn save_lesson(&self, concept: &ConceptId, markdown: &str) -> Result<(), ServiceError> {
        if self.graph.get(concept).is_none() {
            return Err(ServiceError::UnknownConcept(concept.clone()));
        }
        let path = self
            .concept_notes_path(concept)
            .ok_or(ServiceError::NotesUnavailable)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ServiceError::Io(e.to_string()))?;
        }
        std::fs::write(&path, markdown).map_err(|e| ServiceError::Io(e.to_string()))
    }

    /// Today's practice queue: concepts **due for review** (practised but decayed
    /// below [`REVIEW_THRESHOLD`]) first, most-decayed first, then concepts **ready
    /// to learn** (prerequisites solid, not yet started). This is what turns the
    /// decay model into a habit — "here's what to practise today" — instead of
    /// making the learner choose from the whole map (spec §5).
    pub async fn practice_queue(&self, learner: LearnerId) -> Result<Vec<QueueItem>, ServiceError> {
        self.storage.ensure_learner(learner).await?;
        let masteries = self.storage.load_mastery(learner).await?;
        let now = Utc::now();
        let estimate = |id: &ConceptId| -> f32 {
            masteries.get(id).map_or(0.0, |m| self.model.estimated_mastery(m, now))
        };
        let frontier: std::collections::HashSet<ConceptId> =
            ready_frontier(&self.graph, &masteries, &self.model, now)
                .into_iter()
                .collect();

        let mut items: Vec<QueueItem> = Vec::new();
        for c in self.graph.concepts() {
            let mastery = estimate(&c.id);
            let practised = masteries.contains_key(&c.id);
            let practiceable = self.has_practice(&c.id);
            let kind = if practised && mastery < REVIEW_THRESHOLD {
                Some(QueueKind::Review)
            } else if !practised && practiceable && frontier.contains(&c.id) {
                Some(QueueKind::Learn)
            } else {
                None
            };
            if let Some(kind) = kind {
                items.push(QueueItem {
                    concept_id: c.id.clone(),
                    label: c.label.clone(),
                    group: c.group.clone(),
                    estimated_mastery: mastery,
                    practiceable,
                    kind,
                });
            }
        }
        // Review before Learn; within review, most-decayed first; within learn,
        // fewest prerequisites (foundations) first.
        items.sort_by(|a, b| match (a.kind, b.kind) {
            (QueueKind::Review, QueueKind::Learn) => std::cmp::Ordering::Less,
            (QueueKind::Learn, QueueKind::Review) => std::cmp::Ordering::Greater,
            (QueueKind::Review, QueueKind::Review) => a
                .estimated_mastery
                .partial_cmp(&b.estimated_mastery)
                .unwrap_or(std::cmp::Ordering::Equal),
            (QueueKind::Learn, QueueKind::Learn) => a.label.cmp(&b.label),
        });
        Ok(items)
    }

    pub fn subject_id(&self) -> &SubjectId {
        &self.subject.id
    }

    pub fn subject_name(&self) -> &str {
        &self.subject.name
    }

    /// Curriculum groups in display order.
    pub fn groups(&self) -> &[String] {
        &self.subject.groups
    }

    /// A concept's display label and whether it's practiceable — used by the
    /// orchestration layer to resolve a cross-subject prerequisite reference.
    pub fn concept_brief(&self, concept: &ConceptId) -> Option<(String, bool)> {
        self.graph
            .get(concept)
            .map(|c| (c.label.clone(), self.has_practice(concept)))
    }

    // --- internals ---

    /// Whether `concept` has *any* practice — a generator template or a curated
    /// static problem. Drives the practiceable flag and the next-problem pick.
    fn has_practice(&self, concept: &ConceptId) -> bool {
        self.subject.templates.iter().any(|t| &t.concept == concept)
            || self
                .subject
                .static_problems
                .iter()
                .any(|p| &p.concept == concept)
    }

    /// On-disk path for a concept's lesson, when the subject root is known.
    fn concept_notes_path(&self, concept: &ConceptId) -> Option<std::path::PathBuf> {
        self.notes_root
            .as_ref()
            .map(|root| root.join("notes").join(format!("{concept}.md")))
    }

    /// A concept's authored notes: the on-disk file if present (so saved/AI
    /// drafts appear without a restart), otherwise whatever was loaded at boot.
    fn read_notes(&self, concept: &ConceptId) -> Option<String> {
        if let Some(path) = self.concept_notes_path(concept) {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if !text.trim().is_empty() {
                    return Some(text);
                }
            }
        }
        self.graph.get(concept).and_then(|c| c.notes.clone())
    }

    /// Whether a lesson exists for `concept` — a cheap existence check for the
    /// concept map, avoiding reading every file just to flag which have notes.
    fn has_notes(&self, concept: &ConceptId) -> bool {
        self.concept_notes_path(concept)
            .is_some_and(|p| p.exists())
            || self
                .graph
                .get(concept)
                .is_some_and(|c| c.notes.is_some())
    }

    /// The first concept (in graph order) with any practice — the deterministic
    /// fallback when the ready frontier has nothing practiceable.
    fn first_practiceable_concept(&self) -> Option<ConceptId> {
        self.graph
            .concepts()
            .map(|c| &c.id)
            .find(|id| self.has_practice(id))
            .cloned()
    }

    /// Every way to practise `concept`: each generator template plus each curated
    /// static problem. A concept can have several — difficulty tiers and distinct
    /// problem *forms* — and [`generate_for`](Self::generate_for) picks one at
    /// random so repeated practice varies instead of repeating one form.
    fn practice_pool<'a>(&'a self, concept: &ConceptId) -> Vec<PracticeItem<'a>> {
        let templates = self
            .subject
            .templates
            .iter()
            .filter(|t| &t.concept == concept)
            .map(PracticeItem::Template);
        let statics = self
            .subject
            .static_problems
            .iter()
            .filter(|p| &p.concept == concept)
            .map(PracticeItem::Static);
        templates.chain(statics).collect()
    }

    /// Materialise + persist a problem for `concept`, or `None` if it has no
    /// practice at all. One item from the pool is chosen uniformly at random; a
    /// template is generated fresh, a static problem is served verbatim with a new
    /// id. The RNG is created and dropped inside the sync block so the returned
    /// future stays `Send`.
    async fn generate_for(&self, concept: &ConceptId) -> Result<Option<Problem>, ServiceError> {
        let pool = self.practice_pool(concept);
        if pool.is_empty() {
            return Ok(None);
        }
        let problem = {
            let mut rng = rand::rng();
            match pool[rng.random_range(0..pool.len())] {
                PracticeItem::Template(t) => t.generate(&self.subject.id, &mut rng),
                PracticeItem::Static(s) => Problem {
                    id: ProblemId::new(),
                    subject_id: self.subject.id.clone(),
                    concepts: vec![s.concept.clone()],
                    difficulty: s.difficulty,
                    content: s.content.clone(),
                    solution: s.solution.clone(),
                    generated_by: ProblemSource::Static,
                    attribution: s.attribution.clone(),
                    hints: s.hints.clone(),
                    steps: s.steps.clone(),
                },
            }
        };
        self.storage.save_problem(&problem).await?;
        Ok(Some(problem))
    }

}

impl LatticeService<SqliteStorage, Bkt> {
    /// Convenience constructor for the default V1 stack: load a subject from a
    /// directory, open a SQLite database, and use Bayesian Knowledge Tracing as
    /// the learner model. This is what `src-tauri` calls at startup.
    pub async fn bootstrap(
        subject_dir: impl AsRef<std::path::Path>,
        db_path: impl AsRef<std::path::Path>,
    ) -> Result<Self, ServiceError> {
        let root = subject_dir.as_ref().to_path_buf();
        let subject = load_subject(&root)?;
        let storage = SqliteStorage::open(db_path).await?;
        // Keep the root so lessons can be read fresh and saved at runtime.
        Ok(Self::new(subject, storage, Bkt::default())?.with_notes_root(root))
    }
}

/// Knowledge-tracing operations available when the learner model is BKT.
impl<S: Storage> LatticeService<S, Bkt> {
    /// The learner model's current parameters.
    pub fn model_params(&self) -> BktParams {
        self.model.params()
    }

    /// **Rung 2 — refit the BKT parameters to this learner's full attempt
    /// history and apply them in place.** Returns the fitted parameters.
    ///
    /// Sequences are built per concept (in chronological order) then pooled —
    /// one learner rarely has enough data to fit each skill separately.
    pub async fn refit_model(&self, learner: LearnerId) -> Result<BktParams, ServiceError> {
        let attempts = self.storage.attempts_for_learner(learner).await?;
        let mut by_concept: HashMap<ConceptId, Vec<bool>> = HashMap::new();
        for attempt in &attempts {
            if let Some(problem) = self.storage.get_problem(attempt.problem_id).await? {
                for concept in problem.concepts {
                    by_concept.entry(concept).or_default().push(attempt.is_correct);
                }
            }
        }
        let sequences: Vec<Vec<bool>> = by_concept.into_values().collect();
        Ok(self.model.fit(&sequences))
    }
}

/// V1 correctness check (spec open Q6) — delegates to the shared
/// [`lattice_core::answers_match`] equivalence (numeric/fraction by value, else
/// substring) so grading and AI-generation verification stay consistent.
fn answer_is_correct(problem: &Problem, submitted_work: &str) -> bool {
    lattice_core::answers_match(&problem.solution, submitted_work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::{Difficulty, ProblemSource};

    fn problem_with_solution(sol: &str) -> Problem {
        Problem {
            id: ProblemId::new(),
            subject_id: SubjectId::new("math"),
            concepts: vec![],
            difficulty: Difficulty::Easy,
            content: String::new(),
            solution: sol.to_string(),
            generated_by: ProblemSource::Template,
            attribution: None,
            hints: Vec::new(),
            steps: Vec::new(),
        }
    }

    #[test]
    fn numeric_answers_compare_by_value() {
        let ok = |sol: &str, work: &str| answer_is_correct(&problem_with_solution(sol), work);
        assert!(ok("x = 3", "3"));
        assert!(ok("x = 3", "x = 3"));
        assert!(ok("3", "the answer is 3"));
        assert!(ok("x = 3", "2x + 1 = 7\n2x = 6\nx = 3")); // multi-line work
        assert!(ok("1/2", "2/4")); // unreduced fraction
        assert!(ok("1/2", "0.5")); // decimal
        // No spurious substring matches:
        assert!(!ok("7", "70"));
        assert!(!ok("3", "i think it is 4"));
    }

    #[test]
    fn structural_answers_use_substring() {
        let ok = |sol: &str, work: &str| answer_is_correct(&problem_with_solution(sol), work);
        assert!(ok("(x - 1)(x + 1)", "(x-1)(x+1)"));
        assert!(ok("2x + 3", "the derivative is 2x + 3"));
        assert!(ok("3, 5", "(3, 5)"));
        assert!(!ok("2x + 3", "2x + 5"));
    }

    fn math_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../subjects/math")
    }

    async fn service() -> LatticeService<SqliteStorage, Bkt> {
        let subject = load_subject(math_dir()).unwrap();
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        LatticeService::new(subject, storage, Bkt::default()).unwrap()
    }

    #[tokio::test]
    async fn full_loop_diagnoses_failure_and_rewards_success() {
        let svc = service().await;
        let learner = LearnerId::new();

        // A wrong answer: logged, diagnosed to a weak link, practice offered.
        let problem = svc.next_problem(learner).await.unwrap();
        let wrong = svc
            .submit_attempt(learner, problem.id, "definitely not it".into())
            .await
            .unwrap();
        assert!(!wrong.is_correct);
        assert!(wrong.weak_link.is_some(), "failure should trace to a concept");
        assert!(
            wrong.practice.is_some(),
            "a practice problem should be offered for the weak link"
        );

        // A correct answer (echo the solution back) raises mastery.
        let problem = svc.next_problem(learner).await.unwrap();
        let right = svc
            .submit_attempt(learner, problem.id, problem.solution.clone())
            .await
            .unwrap();
        assert!(right.is_correct);
        assert!(right.weak_link.is_none());

        let map = svc.concept_map(learner).await.unwrap();
        assert!(
            map.iter().any(|c| c.estimated_mastery > 0.0),
            "mastery should be recorded after a correct attempt"
        );
        // Sanity: the map covers the whole authored graph.
        assert!(map.len() >= 20);
    }

    #[tokio::test]
    async fn save_then_read_lesson_round_trips_via_disk() {
        let subject = load_subject(math_dir()).unwrap();
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        // An isolated notes root so the test never writes into the real subject.
        let tmp = std::env::temp_dir().join(format!("lattice-notes-{}", std::process::id()));
        let svc = LatticeService::new(subject, storage, Bkt::default())
            .unwrap()
            .with_notes_root(&tmp);

        // A lesson saved at runtime is read straight back from the notes root,
        // overriding the copy loaded at boot — so edits show without a restart.
        let concept = ConceptId::new("variance");
        let boot = svc.lesson(&concept).unwrap().notes;
        let body = "# Variance (edited)\n\nA distinctive sentinel body for the test.";
        assert_ne!(boot.as_deref(), Some(body), "sentinel must differ from boot copy");

        svc.save_lesson(&concept, body).unwrap();
        let lesson = svc.lesson(&concept).unwrap();
        assert_eq!(lesson.notes.as_deref(), Some(body));
        assert_eq!(lesson.label, "Variance");

        // Unknown concepts error rather than writing an orphan file.
        assert!(matches!(
            svc.save_lesson(&ConceptId::new("nope"), "x"),
            Err(ServiceError::UnknownConcept(_))
        ));
        assert!(matches!(
            svc.lesson(&ConceptId::new("nope")),
            Err(ServiceError::UnknownConcept(_))
        ));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[tokio::test]
    async fn editing_a_lesson_preserves_frontmatter_attribution() {
        let subject = load_subject(math_dir()).unwrap();
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        let tmp = std::env::temp_dir().join(format!("lattice-fm-{}", std::process::id()));
        let svc = LatticeService::new(subject, storage, Bkt::default())
            .unwrap()
            .with_notes_root(&tmp);
        let concept = ConceptId::new("expectation");

        // The editor saves the full raw file (frontmatter included).
        let file = "---\nsource: OpenStax\nlicense: CC BY 4.0\n---\n\n# Expectation\n\nBody $E[X]$.";
        svc.save_lesson(&concept, file).unwrap();

        let lesson = svc.lesson(&concept).unwrap();
        // `raw` is what the editor re-loads — frontmatter intact, so a re-save
        // can't silently drop the attribution.
        assert_eq!(lesson.raw.as_deref(), Some(file));
        // `notes` renders the body only; source/license surface for the footer.
        assert_eq!(lesson.notes.as_deref(), Some("# Expectation\n\nBody $E[X]$."));
        assert_eq!(lesson.source.as_deref(), Some("OpenStax"));
        assert_eq!(lesson.license.as_deref(), Some("CC BY 4.0"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[tokio::test]
    async fn serves_curated_static_problems_with_attribution() {
        // A subject with a concept that has ONLY a curated static problem (no
        // template) — the "add your own problems as data" path.
        let dir = std::env::temp_dir().join(format!("lattice-static-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("concepts.json"),
            r#"{"subject":{"id":"t","name":"T"},"groups":["G"],
                "concepts":[{"id":"a","label":"A","group":"G","prerequisites":[]}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("problems.json"),
            r#"{"problems":[{"id":"p1","concept":"a","difficulty":"easy",
                "content":"1+1=?","solution":"2","source":"OpenStax","license":"CC BY 4.0"}]}"#,
        )
        .unwrap();

        let subject = load_subject(&dir).unwrap();
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        let svc = LatticeService::new(subject, storage, Bkt::default()).unwrap();
        let learner = LearnerId::new();

        let p = svc
            .practice_concept(learner, ConceptId::new("a"))
            .await
            .unwrap();
        assert_eq!(p.generated_by, ProblemSource::Static);
        assert_eq!(p.solution, "2");
        let attr = p.attribution.clone().expect("attribution flows through");
        assert_eq!(attr.source, "OpenStax");
        assert_eq!(attr.license.as_deref(), Some("CC BY 4.0"));

        // It grades through the normal loop even though attribution isn't persisted.
        let outcome = svc.submit_attempt(learner, p.id, "2".into()).await.unwrap();
        assert!(outcome.is_correct);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn practice_queue_separates_review_from_learn() {
        let svc = service().await;
        let learner = LearnerId::new();

        // Nothing practised yet: the queue is all "learn" (ready frontier).
        let q = svc.practice_queue(learner).await.unwrap();
        assert!(!q.is_empty(), "a fresh learner should have things ready to learn");
        assert!(q.iter().all(|i| i.kind == QueueKind::Learn));

        // Fail a concept → it gets a low-mastery record → surfaces as review.
        let p = svc.next_problem(learner).await.unwrap();
        let concept = p.concepts[0].clone();
        svc.submit_attempt(learner, p.id, "definitely wrong".into())
            .await
            .unwrap();
        let q = svc.practice_queue(learner).await.unwrap();
        assert!(
            q.iter()
                .any(|i| i.concept_id == concept && i.kind == QueueKind::Review),
            "a just-failed concept should be due for review"
        );
        // Review items sort ahead of learn items.
        if let (Some(first_review), Some(first_learn)) = (
            q.iter().position(|i| i.kind == QueueKind::Review),
            q.iter().position(|i| i.kind == QueueKind::Learn),
        ) {
            assert!(first_review < first_learn, "reviews come before new material");
        }
    }

    #[tokio::test]
    async fn unknown_problem_is_an_error_not_a_panic() {
        let svc = service().await;
        let err = svc
            .submit_attempt(LearnerId::new(), ProblemId::new(), "x".into())
            .await;
        assert!(matches!(err, Err(ServiceError::ProblemNotFound(_))));
    }

    #[tokio::test]
    async fn refit_model_fits_and_applies_from_history() {
        let svc = service().await;
        let learner = LearnerId::new();
        for _ in 0..4 {
            let p = svc.next_problem(learner).await.unwrap();
            svc.submit_attempt(learner, p.id, p.solution.clone())
                .await
                .unwrap();
        }
        let fitted = svc.refit_model(learner).await.unwrap();
        for v in [fitted.p_init, fitted.p_learn, fitted.p_slip, fitted.p_guess] {
            assert!((0.0..=1.0).contains(&v), "param out of range: {v}");
        }
        assert!(fitted.p_slip < 0.5 && fitted.p_guess < 0.5);
        // The fit is applied to the live model in place.
        assert_eq!(svc.model_params().p_slip, fitted.p_slip);
    }
}
