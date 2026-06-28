//! Tauri shell for Lattice.
//!
//! This crate is a *thin adapter* (spec §9): it owns no domain logic. It boots a
//! [`LatticeService`] at startup, holds it as managed state, and exposes a
//! handful of `#[tauri::command]`s that forward to the service. A future
//! `lattice-api` (Axum) would be the same forwarding over HTTP instead of IPC.

use std::path::{Path, PathBuf};

use lattice_core::{AttemptId, ConceptId, Diagnosis, Difficulty, LearnerId, Problem, ProblemId};
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

struct AppState {
    service: Service,
    learner: LearnerId,
    config_dir: PathBuf,
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

#[tauri::command]
async fn subject_info(state: State<'_, AppState>) -> Result<SubjectInfo, String> {
    Ok(SubjectInfo {
        id: state.service.subject_id().to_string(),
        name: state.service.subject_name().to_string(),
        groups: state.service.groups().to_vec(),
    })
}

#[tauri::command]
async fn concept_map(state: State<'_, AppState>) -> Result<Vec<ConceptStatus>, String> {
    state
        .service
        .concept_map(state.learner)
        .await
        .map_err(to_message)
}

#[tauri::command]
async fn next_problem(state: State<'_, AppState>) -> Result<Problem, String> {
    state
        .service
        .next_problem(state.learner)
        .await
        .map_err(to_message)
}

#[tauri::command]
async fn submit_attempt(
    state: State<'_, AppState>,
    problem_id: ProblemId,
    submitted_work: String,
) -> Result<AttemptOutcome, String> {
    state
        .service
        .submit_attempt(state.learner, problem_id, submitted_work)
        .await
        .map_err(to_message)
}

#[tauri::command]
async fn practice_concept(
    state: State<'_, AppState>,
    concept_id: ConceptId,
) -> Result<Problem, String> {
    state
        .service
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
    state.service.lesson(&concept_id).map_err(to_message)
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
        .service
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
        .service
        .save_lesson(&concept_id, &markdown)
        .map_err(to_message)
}

#[tauri::command]
async fn model_params(state: State<'_, AppState>) -> Result<BktParams, String> {
    Ok(state.service.model_params())
}

#[tauri::command]
async fn refit_model(state: State<'_, AppState>) -> Result<BktParams, String> {
    state
        .service
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
        .service
        .diagnose_attempt(attempt_id, problem_id, &submitted_work, &provider)
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
        .service
        .generate_ai_problem(state.learner, concept_id, difficulty, &provider)
        .await
        .map_err(to_message)
}

/// Where the subject data lives. For V1/dev it sits next to `src-tauri` in the
/// repo; bundling it as a Tauri resource is a release-time follow-up.
fn subject_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../subjects/math")
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
            let db_path = data_dir.join("lattice.db");

            // The service init is async (SQLite connect + migrate); run it to
            // completion on Tauri's runtime before the first window paints.
            let service = tauri::async_runtime::block_on(LatticeService::bootstrap(
                subject_dir(),
                db_path,
            ))
            .expect("initialize lattice service");

            app.manage(AppState {
                service,
                learner: SOLO_LEARNER,
                config_dir: data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            generate_ai_problem
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
