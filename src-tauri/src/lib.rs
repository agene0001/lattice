//! Tauri shell for Lattice.
//!
//! This crate is a *thin adapter* (spec §9): it owns no domain logic. It boots a
//! [`LatticeService`] at startup, holds it as managed state, and exposes a
//! handful of `#[tauri::command]`s that forward to the service. A future
//! `lattice-api` (Axum) would be the same forwarding over HTTP instead of IPC.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lattice_core::{
    AttemptId, ConceptId, ConceptRef, Diagnosis, Difficulty, LearnerId, Problem, ProblemId,
};
use lattice_graph::{Bkt, BktParams};
use lattice_service::{
    AttemptOutcome, ConceptStatus, LatticeService, Lesson, Provider, ProviderConfig, ServiceError,
};
use lattice_storage::SqliteStorage;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use uuid::Uuid;

type Service = LatticeService<SqliteStorage, Bkt>;

/// Single-user V1: one fixed learner (you, dogfooding — spec open Q1/§9). A
/// stable id so progress persists across launches.
const SOLO_LEARNER: LearnerId =
    LearnerId(Uuid::from_u128(0x1a77_1ce0_0000_4000_8000_0000_0000_0001));

/// One [`LatticeService`] per subject, keyed by subject id, plus the currently
/// selected subject. The map is immutable after setup, so commands borrow a
/// `&Service` straight from it; only the *selection* needs a lock.
struct AppState {
    subjects: HashMap<String, Service>,
    /// Subjects in display order (discovery order), for the switcher.
    order: Vec<SubjectSummary>,
    active: Mutex<String>,
    learner: LearnerId,
    config_dir: PathBuf,
}

impl AppState {
    /// The service for the currently selected subject.
    fn active_service(&self) -> Result<&Service, String> {
        let id = self
            .active
            .lock()
            .map_err(|_| "subject selection lock poisoned".to_string())?
            .clone();
        self.subjects
            .get(&id)
            .ok_or_else(|| format!("unknown subject: {id}"))
    }
}

