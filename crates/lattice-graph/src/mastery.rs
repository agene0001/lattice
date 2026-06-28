//! Pluggable learner models — the ML/DL track (spec §2.2, open Q4).
//!
//! A [`MasteryModel`] owns both halves of the learner model:
//!   * `estimated_mastery` — the read-time belief (used for diagnosis/display),
//!   * `update` — how one graded attempt changes that belief.
//!
//! V1 ships two:
//!   * [`ExponentialDecay`] — a closed-form forgetting curve + a proportional
//!     update nudge. No training data required.
//!   * [`Bkt`] — Bayesian Knowledge Tracing, the classic interpretable
//!     knowledge-tracing model and the first rung of the ML ladder
//!     (BKT → IRT → Deep Knowledge Tracing). It treats each skill as a two-state
//!     HMM and updates `P(known)` by Bayes on every attempt.
//!
//! Both implement the same trait, so switching models is a one-line change at the
//! service boundary — and a *learned* model (fit BKT params by EM, or train a DKT
//! LSTM on the `attempts` log) drops in here without touching call sites.

use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use lattice_core::{ConceptId, ConceptMastery, MasteryState};
use serde::{Deserialize, Serialize};

/// How far a correct answer moves [`ExponentialDecay`] confidence toward 1.0.
const LEARN_RATE: f32 = 0.5;
/// How much an incorrect answer multiplies [`ExponentialDecay`] confidence down.
const FORGET_FACTOR: f32 = 0.4;
/// Per-day forgetting rate assigned to a freshly-seen concept.
const DEFAULT_DECAY_RATE: f32 = 0.02;

/// Map a continuous confidence onto the coarse, displayable [`MasteryState`].
fn state_for(confidence: f32) -> MasteryState {
    if confidence >= 0.85 {
        MasteryState::Mastered
    } else if confidence >= 0.6 {
        MasteryState::Familiar
    } else if confidence >= 0.3 {
        MasteryState::Rusty
    } else {
        MasteryState::Forgotten
    }
}

/// Shared exponential forgetting curve: `confidence · e^(-rate · days)`.
fn decayed(m: &ConceptMastery, rate: f32, now: DateTime<Utc>) -> f32 {
    let days = (now - m.last_practiced_at).num_seconds().max(0) as f32 / 86_400.0;
    (m.confidence * (-rate * days).exp()).clamp(0.0, 1.0)
}

/// The learner model: turns stored mastery + observed attempts into a live
/// mastery belief. The one seam where a trained model replaces a hand rule.
pub trait MasteryModel {
    /// Current decay-adjusted mastery in `[0, 1]` (read-time).
    fn estimated_mastery(&self, mastery: &ConceptMastery, now: DateTime<Utc>) -> f32;

    /// The new mastery after observing one graded attempt (write-time).
    fn update(
        &self,
        prior: Option<&ConceptMastery>,
        concept: &ConceptId,
        correct: bool,
        now: DateTime<Utc>,
    ) -> ConceptMastery;
}

/// V1 closed-form model: exponential decay at read time, a proportional nudge at
/// write time. Deterministic, needs no training data (spec §2.2, open Q4).
#[derive(Debug, Clone, Copy, Default)]
pub struct ExponentialDecay;

impl MasteryModel for ExponentialDecay {
    fn estimated_mastery(&self, m: &ConceptMastery, now: DateTime<Utc>) -> f32 {
        decayed(m, m.decay_rate, now)
    }

    fn update(
        &self,
        prior: Option<&ConceptMastery>,
        concept: &ConceptId,
        correct: bool,
        now: DateTime<Utc>,
    ) -> ConceptMastery {
        let prior_estimate = prior.map_or(0.0, |m| self.estimated_mastery(m, now));
        let confidence = if correct {
            prior_estimate + (1.0 - prior_estimate) * LEARN_RATE
        } else {
            prior_estimate * FORGET_FACTOR
        }
        .clamp(0.0, 1.0);

        ConceptMastery {
            concept_id: concept.clone(),
            state: state_for(confidence),
            confidence,
            last_practiced_at: now,
            decay_rate: prior.map_or(DEFAULT_DECAY_RATE, |m| m.decay_rate),
        }
    }
}

/// **Bayesian Knowledge Tracing** — the first real ML model in the stack.
///
/// Each skill is a two-state HMM (known / not-known). We keep `confidence` =
/// `P(known)` and, on every graded attempt, (1) apply Bayes given the observation
/// — tempered by *slip* (`P(wrong | known)`) and *guess* (`P(correct | ¬known)`)
/// — then (2) apply the learning transition (a chance the skill was acquired this
/// opportunity).
///
/// V1 hybrid: classic BKT assumes no forgetting, but decay is Lattice's whole
/// premise (spec §2.2), so `estimated_mastery` still applies an exponential decay
/// at read time. Parameters are fixed here; **fitting them from the `attempts`
/// log (EM or gradient descent) is the natural next ML exercise**, and the rung
/// after that is a Deep Knowledge Tracing LSTM implementing this same trait.
#[derive(Debug, Clone)]
pub struct Bkt {
    // Behind a lock so the parameters can be refit and applied at runtime
    // (Rung 2) while the model is shared as immutable `&self` everywhere.
    params: Arc<RwLock<BktParams>>,
}

