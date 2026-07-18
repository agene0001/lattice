//! `lattice-graph` — the deterministic diagnosis engine (spec §2.1–§2.2, §5).
//!
//! No AI lives here, by design: this is "the cheapest, most validate-able part
//! of the product" (spec §2.1) and ships standalone in V1. Given a concept DAG
//! and a learner's mastery state it answers two questions:
//!
//!   * **Remediation** — when a problem is failed, which prerequisite is the
//!     weak link actually responsible? ([`find_weakest_prerequisite`])
//!   * **Progress** — which concepts is the learner ready to learn next?
//!     ([`ready_frontier`])
//!
//! Mastery estimation is deliberately behind the [`MasteryModel`] trait. V1 uses
//! [`ExponentialDecay`]; this is the seam where a *learned* model (knowledge
//! tracing) can replace the closed-form decay later (spec §2.2, open Q4).

use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use lattice_core::{Concept, ConceptId, ConceptMastery, MasteryState, Problem};
use serde::{Deserialize, Serialize};

mod mastery;
pub use mastery::{Bkt, BktParams, ExponentialDecay, MasteryModel};

/// Default mastery cutoff: at or above this estimated mastery, a concept is
/// treated as "known well enough" — not a suspect in diagnosis, and solid enough
/// to unlock the concepts that depend on it.
pub const MASTERY_THRESHOLD: f32 = 0.7;

/// The concept DAG for one subject. Cheap to build, cheap to query.
#[derive(Debug, Clone)]
pub struct ConceptGraph {
    concepts: HashMap<ConceptId, Concept>,
}

/// Errors from validating authored subject data.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("concept graph contains a cycle through `{0}`")]
    Cycle(ConceptId),
    #[error("concept `{concept}` lists unknown prerequisite `{prerequisite}`")]
    UnknownPrerequisite {
        concept: ConceptId,
        prerequisite: ConceptId,
    },
}

impl ConceptGraph {
    pub fn new(concepts: impl IntoIterator<Item = Concept>) -> Self {
        Self {
            concepts: concepts.into_iter().map(|c| (c.id.clone(), c)).collect(),
        }
    }

    pub fn get(&self, id: &ConceptId) -> Option<&Concept> {
        self.concepts.get(id)
    }

    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    pub fn concepts(&self) -> impl Iterator<Item = &Concept> {
        self.concepts.values()
    }

    /// Validate that every prerequisite resolves and the graph is acyclic.
    ///
    /// A concept graph is a DAG by definition (spec §2.1); a cycle or a dangling
    /// edge is an authoring error we want to catch when *loading* subject data,
    /// not at query time.
    pub fn validate(&self) -> Result<(), GraphError> {
        for c in self.concepts.values() {
            for p in &c.prerequisites {
                if !self.concepts.contains_key(p) {
                    return Err(GraphError::UnknownPrerequisite {
                        concept: c.id.clone(),
                        prerequisite: p.clone(),
                    });
                }
            }
        }
        self.check_acyclic()
    }

    /// Iterative depth-first search with a three-state marking (unvisited /
    /// visiting / done). Encountering a `Visiting` node along the current path is
    /// a back-edge, i.e. a cycle. Iterative rather than recursive so a deep graph
    /// can't blow the stack.
    fn check_acyclic(&self) -> Result<(), GraphError> {
        #[derive(Clone, Copy)]
        enum Mark {
            Visiting,
            Done,
        }
        let mut marks: HashMap<ConceptId, Mark> = HashMap::new();

        for start in self.concepts.keys() {
            if marks.contains_key(start) {
                continue;
            }
            // Stack of (node, index of next prerequisite to explore).
            let mut stack: Vec<(ConceptId, usize)> = vec![(start.clone(), 0)];
            marks.insert(start.clone(), Mark::Visiting);

            while let Some((node, idx)) = stack.last().cloned() {
                let prereqs = &self.concepts[&node].prerequisites;
                if idx < prereqs.len() {
                    stack.last_mut().unwrap().1 += 1;
                    let next = prereqs[idx].clone();
                    match marks.get(&next).copied() {
                        Some(Mark::Visiting) => return Err(GraphError::Cycle(next)),
                        Some(Mark::Done) => {}
                        None => {
                            marks.insert(next.clone(), Mark::Visiting);
                            stack.push((next, 0));
                        }
                    }
                } else {
                    marks.insert(node, Mark::Done);
                    stack.pop();
                }
            }
        }
        Ok(())
    }

