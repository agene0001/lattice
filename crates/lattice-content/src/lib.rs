//! `lattice-content` — turning a subject's static data into concrete problems.
//!
//! Two responsibilities, both V1:
//!   * [`subject`] — load a subject's concept graph + templates from
//!     `subjects/<id>/` (data, not code — the lever for Pillar 6, spec §2.6).
//!   * [`template`] — the parameterized template engine (spec §2.3). Generation
//!     is deliberately *not* AI here; it's deterministic and verifiable, which
//!     matters because an unsolvable math problem is a silent, real failure mode.
//!
//! AI-backed generation (Phase 3) layers on top of this behind the same
//! correctness discipline — it does not replace it (spec §2.3).

pub mod generate;
pub mod lesson;
pub mod subject;
pub mod template;

pub use generate::{generate_problem, GenError};
pub use lesson::draft_lesson;
pub use subject::{load_subject, LoadError, Subject};
pub use template::{Instance, Template, TemplateKind};