impl Bkt {
    pub fn new(params: BktParams) -> Self {
        Self {
            params: Arc::new(RwLock::new(params)),
        }
    }

    /// Snapshot of the current parameters.
    pub fn params(&self) -> BktParams {
        *self.params.read().expect("bkt params lock poisoned")
    }

    /// Replace the parameters (e.g. after refitting); the shared handle is kept,
    /// so the live model adapts immediately.
    pub fn set_params(&self, params: BktParams) {
        *self.params.write().expect("bkt params lock poisoned") = params;
    }

    /// Fit `init/learn/slip/guess` to attempt sequences and apply them in place
    /// (Rung 2); `decay_rate` is preserved. Returns the fitted parameters.
    pub fn fit(&self, sequences: &[Vec<bool>]) -> BktParams {
        let mut fitted = BktParams::fit(sequences);
        fitted.decay_rate = self.params().decay_rate;
        self.set_params(fitted);
        fitted
    }
}

/// BKT parameters. The default is a reasonable prior; [`BktParams::fit`] learns
/// them from attempt data (Rung 2).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BktParams {
    /// `P(known)` before any practice.
    pub p_init: f32,
    /// `P(¬known → known)` per opportunity.
    pub p_learn: f32,
    /// `P(wrong | known)` — a slip.
    pub p_slip: f32,
    /// `P(correct | ¬known)` — a guess.
    pub p_guess: f32,
    /// Read-time forgetting rate (the hybrid term; classic BKT has none).
    pub decay_rate: f32,
}

impl Default for BktParams {
    fn default() -> Self {
        Self {
            p_init: 0.2,
            p_learn: 0.3,
            p_slip: 0.1,
            p_guess: 0.2,
            decay_rate: DEFAULT_DECAY_RATE,
        }
    }
}

impl BktParams {
    /// **Rung 2 — fit the four BKT parameters to observed correct/incorrect
    /// sequences by maximum likelihood.** Instead of the hardcoded defaults, find
    /// the `init/learn/slip/guess` that best explain the learner's real attempt
    /// history. `decay_rate` is not part of the BKT likelihood, so it's left at
    /// the default here.
    ///
    /// Optimisation is coordinate ascent over a fine grid: cheap, dependency-free,
    /// and easy to reason about. `slip` and `guess` are constrained below 0.5 so
    /// the "known" state can't swap meaning (BKT's identifiability condition).
    pub fn fit(sequences: &[Vec<bool>]) -> Self {
        let mut params = BktParams::default();
        let observations: usize = sequences.iter().map(|s| s.len()).sum();
        if observations == 0 {
            return params; // nothing to learn from yet
        }

        let grid: Vec<f32> = (1..=99).map(|i| i as f32 / 100.0).collect();
        let bounded: Vec<f32> = grid.iter().copied().filter(|&v| v < 0.5).collect();

        for _ in 0..6 {
            params.p_init = best_param(&grid, sequences, params, |p, v| p.p_init = v);
            params.p_learn = best_param(&grid, sequences, params, |p, v| p.p_learn = v);
            params.p_slip = best_param(&bounded, sequences, params, |p, v| p.p_slip = v);
            params.p_guess = best_param(&bounded, sequences, params, |p, v| p.p_guess = v);
        }
        params
    }

    /// Total log-likelihood of `sequences` under these parameters.
    pub fn log_likelihood(&self, sequences: &[Vec<bool>]) -> f64 {
        sequences
            .iter()
            .map(|s| self.sequence_log_likelihood(s))
            .sum()
    }

    /// BKT forward pass for one sequence: at each opportunity, accumulate the
    /// marginal likelihood of the observation, then Bayes-update and apply the
    /// learning transition — the same recursion as [`Bkt::update`].
    fn sequence_log_likelihood(&self, seq: &[bool]) -> f64 {
        let (s, g, t) = (self.p_slip as f64, self.p_guess as f64, self.p_learn as f64);
        let mut p_known = self.p_init as f64;
        let mut ll = 0.0;
        for &correct in seq {
            let p_obs = if correct {
                p_known * (1.0 - s) + (1.0 - p_known) * g
            } else {
                p_known * s + (1.0 - p_known) * (1.0 - g)
            }
            .max(1e-12);
            ll += p_obs.ln();

            let posterior = if correct {
                p_known * (1.0 - s) / p_obs
            } else {
                p_known * s / p_obs
            };
            p_known = posterior + (1.0 - posterior) * t;
        }
        ll
    }
}

/// Line search: the grid value (with one field overwritten by `set`) that
/// maximises the data log-likelihood.
fn best_param<F: Fn(&mut BktParams, f32)>(
    grid: &[f32],
    sequences: &[Vec<bool>],
    base: BktParams,
    set: F,
) -> f32 {
    let mut best_v = grid[0];
    let mut best_ll = f64::NEG_INFINITY;
    for &v in grid {
        let mut p = base;
        set(&mut p, v);
        let ll = p.log_likelihood(sequences);
        if ll > best_ll {
            best_ll = ll;
            best_v = v;
        }
    }
    best_v
}