    /// Transitive prerequisite closure of `seeds`, *including* the seeds.
    ///
    /// Returned sorted, so any downstream argmin tie-break is deterministic.
    /// Safe on a DAG; the `seen` set also makes it safe if the graph hasn't been
    /// validated yet.
    pub fn prerequisite_closure(&self, seeds: &[ConceptId]) -> Vec<ConceptId> {
        let mut seen: HashSet<ConceptId> = HashSet::new();
        let mut queue: VecDeque<ConceptId> = seeds.iter().cloned().collect();
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(c) = self.concepts.get(&id) {
                for p in &c.prerequisites {
                    queue.push_back(p.clone());
                }
            }
        }
        let mut out: Vec<ConceptId> = seen.into_iter().collect();
        out.sort();
        out
    }
}

/// The suspected weak link behind a failure, with *why* it's suspected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeakLink {
    pub concept_id: ConceptId,
    pub estimated_mastery: f32,
    pub reason: WeakReason,
}

/// The distinction the spec (§2.2) insists on making explicit: "never learned",
/// "learned then quietly decayed", and "currently still weak" are different
/// situations that call for different remediation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeakReason {
    /// No mastery record at all — never practiced.
    NeverLearned,
    /// Was previously `Mastered`/`Familiar` but estimated mastery has decayed
    /// below threshold — the "eroded prerequisite nobody told you about" case.
    Decayed,
    /// Has a record and is below threshold, but was never strong — genuinely
    /// still being learned.
    Weak,
}

