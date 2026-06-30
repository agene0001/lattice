//! Verification (spec §10.4).
//!
//! Before an imported problem joins the active practice pool, its stated answer
//! is checked. V1 reuses the generation pipeline's discipline: an **independent
//! LLM re-solve**, accepted only if it agrees with the source's answer via
//! [`lattice_core::answers_match`]. This catches extraction slips (a mis-parsed
//! `\boxed{}`) and answer-key errors without trusting the source blindly.
//!
//! A deterministic CAS check (SymPy, §10.4) is the stronger follow-up — it would
//! verify symbolic algebra/calculus without a second model call, and shares the
//! §12.6 answer-leak machinery. The re-solve is the dependency-free V1 stand-in.

use lattice_core::answers_match;
use lattice_llm::{complete, ProviderConfig};

use crate::ImportError;

/// Independently re-solve `content` and report whether the result agrees with
/// `stated_solution`. A disagreement means *do not trust this import yet*, not
/// necessarily that the problem is wrong — it's routed to manual review.
pub async fn verify_solution(
    config: &ProviderConfig,
    content: &str,
    stated_solution: &str,
) -> Result<bool, ImportError> {
    let system = "You are solving a math problem. Reply with ONLY the final \
        answer — no working, no prose.";
    let got = complete(config, system, content).await?;
    Ok(answers_match(stated_solution, &got))
}