#[derive(Serialize, Clone)]
struct SubjectSummary {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct SubjectInfo {
    id: String,
    name: String,
    groups: Vec<String>,
}

fn to_message(err: ServiceError) -> String {
    err.to_string()
}

/// The subjects available to switch between (the whole `subjects/` directory).
#[tauri::command]
async fn list_subjects(state: State<'_, AppState>) -> Result<Vec<SubjectSummary>, String> {
    Ok(state.order.clone())
}

/// Switch the active subject. Subsequent commands operate on it.
#[tauri::command]
async fn select_subject(state: State<'_, AppState>, subject_id: String) -> Result<(), String> {
    if !state.subjects.contains_key(&subject_id) {
        return Err(format!("unknown subject: {subject_id}"));
    }
    *state
        .active
        .lock()
        .map_err(|_| "subject selection lock poisoned".to_string())? = subject_id;
    Ok(())
}

/// A cross-subject prerequisite resolved to something the UI can show and link:
/// the target subject's name and the concept's label + practiceability. Resolving
/// this needs *all* subjects, so it lives here (the registry), not in a single
/// per-subject service.
#[derive(Serialize)]
struct ResolvedRef {
    subject_id: String,
    subject_name: String,
    concept_id: String,
    label: String,
    practiceable: bool,
}

#[tauri::command]
async fn resolve_refs(
    state: State<'_, AppState>,
    refs: Vec<ConceptRef>,
) -> Result<Vec<ResolvedRef>, String> {
    let mut out = Vec::new();
    for r in refs {
        if let Some(service) = state.subjects.get(r.subject.as_str()) {
            if let Some((label, practiceable)) = service.concept_brief(&r.concept) {
                out.push(ResolvedRef {
                    subject_id: r.subject.to_string(),
                    subject_name: service.subject_name().to_string(),
                    concept_id: r.concept.to_string(),
                    label,
                    practiceable,
                });
            }
        }
    }
    Ok(out)
}

#[tauri::command]
async fn subject_info(state: State<'_, AppState>) -> Result<SubjectInfo, String> {
    let service = state.active_service()?;
    Ok(SubjectInfo {
        id: service.subject_id().to_string(),
        name: service.subject_name().to_string(),
        groups: service.groups().to_vec(),
    })
}

#[tauri::command]
async fn concept_map(state: State<'_, AppState>) -> Result<Vec<ConceptStatus>, String> {
    state
        .active_service()?
        .concept_map(state.learner)
        .await
        .map_err(to_message)
}

#[tauri::command]
async fn next_problem(state: State<'_, AppState>) -> Result<Problem, String> {
    state
        .active_service()?
        .next_problem(state.learner)
        .await
        .map_err(to_message)
}

/// A cross-subject prerequisite that looks weak — surfaced after a wrong answer
/// so a failed Physics problem can point at the shaky Math skill underneath.
#[derive(Serialize)]
struct CrossWeakLink {
    subject_id: String,
    subject_name: String,
    concept_id: String,
    label: String,
    mastery: f32,
    practiceable: bool,
}

/// The attempt outcome plus any weak cross-subject prerequisites (resolved across
/// the whole subject registry — a single per-subject service can't see them).
#[derive(Serialize)]
struct SubmitView {
    #[serde(flatten)]
    outcome: AttemptOutcome,
    cross_weak_links: Vec<CrossWeakLink>,
}

/// Below this mastery, a cross-subject prerequisite is worth flagging.
const CROSS_WEAK_THRESHOLD: f32 = 0.6;

#[tauri::command]
async fn submit_attempt(
    state: State<'_, AppState>,
    problem_id: ProblemId,
    submitted_work: String,
) -> Result<SubmitView, String> {
    let outcome = state
        .active_service()?
        .submit_attempt(state.learner, problem_id, submitted_work)
        .await
        .map_err(to_message)?;

    // For each cross-subject prerequisite of the failed concept, check the
    // learner's mastery *in that subject* and surface the weak ones.
    let mut cross_weak_links = Vec::new();
    for r in &outcome.external_prerequisites {
        let Some(service) = state.subjects.get(r.subject.as_str()) else {
            continue;
        };
        let Some((label, practiceable)) = service.concept_brief(&r.concept) else {
            continue;
        };
        let mastery = service
            .concept_mastery(state.learner, &r.concept)
            .await
            .map_err(to_message)?
            .unwrap_or(0.0);
        if mastery < CROSS_WEAK_THRESHOLD {
            cross_weak_links.push(CrossWeakLink {
                subject_id: r.subject.to_string(),
                subject_name: service.subject_name().to_string(),
                concept_id: r.concept.to_string(),
                label,
                mastery,
                practiceable,
            });
        }
    }

    Ok(SubmitView {
        outcome,
        cross_weak_links,
    })
}

#[tauri::command]
async fn practice_concept(
    state: State<'_, AppState>,
    concept_id: ConceptId,
) -> Result<Problem, String> {
    state
        .active_service()?
        .practice_concept(state.learner, concept_id)
        .await
        .map_err(to_message)
}

// --- Learn: the "learn the concept" lessons (spec §2.2) ---
//
// Lessons are data — original Markdown+KaTeX under `subjects/<id>/notes/`. The
// service reads them fresh (so a save shows immediately) and can draft new ones
// with the same BYOK provider as diagnosis/generation.

#[tauri::command]
async fn lesson(state: State<'_, AppState>, concept_id: ConceptId) -> Result<Lesson, String> {
    state.active_service()?.lesson(&concept_id).map_err(to_message)
}

#[tauri::command]
async fn draft_lesson(
    state: State<'_, AppState>,
    concept_id: ConceptId,
) -> Result<String, String> {
    let settings = load_settings(&state.config_dir);
    let api_key = read_api_key(settings.provider)
        .ok_or_else(|| format!("No API key set for {}.", settings.provider.label()))?;
    let provider = ProviderConfig {
        provider: settings.provider,
        api_key,
        model: settings.model,
    };
    state
        .active_service()?
        .draft_lesson(&concept_id, &provider)
        .await
        .map_err(to_message)
}

#[tauri::command]
async fn save_lesson(
    state: State<'_, AppState>,
    concept_id: ConceptId,
    markdown: String,
) -> Result<(), String> {
    state
        .active_service()?
        .save_lesson(&concept_id, &markdown)
        .map_err(to_message)
}

#[tauri::command]
async fn model_params(state: State<'_, AppState>) -> Result<BktParams, String> {
    Ok(state.active_service()?.model_params())
}

#[tauri::command]
async fn refit_model(state: State<'_, AppState>) -> Result<BktParams, String> {
    state
        .active_service()?
        .refit_model(state.learner)
        .await
        .map_err(to_message)
}

// --- Phase 2: BYOK misconception diagnosis ---
//
// Keys live in the OS keychain (never on disk); the provider + model choice is a
// small JSON file in the app-data dir. The diagnosis crate sees neither store —
// the app reads both and hands it a ready `ProviderConfig`.

const KEYCHAIN_SERVICE: &str = "com.lattice.app";

#[derive(Serialize, Deserialize, Clone)]
struct DiagnosisSettings {
    provider: Provider,
    model: String,
}

impl Default for DiagnosisSettings {
    fn default() -> Self {
        Self {
            provider: Provider::Anthropic,
            model: Provider::Anthropic.default_model().to_string(),
        }
    }
}

fn settings_path(dir: &Path) -> PathBuf {
    dir.join("diagnosis-settings.json")
}

fn load_settings(dir: &Path) -> DiagnosisSettings {
    std::fs::read_to_string(settings_path(dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Keychain account name for a provider's key — its lowercase serde name.
fn keychain_account(provider: Provider) -> String {
    serde_json::to_value(provider)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "anthropic".to_string())
}

fn read_api_key(provider: Provider) -> Option<String> {
    let key = keyring::Entry::new(KEYCHAIN_SERVICE, &keychain_account(provider))
        .ok()?
        .get_password()
        .ok()?;
    (!key.is_empty()).then_some(key)
}

#[derive(Serialize)]
struct ProviderOption {
    id: Provider,
    label: String,
    default_model: String,
}

#[derive(Serialize)]
struct DiagnosisSettingsView {
    provider: Provider,
    model: String,
    has_key: bool,
    providers: Vec<ProviderOption>,
}

#[tauri::command]
async fn get_diagnosis_settings(state: State<'_, AppState>) -> Result<DiagnosisSettingsView, String> {
    let settings = load_settings(&state.config_dir);
    let has_key = read_api_key(settings.provider).is_some();
    let providers = [Provider::Anthropic, Provider::OpenAi, Provider::Gemini]
        .into_iter()
        .map(|p| ProviderOption {
            id: p,
            label: p.label().to_string(),
            default_model: p.default_model().to_string(),
        })
        .collect();
    Ok(DiagnosisSettingsView {
        provider: settings.provider,
        model: settings.model,
        has_key,
        providers,
    })
}

#[tauri::command]
async fn set_diagnosis_settings(
    state: State<'_, AppState>,
    provider: Provider,
    model: String,
) -> Result<(), String> {
    let model = if model.trim().is_empty() {
        provider.default_model().to_string()
    } else {
        model
    };
    let json = serde_json::to_string_pretty(&DiagnosisSettings { provider, model })
        .map_err(|e| e.to_string())?;
    std::fs::write(settings_path(&state.config_dir), json).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_api_key(provider: Provider, key: String) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &keychain_account(provider))
        .map_err(|e| e.to_string())?;
    if key.trim().is_empty() {
        let _ = entry.delete_credential();
        Ok(())
    } else {
        entry.set_password(&key).map_err(|e| e.to_string())
    }
}

#[tauri::command]
async fn diagnose_attempt(
    state: State<'_, AppState>,
    attempt_id: AttemptId,
    problem_id: ProblemId,
    submitted_work: String,
) -> Result<Diagnosis, String> {
    let settings = load_settings(&state.config_dir);
    let api_key = read_api_key(settings.provider)
        .ok_or_else(|| format!("No API key set for {}.", settings.provider.label()))?;
    let provider = ProviderConfig {
        provider: settings.provider,
        api_key,
        model: settings.model,
    };
    state
        .active_service()?
        .diagnose_attempt(attempt_id, problem_id, &submitted_work, &provider)
        .await
        .map_err(to_message)
}

#[tauri::command]
async fn explain_problem(
    state: State<'_, AppState>,
    problem_id: ProblemId,
) -> Result<String, String> {
    let settings = load_settings(&state.config_dir);
    let api_key = read_api_key(settings.provider)
        .ok_or_else(|| format!("No API key set for {}.", settings.provider.label()))?;
    let provider = ProviderConfig {
        provider: settings.provider,
        api_key,
        model: settings.model,
    };
    state
        .active_service()?
        .explain_problem(problem_id, &provider)
        .await
        .map_err(to_message)
}

#[tauri::command]
async fn generate_ai_problem(
    state: State<'_, AppState>,
    concept_id: ConceptId,
    difficulty: Difficulty,
) -> Result<Problem, String> {
    let settings = load_settings(&state.config_dir);
    let api_key = read_api_key(settings.provider)
        .ok_or_else(|| format!("No API key set for {}.", settings.provider.label()))?;
    let provider = ProviderConfig {
        provider: settings.provider,
        api_key,
        model: settings.model,
    };
    state
        .active_service()?
        .generate_ai_problem(state.learner, concept_id, difficulty, &provider)
        .await
        .map_err(to_message)
}

/// The root holding every subject directory. For V1/dev it sits next to
/// `src-tauri` in the repo; bundling it as a Tauri resource is a release-time
/// follow-up.
fn subjects_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../subjects")
}

/// Every subject directory under [`subjects_root`] — any subdirectory that has a
/// `concepts.json`. Sorted for a stable switcher order.
fn discover_subject_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(subjects_root())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("concepts.json").is_file())
        .collect();
    dirs.sort();
    dirs
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("resolve app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            // Bootstrap one service per subject, each with its own SQLite file so
            // subjects stay fully isolated (no concept-id collisions across
            // subjects). The service init is async; run it to completion on
            // Tauri's runtime before the first window paints.
            let mut subjects = HashMap::new();
            let mut order = Vec::new();
            for dir in discover_subject_dirs() {
                let dir_name = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "subject".to_string());
                let db_path = data_dir.join(format!("lattice-{dir_name}.db"));
                let service = tauri::async_runtime::block_on(LatticeService::bootstrap(
                    &dir, db_path,
                ))
                .unwrap_or_else(|e| panic!("initialize subject `{dir_name}`: {e}"));
                let id = service.subject_id().to_string();
                order.push(SubjectSummary {
                    id: id.clone(),
                    name: service.subject_name().to_string(),
                });
                subjects.insert(id, service);
            }
            let active = order
                .first()
                .map(|s| s.id.clone())
                .expect("at least one subject under subjects/");

            app.manage(AppState {
                subjects,
                order,
                active: Mutex::new(active),
                learner: SOLO_LEARNER,
                config_dir: data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_subjects,
            select_subject,
            resolve_refs,
            subject_info,
            concept_map,
            next_problem,
            submit_attempt,
            practice_concept,
            lesson,
            draft_lesson,
            save_lesson,
            model_params,
            refit_model,
            get_diagnosis_settings,
            set_diagnosis_settings,
            set_api_key,
            diagnose_attempt,
            explain_problem,
            generate_ai_problem
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
