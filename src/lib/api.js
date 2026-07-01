// Thin wrappers over the Tauri IPC commands defined in `src-tauri/src/lib.rs`.
// Tauri maps these camelCase JS keys to the snake_case Rust parameters.
import { invoke } from '@tauri-apps/api/core';

// --- Subjects (multi-subject: the app boots one service per subjects/<id>/) ---
export const listSubjects = () => invoke('list_subjects');
export const selectSubject = (subjectId) => invoke('select_subject', { subjectId });
// Resolve cross-subject prerequisite refs to {subject_id, subject_name, concept_id, label, practiceable}.
export const resolveRefs = (refs) => invoke('resolve_refs', { refs });
export const subjectInfo = () => invoke('subject_info');
export const conceptMap = () => invoke('concept_map');
export const nextProblem = () => invoke('next_problem');
export const submitAttempt = (problemId, submittedWork) =>
  invoke('submit_attempt', { problemId, submittedWork });
export const practiceConcept = (conceptId) => invoke('practice_concept', { conceptId });

// --- Learn: "learn the concept" lessons (Markdown + KaTeX, authored as data) ---
export const lesson = (conceptId) => invoke('lesson', { conceptId });
export const draftLesson = (conceptId) => invoke('draft_lesson', { conceptId });
export const saveLesson = (conceptId, markdown) => invoke('save_lesson', { conceptId, markdown });

// --- Phase 2: AI misconception diagnosis (BYOK) ---
export const getDiagnosisSettings = () => invoke('get_diagnosis_settings');
export const setDiagnosisSettings = (provider, model) =>
  invoke('set_diagnosis_settings', { provider, model });
export const setApiKey = (provider, key) => invoke('set_api_key', { provider, key });
export const diagnoseAttempt = (attemptId, problemId, submittedWork) =>
  invoke('diagnose_attempt', { attemptId, problemId, submittedWork });

// --- Phase 3: AI-generated practice problems ---
export const generateAiProblem = (conceptId, difficulty) =>
  invoke('generate_ai_problem', { conceptId, difficulty });
export const modelParams = () => invoke('model_params');
export const refitModel = () => invoke('refit_model');