/// Trace a failed `problem` to the single weakest concept it rests on.
///
/// Considers the problem's tagged concepts *and their entire prerequisite
/// closure*, so the diagnosis can point two levels down at the real root cause
/// (a decayed factoring skill) rather than the surface topic the problem was
/// filed under (integration) — the core move described in spec §2.1.
///
/// Returns `None` only if the problem is tagged with no concepts.
pub fn find_weakest_prerequisite(
    graph: &ConceptGraph,
    problem: &Problem,
    masteries: &HashMap<ConceptId, ConceptMastery>,
    model: &impl MasteryModel,
    now: DateTime<Utc>,
) -> Option<WeakLink> {
    graph
        .prerequisite_closure(&problem.concepts)
        .into_iter()
        .map(|id| {
            let mastery = masteries.get(&id);
            let estimated_mastery = mastery.map_or(0.0, |m| model.estimated_mastery(m, now));
            WeakLink {
                reason: classify(mastery, estimated_mastery),
                concept_id: id,
                estimated_mastery,
            }
        })
        // Weakest = lowest estimated mastery. The closure is sorted, so ties
        // resolve deterministically to the first concept id.
        .min_by(|a, b| {
            a.estimated_mastery
                .partial_cmp(&b.estimated_mastery)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn classify(mastery: Option<&ConceptMastery>, estimated: f32) -> WeakReason {
    match mastery {
        None => WeakReason::NeverLearned,
        Some(m) => {
            let was_strong = matches!(m.state, MasteryState::Mastered | MasteryState::Familiar);
            if was_strong && estimated < MASTERY_THRESHOLD {
                WeakReason::Decayed
            } else {
                WeakReason::Weak
            }
        }
    }
}

/// The "ready to learn" frontier: concepts not yet mastered, but whose direct
/// prerequisites all are (spec §5, `next_ready_concept`). Drives forward
/// progress, not just remediation. Returned sorted for stable display.
pub fn ready_frontier(
    graph: &ConceptGraph,
    masteries: &HashMap<ConceptId, ConceptMastery>,
    model: &impl MasteryModel,
    now: DateTime<Utc>,
) -> Vec<ConceptId> {
    let estimate = |id: &ConceptId| -> f32 {
        masteries
            .get(id)
            .map_or(0.0, |m| model.estimated_mastery(m, now))
    };

    let mut frontier: Vec<ConceptId> = graph
        .concepts()
        // Not yet mastered → there's something to learn here.
        .filter(|c| estimate(&c.id) < MASTERY_THRESHOLD)
        // Every prerequisite is solid → it's actually unlocked.
        .filter(|c| c.prerequisites.iter().all(|p| estimate(p) >= MASTERY_THRESHOLD))
        .map(|c| c.id.clone())
        .collect();
    frontier.sort();
    frontier
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use lattice_core::{Difficulty, ProblemId, ProblemSource, SubjectId};

    fn concept(id: &str, prereqs: &[&str]) -> Concept {
        Concept {
            id: ConceptId::new(id),
            subject_id: SubjectId::new("math"),
            label: id.replace('_', " "),
            group: "test".to_string(),
            notes: None,
            prerequisites: prereqs.iter().map(|p| ConceptId::new(*p)).collect(),
            external_prerequisites: Vec::new(),
        }
    }

    fn mastery(
        id: &str,
        state: MasteryState,
        confidence: f32,
        days_ago: i64,
        decay_rate: f32,
    ) -> (ConceptId, ConceptMastery) {
        let concept_id = ConceptId::new(id);
        (
            concept_id.clone(),
            ConceptMastery {
                concept_id,
                state,
                confidence,
                decay_rate,
                last_practiced_at: Utc::now() - Duration::days(days_ago),
            },
        )
    }

    /// The spec's running example (§2.1): a slice of the calc prerequisite chain.
    fn calc_graph() -> ConceptGraph {
        ConceptGraph::new([
            concept("algebra", &[]),
            concept("factoring", &["algebra"]),
            concept("difference_of_squares", &["factoring"]),
            concept("partial_fractions", &["factoring"]),
            concept("integration_techniques", &["partial_fractions"]),
        ])
    }

    /// A learner who is solid on the basics, actively learning integration, but
    /// whose factoring quietly decayed ~14 months ago (the §2.2 scenario).
    fn decayed_factoring_learner() -> HashMap<ConceptId, ConceptMastery> {
        [
            mastery("algebra", MasteryState::Mastered, 0.95, 5, 0.01),
            mastery("factoring", MasteryState::Mastered, 0.90, 400, 0.01), // → ~0.016
            mastery("partial_fractions", MasteryState::Familiar, 0.80, 7, 0.01),
            mastery("integration_techniques", MasteryState::Familiar, 0.65, 3, 0.01),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn calc_graph_is_a_valid_dag() {
        assert!(calc_graph().validate().is_ok());
    }

    #[test]
    fn validate_rejects_a_cycle() {
        let g = ConceptGraph::new([concept("a", &["b"]), concept("b", &["a"])]);
        assert!(matches!(g.validate(), Err(GraphError::Cycle(_))));
    }

    #[test]
    fn validate_rejects_dangling_prerequisite() {
        let g = ConceptGraph::new([concept("factoring", &["algebra"])]); // algebra missing
        assert!(matches!(
            g.validate(),
            Err(GraphError::UnknownPrerequisite { .. })
        ));
    }

    #[test]
    fn closure_pulls_in_transitive_prereqs_but_not_siblings() {
        let closure = calc_graph().prerequisite_closure(&[ConceptId::new("integration_techniques")]);
        assert!(closure.contains(&ConceptId::new("algebra")));
        assert!(closure.contains(&ConceptId::new("factoring")));
        // A sibling under factoring is not a prerequisite of integration.
        assert!(!closure.contains(&ConceptId::new("difference_of_squares")));
    }

    #[test]
    fn weakest_link_traces_past_the_surface_topic_to_the_decayed_prereq() {
        // A failed Calc-3 integration problem is not really "integration is weak"
        // — it traces to a factoring prerequisite that decayed long ago (§2.1).
        let problem = Problem {
            id: ProblemId::new(),
            subject_id: SubjectId::new("math"),
            concepts: vec![ConceptId::new("integration_techniques")],
            difficulty: Difficulty::Hard,
            content: r"\int \frac{1}{x^2 - 1}\,dx".into(),
            solution: r"\tfrac{1}{2}\ln\left|\frac{x-1}{x+1}\right| + C".into(),
            generated_by: ProblemSource::Template,
            attribution: None,
            hints: Vec::new(),
            steps: Vec::new(),
        };

        let weak = find_weakest_prerequisite(
            &calc_graph(),
            &problem,
            &decayed_factoring_learner(),
            &ExponentialDecay,
            Utc::now(),
        )
        .expect("problem has tagged concepts");

        assert_eq!(weak.concept_id, ConceptId::new("factoring"));
        assert_eq!(weak.reason, WeakReason::Decayed);
        assert!(weak.estimated_mastery < 0.1, "got {}", weak.estimated_mastery);
    }

    #[test]
    fn ready_frontier_unlocks_only_when_prerequisites_are_met() {
        let frontier = ready_frontier(
            &calc_graph(),
            &decayed_factoring_learner(),
            &ExponentialDecay,
            Utc::now(),
        );
        // Factoring decayed, but its prerequisite (algebra) is solid → ready to
        // remediate.
        assert!(frontier.contains(&ConceptId::new("factoring")));
        // difference_of_squares is blocked: its prerequisite factoring is below
        // threshold.
        assert!(!frontier.contains(&ConceptId::new("difference_of_squares")));
        // partial_fractions is already mastered → nothing to learn there.
        assert!(!frontier.contains(&ConceptId::new("partial_fractions")));
    }
}