impl Default for Bkt {
    fn default() -> Self {
        Self::new(BktParams::default())
    }
}

impl MasteryModel for Bkt {
    fn estimated_mastery(&self, m: &ConceptMastery, now: DateTime<Utc>) -> f32 {
        decayed(m, self.params().decay_rate, now)
    }

    fn update(
        &self,
        prior: Option<&ConceptMastery>,
        concept: &ConceptId,
        correct: bool,
        now: DateTime<Utc>,
    ) -> ConceptMastery {
        let p = self.params();
        // Start from the decayed prior belief, so forgetting feeds the update.
        let prior_known = prior
            .map_or(p.p_init, |m| self.estimated_mastery(m, now))
            .clamp(0.0, 1.0);

        // (1) Bayes: P(known | observation), tempered by slip and guess.
        let posterior = if correct {
            let num = prior_known * (1.0 - p.p_slip);
            let den = num + (1.0 - prior_known) * p.p_guess;
            if den > 0.0 {
                num / den
            } else {
                prior_known
            }
        } else {
            let num = prior_known * p.p_slip;
            let den = num + (1.0 - prior_known) * (1.0 - p.p_guess);
            if den > 0.0 {
                num / den
            } else {
                prior_known
            }
        };

        // (2) Learning transition.
        let known_now = (posterior + (1.0 - posterior) * p.p_learn).clamp(0.0, 1.0);

        ConceptMastery {
            concept_id: concept.clone(),
            state: state_for(known_now),
            confidence: known_now,
            last_practiced_at: now,
            decay_rate: p.decay_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    #[test]
    fn bkt_rises_with_correct_and_falls_with_incorrect() {
        let bkt = Bkt::default();
        let now = Utc::now();
        let c = ConceptId::new("x");

        // One correct answer from scratch already beats the prior.
        let mut m = bkt.update(None, &c, true, now);
        assert!(m.confidence > bkt.params().p_init);

        // A streak drives toward mastery.
        for _ in 0..6 {
            m = bkt.update(Some(&m), &c, true, now);
        }
        assert!(m.confidence > 0.9, "got {}", m.confidence);
        assert_eq!(m.state, MasteryState::Mastered);

        // A wrong answer knocks it back down (but slip means not to zero).
        let after_wrong = bkt.update(Some(&m), &c, false, now);
        assert!(after_wrong.confidence < m.confidence);
    }

    #[test]
    fn bkt_estimate_decays_over_time() {
        let bkt = Bkt::default();
        let now = Utc::now();
        let m = ConceptMastery {
            concept_id: ConceptId::new("x"),
            state: MasteryState::Mastered,
            confidence: 0.95,
            last_practiced_at: now - Duration::days(60),
            decay_rate: bkt.params().decay_rate,
        };
        assert!(bkt.estimated_mastery(&m, now) < 0.95);
    }

    #[test]
    fn exponential_decay_rewards_correct_over_incorrect() {
        let model = ExponentialDecay;
        let now = Utc::now();
        let c = ConceptId::new("x");
        let correct = model.update(None, &c, true, now);
        let wrong = model.update(None, &c, false, now);
        assert!(correct.confidence > wrong.confidence);
    }

    /// Generate one correct/incorrect sequence from the BKT generative process.
    fn simulate(p: &BktParams, length: usize, rng: &mut impl Rng) -> Vec<bool> {
        let mut known = rng.random_bool(p.p_init as f64);
        let mut seq = Vec::with_capacity(length);
        for _ in 0..length {
            let correct = if known {
                rng.random_bool(1.0 - p.p_slip as f64)
            } else {
                rng.random_bool(p.p_guess as f64)
            };
            seq.push(correct);
            // Unknown → known with probability p_learn; "known" is absorbing.
            if !known {
                known = rng.random_bool(p.p_learn as f64);
            }
        }
        seq
    }

    #[test]
    fn bkt_fit_recovers_synthetic_parameters() {
        // Generate data from known parameters, then check `fit` recovers them.
        let truth = BktParams {
            p_init: 0.25,
            p_learn: 0.25,
            p_slip: 0.08,
            p_guess: 0.20,
            decay_rate: 0.02,
        };
        let mut rng = StdRng::seed_from_u64(7);
        let sequences: Vec<Vec<bool>> = (0..400).map(|_| simulate(&truth, 14, &mut rng)).collect();

        let fitted = BktParams::fit(&sequences);

        // Emission params (slip/guess) are well identified; learn a bit looser.
        assert!((fitted.p_slip - truth.p_slip).abs() < 0.10, "slip={}", fitted.p_slip);
        assert!((fitted.p_guess - truth.p_guess).abs() < 0.10, "guess={}", fitted.p_guess);
        assert!((fitted.p_learn - truth.p_learn).abs() < 0.15, "learn={}", fitted.p_learn);
        // MLE property: the fit explains the data at least as well as the truth.
        assert!(fitted.log_likelihood(&sequences) >= truth.log_likelihood(&sequences) - 1e-6);
    }
}
