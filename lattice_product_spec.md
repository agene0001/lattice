# Lattice — Concept-Graph Adaptive Tutoring Platform
### Product Specification (v0.1)

> "lattice" is a placeholder codename — a prerequisite graph is literally a partial order / lattice structure. Swap freely.

---

## 1. Problem Statement

Most learning platforms operate as:

```
Concept → Lesson → Quiz
```

A wrong answer gets "Incorrect. Review integration techniques." — true, but useless. A human tutor instead thinks:

```
Problem → Mistake Analysis → Concept Diagnosis → Targeted Practice
```

The actual failure mode this solves: a student doesn't fail Calc 3 because integration is impossible — they fail because a prerequisite from two years ago (factoring, exponent rules, algebraic manipulation) quietly eroded and nobody told them which one. Most platforms can't tell the difference between "never learned this" and "learned this, forgot it" and "learned the prerequisite wrong in a specific, identifiable way." Lattice is built around making that distinction explicit and acting on it.

---

## 2. Platform Vision & Goals

As with the language-platform spec, this section covers the full intended system, including pillars not in V1 — specified in detail because they shape the data model now even though they're built later. See §3 for what's actually in scope per phase.

### 2.1 Pillar 1 — Prerequisite Concept Graph & Deterministic Diagnosis

This is the foundation, and deliberately **does not require AI**. Every subject is modeled as a directed acyclic graph: nodes are concepts, edges are prerequisite relationships.

```
Algebra
 ├─ Factoring
 │   └─ Partial Fractions
 │       └─ Integration Techniques
 │           └─ Differential Equations
```

Every exercise is tagged with the concept(s) it exercises. When a student fails a problem:

1. Look up the concepts required by that problem.
2. Check the student's mastery score for each.
3. Find the weakest one.
4. Generate targeted review on that concept specifically — not the surface-level topic the problem happened to be filed under.
5. Retest the original problem once the weak concept is addressed.

This alone — pure graph traversal against tagged mastery scores — is a meaningfully better experience than a flat question bank, and ships without any LLM dependency. It's the cheapest, most validate-able part of the product and should be built first.

### 2.2 Pillar 2 — Mastery Tracking & Learning Decay

Mastery isn't binary. It's a spectrum that degrades over time:

```
Mastered → Familiar → Rusty → Forgotten
```

Each concept tracks a mastery/confidence score, a last-practiced timestamp, and a decay rate. A concept "mastered" 14 months ago and never touched since should be treated as a likely failure point even though it was never formally "forgotten" — i.e. when a downstream problem fails and no directly-unmastered prerequisite explains it, the system should check for **decayed-but-previously-mastered** prerequisites as the likely root cause before concluding the failure is mysterious.

This is conceptually spaced repetition, but applied across an entire prerequisite graph rather than a flat deck — review is only proactively surfaced for decaying concepts that are actually relevant to what the student is currently working on, not a blanket daily review queue.

### 2.3 Pillar 3 — Adaptive Exercise Generation

Static question banks run out. The system should be able to generate practice on demand for a target concept and difficulty:

```
Student struggles with: completing the square
Generate: 10 easy, 10 medium, 3 word problems
```

For V1, this is **template-based, not AI-based** — parameterized problem templates (e.g. `ax + b = c` with randomized coefficients within a difficulty band) are cheap, deterministic, and easy to verify correct, which matters a lot for math specifically (an LLM silently generating a problem with no valid solution is a real failure mode). AI-backed generation (more varied, word-problem-style, less templatable content) is a Phase 3 enhancement layered on top, not the V1 mechanism.

### 2.4 Pillar 4 — AI Misconception Diagnosis

This is the genuinely hard, genuinely AI-requiring part, and the real differentiator. Knowing *which* concept a student is weak in is necessary but not sufficient — knowing *why* a specific answer was wrong is what a good human tutor actually does.

Example: a student is asked to factor `x² - 1` and writes `(x - 1)²`. That's not "doesn't know factoring" — it's a specific, identifiable misconception: confusing the difference-of-squares identity with squaring a binomial. A deterministic concept-tag system can tell you "factoring is weak." Only something that can read the actual submitted work can tell you *what kind of wrong* it is, and generate practice targeted at that specific confusion rather than generic factoring drills.

This requires capturing the student's **work**, not just a final answer — a multiple-choice or numeric-answer-only UI can't support this pillar. That's a real V1 design constraint, not a Phase-4 nuance (see §6, §11).

### 2.5 Pillar 5 — Socratic AI Tutoring

Most AI tutoring fails in one of two directions:

- **Too helpful** — student says "I don't know," AI gives the full solution. Learning stops.
- **Not helpful enough** — AI says "try again." Frustrating, no scaffolding.

The target behavior is Socratic: guide through questions, let the student do the actual work.

```
Student: Solve x + 5 = 12. I don't know.
AI:      What operation would undo adding 5?
Student: Subtract 5.
AI:      Right. What happens if we subtract 5 from both sides?
```

This is a real engineering risk, not just a prompting nicety — LLMs reliably over-help by default, and "never reveal the answer" is hard to guarantee through instruction alone. V1 design for this pillar (when it ships, Phase 3) should include a deterministic guardrail: check the model's response against the stored problem solution before showing it to the student, and reject/regenerate if the final answer appears, rather than trusting the model's self-restraint.

### 2.6 Pillar 6 — Multi-Subject "Learn Anything" Architecture

The concept-graph + mastery + diagnosis + adaptive-generation pattern isn't math-specific:

```
Math · Programming · Statistics · Languages · Chess openings · Music theory
```

are all, structurally, prerequisite graphs with tagged exercises. The architecture goal (§5, §9) is for a new subject to mean "new concept graph + problem bank + diagnosis prompt templates," defined as **data**, not new application code. Math ships in V1. Programming is the natural second subject — it has the same "wrong answer reveals a specific, diagnosable misconception" shape (e.g. a Rust ownership error isn't "doesn't know Rust," it's specifically "doesn't yet have move semantics" while functions and variables are fine), and is a subject you could dogfood and content-author yourself given your own background.

### 2.7 Business Model & Monetization

**Free tier:**
- Concept graph practice within one subject
- Static/templated problem bank
- Basic mastery tracking and decay-aware review queue

**Premium tier:**
- AI misconception diagnosis (Pillar 4) — the most expensive-to-run, highest-value feature
- Socratic AI tutoring (Pillar 5)
- AI-generated exercises beyond the template bank (Phase 3 enhancement to Pillar 3)
- Multi-subject access (Phase 4)
- Progress reporting — relevant if a K-12 vertical is pursued later, where parents rather than students are often the actual payer

**Cost structure note:** same shape as the language platform — LLM calls are the dominant variable cost, concentrated specifically in Pillars 4 and 5. Because Pillar 1–3 are deliberately non-AI in V1, the free tier is cheap to run almost indefinitely; the premium gate maps cleanly onto "the parts that cost money to run," which is a clean place to put a paywall.

---

## 3. Scope & Phasing Summary

| Pillar | V1 (personal use) | Phase 2 | Phase 3 | Phase 4 |
|---|---|---|---|---|
| 2.1 Concept graph + deterministic diagnosis | ✅ Built (no AI) | | | |
| 2.2 Mastery tracking + decay | ✅ Built | | | |
| 2.3 Adaptive exercise generation | ✅ Built (templates) | | AI-enhanced | |
| 2.4 AI misconception diagnosis | | ✅ Built | | |
| 2.5 Socratic AI tutoring | | | ✅ Built | |
| 2.6 Multi-subject plugin architecture | Data-driven design, math only | | + Programming subject | + further subjects |
| Multi-user / auth / billing | | | | ✅ Built |
| 2.7 Monetization gating | | | | ✅ Built |
| Website deployment | | | | ✅ Built |

V1 is deliberately AI-free at the core. This is the opposite emphasis from the language platform, where the LLM was central from day one — here the graph/decay engine is the thing to validate first precisely because it's cheap and fast to get right, before spending on the harder AI pillars.

---

## 4. System Architecture

Same shell choice and reasoning as the language platform (Tauri 2.x: web-tech frontend reusable as a future website, mobile targets from the same codebase, Rust backend keeps the actual complexity in Rust). Not re-derived here — see that spec §4.1 if you want the full rationale again.

### Workspace Layout

```
lattice/
├── crates/
│   ├── lattice-core/         # domain types, no I/O
│   ├── lattice-graph/        # concept DAG, mastery state, decay, weakest-node diagnosis
│   ├── lattice-content/      # exercise generation: template engine (V1) + AI generator (Phase 3)
│   ├── lattice-diagnosis/    # AI misconception classification from submitted work (Phase 2)
│   ├── lattice-tutor/        # Socratic dialogue engine + answer-leak guardrail (Phase 3)
│   ├── lattice-storage/      # Postgres (writes) + DuckDB (analytics reads)
│   └── lattice-service/      # transport-agnostic orchestration layer
├── subjects/
│   └── math/                 # concept graph + problem templates, as data (JSON/YAML)
├── src-tauri/                 # Tauri shell, registers lattice-service as IPC commands
└── frontend/                  # web frontend (SvelteKit), needs math rendering (KaTeX/MathJax)
```

### Data Flow (failure → diagnosis → practice)

```
frontend
  → invoke('submit_attempt', { problem_id, submitted_work })
  → lattice-service::submit_attempt(...)
       → lattice-storage::record_attempt(...)
       → lattice-graph::find_weakest_prerequisite(problem_id, learner_id)   // deterministic, V1
       → [Phase 2] lattice-diagnosis::classify_misconception(submitted_work)
       → lattice-content::generate_practice(target_concept, difficulty)
  ← PracticeSet { concepts_targeted, problems }
```

The deterministic path (`lattice-graph::find_weakest_prerequisite`) works standalone in V1. `lattice-diagnosis` is additive in Phase 2 — it refines *which* concept and *why*, it doesn't replace the graph traversal.

---

## 5. Crate / Module Breakdown

### `lattice-core`
```rust
pub enum MasteryState { Mastered, Familiar, Rusty, Forgotten }

pub struct Concept {
    pub id: ConceptId,
    pub subject_id: SubjectId,
    pub label: String,
    pub prerequisites: Vec<ConceptId>,
}

pub struct ConceptMastery {
    pub concept_id: ConceptId,
    pub state: MasteryState,
    pub confidence: f32,
    pub last_practiced_at: DateTime<Utc>,
    pub decay_rate: f32,
}

pub struct Problem {
    pub id: ProblemId,
    pub subject_id: SubjectId,
    pub concepts: Vec<ConceptId>,        // many-to-many tagging
    pub difficulty: Difficulty,
    pub content: String,                  // LaTeX or plain text
    pub solution: String,
}

pub struct Attempt {
    pub id: AttemptId,
    pub learner_id: LearnerId,
    pub problem_id: ProblemId,
    pub submitted_work: String,           // not just final answer — see §2.4
    pub is_correct: bool,
}

pub struct Diagnosis {
    pub attempt_id: AttemptId,
    pub diagnosed_concept: ConceptId,
    pub misconception_label: String,
    pub explanation: String,
}
```

### `lattice-graph`
The deterministic core. Owns:
- DAG traversal — given a failed problem's required concepts, find the weakest one by current mastery score.
- Decay computation — `current_estimated_mastery = confidence * decay_function(time_since_last_practiced)`. When no directly-unmastered prerequisite explains a failure, check decayed-but-previously-mastered prerequisites next.
- `next_ready_concept(learner_id) -> ConceptId` — the "ready to learn" frontier: concepts whose prerequisites are sufficiently mastered, for forward progress (not just remediation).

### `lattice-content`
- V1: template engine — parameterized problem templates with randomized coefficients within a difficulty band, deterministically solvable and verifiable.
- Phase 3: AI-backed generation layered on top for more varied/word-problem content, gated behind the same correctness-verification discipline (a generated problem should be checked to actually have a valid, computable solution before being shown).

### `lattice-diagnosis` (Phase 2)
- Takes `submitted_work` + `problem` + the concept(s) involved, calls the Anthropic API, requests structured JSON:
```json
{
  "diagnosed_concept": "difference_of_squares",
  "misconception_label": "confused difference of squares with squaring a binomial",
  "explanation": "...",
  "confidence": 0.8
}
```
- This is the highest-value, highest-cost pillar — worth its own crate boundary even though it depends on `lattice-graph` types, so it can be feature-flagged off entirely for cost control without touching the deterministic path.

### `lattice-tutor` (Phase 3)
- Socratic dialogue state machine: tracks current problem, hint level, escalates hint specificity only after repeated stuck-responses.
- **Answer-leak guardrail**: before returning a tutor response to the frontend, deterministically check it against `problem.solution` and reject/regenerate if the final answer is present. Don't rely on prompt instructions alone for this — see §2.5.

### `lattice-storage`
Same dual-database pattern as the language platform and your sports betting app: Postgres for writes/event log, DuckDB for analytics reads (mastery summaries, decay alerts, readiness queues).

### `lattice-service`
Transport-agnostic orchestration — identical role to `glossa-service`: plain async functions wrapping the domain crates, with zero Tauri- or HTTP-specific types, so a future `lattice-api` (Axum) is a thin adapter rather than a rewrite.

### `subjects/math/`
Concept graph and problem templates as data files (JSON/YAML), not Rust code — this is what makes Pillar 6 (multi-subject) cheap later: adding Programming as a subject means adding `subjects/programming/`, not a new crate.

### `frontend`
Same framework recommendation as the language platform (SvelteKit, React+Vite as alternative) — see that spec §5 for reasoning. Additional requirement here: math content needs LaTeX rendering (KaTeX is the lighter-weight standard choice; MathJax is the heavier/more-compatible alternative). Views needed: Problem (with work-input area, not just an answer field — see §2.4), Diagnosis/Review (weak concept + targeted practice), Graph (visual prerequisite map + mastery state), Tutor (Phase 3, Socratic chat).

---

## 6. Data Model

```sql
-- Postgres (writes, source of truth)
learners(id, created_at)
subjects(id, name)
concepts(id, subject_id, label, description)
concept_prerequisites(concept_id, prerequisite_concept_id)   -- DAG edges
learner_concept_mastery(learner_id, concept_id, state, confidence, last_practiced_at, decay_rate)
problems(id, subject_id, content, difficulty, solution, generated_by)   -- 'template' | 'ai'
problem_concepts(problem_id, concept_id)                      -- many-to-many
attempts(id, learner_id, problem_id, submitted_work, is_correct, created_at)
diagnoses(id, attempt_id, diagnosed_concept_id, misconception_label, explanation, created_at)
tutor_sessions(id, learner_id, problem_id, started_at)
tutor_turns(id, tutor_session_id, speaker, text, hint_level, created_at)
```

```sql
-- DuckDB (reads, via postgres_scanner)
-- mastery_summary: per learner/subject, counts by state, trend over time
-- concept_readiness_queue: concepts whose prerequisites are sufficiently mastered
-- decay_alerts: previously-mastered concepts now decayed past threshold, filtered to relevance for current study
```

`submitted_work` is a text field from V1 onward, even before `lattice-diagnosis` exists to consume it (Phase 2) — capturing it from day one avoids a gap in historical data once the diagnosis pillar ships.

---

## 7. AI Integration & Prompt Design

Two genuinely distinct AI use cases — kept in separate crates deliberately (§5):

**Misconception diagnosis** (`lattice-diagnosis`): given a wrong attempt's submitted work plus the problem and its tagged concepts, classify the specific error type and map it to a concept node. Structured JSON output, not prose — same rationale as the language platform: deterministic downstream handling, no parsing free text.

**Socratic tutoring** (`lattice-tutor`): system prompt instructs the model to never state the final answer directly, to ask a guiding question narrowing toward the relevant concept, and to escalate hint specificity only after repeated stuck-responses. Output is still checked against the stored solution post-hoc (§5) rather than trusting the instruction alone — model self-restraint on "don't give away the answer" is not reliable enough to skip a deterministic check.

---

## 8. Tech Stack

| Concern | Choice |
|---|---|
| Core language | Rust |
| Async runtime | Tokio |
| LLM | Anthropic API (Claude) — diagnosis + Socratic tutoring only, not the core graph |
| App shell | Tauri 2.x |
| Frontend framework | SvelteKit (recommended) or React + Vite |
| Math rendering | KaTeX |
| DB (writes) | PostgreSQL |
| DB (reads/analytics) | DuckDB |
| Serialization | serde / serde_json |
| Future HTTP API (Phase 4) | Axum, calling `lattice-service` |

---

## 9. Designing for Multi-Subject & the Website Transition

Same underlying discipline as the language platform:
- `LearnerId` and `SubjectId` are real types from day one even with one learner and one subject populated.
- `subjects/math/` as data rather than code is the load-bearing decision for Pillar 6 — a new subject should require zero new Rust code if the concept/problem schema is general enough.
- `lattice-service` has zero Tauri- or HTTP-specific types, so `src-tauri` and a future `lattice-api` are both thin adapters over the same functions.
- `attempts.submitted_work` being captured from V1 (§6) means Phase 2's diagnosis pillar has historical data to work with retroactively, not just going forward.

---

## 10. Content Import Pipeline (`lattice-import`)

The biggest practical bottleneck in getting Lattice usable is **populating problems and lessons at scale across many math disciplines**. LLM-generating content from scratch does not scale and carries an unacceptable correctness risk for math (a confidently-generated problem with no clean solution, or a wrong answer key, is exactly the failure mode §12.5 already flags). The right model is to **ingest existing problems and use the LLM to structure and tag them**, not author them.

### 10.1 The Core Reframe — LLM as Structurer, Not Author

The LLM's job in import is a closed-vocabulary classification/extraction task, not open generation:

```
RawProblem (text + solution, from some source)
  → LLM: structure into Problem { content, solution, difficulty }
         + tag with concept_ids drawn from the EXISTING concept graph
  → CAS verification (does the stated solution actually check out?)
  → store with provenance (source, license)
```

This is the same machinery as Pillar 4 misconception diagnosis (§2.4): structured JSON output, constrained to a fixed concept set. It is reliable in a way authoring is not, because the model is mapping existing content onto a known taxonomy rather than inventing both the content and its correctness.

### 10.2 Source Adapters

Same pluggable-trait pattern used elsewhere in the architecture (`TrendSource`, `GameSource`, subject plugins):

```rust
pub struct RawProblem {
    pub content: String,        // LaTeX or plain text, as found
    pub solution: Option<String>,
    pub source_label: String,   // dataset name, textbook title, URL
    pub license: License,       // tracked from ingestion — see 10.6
    pub hint_tags: Vec<String>, // any pre-existing subject/difficulty labels from the source
}

pub trait ProblemSource {
    async fn fetch(&self) -> Result<Vec<RawProblem>>;
}
```

`MathDatasetSource`, `OpenStaxSource`, `OcrSource` are all adapters; the structuring → verification → provenance pipeline downstream is shared and source-agnostic.

### 10.3 Sources, Ordered by What to Build First

**Tier 1 — pre-structured open datasets (no OCR, build first).** These are JSON/CSV with LaTeX already present; many are pre-tagged, so a chunk of the concept mapping is already done:

- **MATH dataset** (Hendrycks et al.) — ~12,500 competition problems with step-by-step solutions, already tagged by subject (Algebra, Number Theory, Geometry, Counting & Probability, Precalculus, etc.) and difficulty level 1–5. The single highest-leverage import: its own subject/level labels seed the `hint_tags` → concept-graph mapping, and it gives thousands of problems to validate the whole graph/diagnosis loop against immediately.
- **GSM8K** — 8.5K grade-school word problems with multi-step solutions, MIT licensed. Relevant if the K-12 vertical (§12.1) is pursued.
- **OpenMathInstruct-1** — 1.8M problem-solution pairs, commercially permissive license. More than needed, but clean.
- **OpenStax** — full CC-BY math textbooks (Algebra, Precalc, Calculus, Statistics) with exercises. This is also the best source for *lesson prose*, not just problems — relevant for the lesson side of content, not only the problem bank. Attribution required, redistributable.

**Tier 2 — OCR path (arbitrary textbooks / blogs, build last).** Only needed for sources not already in structured form. Math-aware OCR is required — general OCR butchers equations:

- **Surya** (`surya_latex_ocr`) — current open-source pick, handles images or PDFs, runs locally (CPU/GPU/MPS). Absorbed and improved on the older `texify` model. Better than pix2tex (block-equations only, hallucinates on text) and Nougat (whole-page, hallucinates on small math-only images).
- **Mathpix OCR API** — paid, highest accuracy on difficult notation. Worth it only if Surya's accuracy on gnarly notation becomes the blocker. (Large academic pipelines have migrated Nougat → Mathpix specifically for reliability.)

Recommended order: MATH first (pre-tagged, immediate scale), OpenStax for lessons, OCR last once the pipeline is proven.

### 10.4 Verification

Every imported problem runs through a CAS check before being marked usable:

- **SymPy** can confirm a stated solution actually satisfies the problem for a large fraction of algebra/calculus content, and flags what it can't verify for manual review (rather than silently trusting it).
- This is the **same symbolic/numeric-equivalence machinery** §12.6 already requires for the answer-leak guardrail — build it once, use it for both import verification and tutor-response checking.
- Problems that fail or can't be auto-verified are stored with `verified = false` and excluded from the active practice pool until reviewed, rather than discarded.

### 10.5 Lessons (vs. Problems)

Lessons — expository prose teaching a concept — are a different import problem from problems, in two ways that push *away* from direct copying:

- **Copyright is worse for prose.** A routine exercise is close to functional/factual content; a textbook's *explanation* is the author's creative expression — the most copyrightable thing in the book. Converting a copyrighted chapter to markdown and bundling it for others is far more exposed than the equivalent for problems.
- **No CAS verification.** Prose has no machine-checkable correctness. The failure mode is also softer (an awkward explanation is still useful; a problem with no valid solution is a hard fail). What's worth checking in generated prose is narrow and factual: definitions and theorem conditions, not the overall exposition.

Because of this, lessons lean on generation more than import. Three lanes, in priority order:

**Lane A — grounded generation (default).** Not cold generation. Feed the LLM an openly-licensed source passage (e.g. an OpenStax section) as grounding context, plus the concept-graph node, target level, and the standard markdown format, and have it produce the lesson *from* that source. This is RAG-flavored: far more accurate than cold generation because the model restructures grounded material rather than recalling from nothing; output is in the platform's own voice/format; and it's clean to redistribute (a derivative of open-licensed material, attribution only). Add a light review pass targeted specifically at definitions/theorem statements. This is the right default for V1.

**Lane B — direct import of openly-licensed lessons.** OpenStax and LibreTexts chapters *are* lessons, CC-licensed and redistributable with attribution. Where polished existing exposition is preferred over generated, convert those to markdown and use directly.

**Lane C — convert arbitrary textbook/blog → markdown.** Technically very doable; sorts by document type:
- *PDF chapters:* try **PyMuPDF4LLM** first for native (selectable-text) PDFs — no ML models, CPU-only, fastest. Fall back to **Marker** (all-rounder: PDF/DOCX/HTML/EPUB → markdown, Surya-based OCR, optional `--use_llm` for messy layouts) when quality isn't there. Use **MinerU** when the math is dense/multi-column (high formula recognition, LaTeX-friendly, wants a GPU). Note **Marker's model weights carry commercial-use licensing restrictions** — fine for personal/dev use, check before any distributed build.
- *Blogs/HTML:* much easier than PDF — the math is usually already LaTeX in the page source (MathJax/KaTeX), so it's structure extraction, not OCR. **Pandoc**, **MarkItDown**, or **Jina Reader** (URL→markdown) all work.

The catch on Lane C: for *copyrighted* sources the output is personal-use-only. In practice, use it to build personal reference material and to produce **grounding input for Lane A** — not as a content source for a distributed build.

Pipeline-wise this is a `LessonSource` trait paralleling `ProblemSource`, sharing the same provenance/license tracking, but the verification leg is a lightweight definition/theorem review pass rather than a CAS check:

```rust
pub struct RawLesson {
    pub markdown: String,
    pub concept_id: Option<ConceptId>,  // may be assigned at tag time rather than ingest
    pub source_label: String,
    pub license: License,
}

pub trait LessonSource {
    async fn fetch(&self) -> Result<Vec<RawLesson>>;
}
```

**Design note:** lessons are lower-stakes in Lattice than in a conventional course platform — §2.2 treats explicit explanation as opt-in support, not the main event. The core product is concept-graph + problems + diagnosis. Don't over-engineer the lesson pipeline; grounded generation (Lane A) is sufficient for V1, and effort is better spent on the problem bank and diagnosis loop.

### 10.6 Provenance & License Tracking (do this from day one)

The relevant fork is **personal-use vs. distributed-to-others**, not free vs. paid. Giving the platform away free or open-source does *not* remove this concern — redistributing copyrighted textbook/blog prose in a public repo is still infringement regardless of price, and arguably more exposed since the content becomes publicly mirrorable. What makes a future open-source release clean is that the *bundled content is itself openly licensed*, not that the software is free.

- Every `Problem` and `Lesson` carries `source` and `license` fields from first ingestion.
- For V1 (a single-user personal tool, §2), pulling problems or prose from a textbook you own or a blog for your own private study is a normal, defensible use.
- For any future release distributed to other people (free, open-source, or paid alike), the bundled content set must be filtered to openly-licensed sources only (OpenStax, LibreTexts, MATH, GSM8K, OpenMathInstruct). If `license` is a column from day one, that's a `WHERE` clause at release time rather than a re-import of everything. Lane-A grounded generation also helps here: a lesson generated from CC-BY grounding is redistributable in a way a converted copyrighted chapter is not.
- *Not legal advice — the eventual distribution question warrants a real opinion at that stage, not an assumption.*

### 10.7 Schema Additions

```sql
-- extends problems table from §6
ALTER TABLE problems ADD COLUMN source TEXT;          -- dataset name / textbook / URL
ALTER TABLE problems ADD COLUMN license TEXT;         -- 'CC-BY' | 'MIT' | 'copyrighted-personal' | ...
ALTER TABLE problems ADD COLUMN verified BOOLEAN DEFAULT FALSE;
ALTER TABLE problems ADD COLUMN import_hint_tags JSONB; -- raw source labels, pre-concept-mapping

-- lessons (new table — expository content keyed to a concept node)
lessons(id, concept_id, subject_id, markdown, source, license, generated_by, reviewed, created_at)
-- generated_by: 'grounded-llm' | 'imported' | 'hand-written'
-- reviewed: definition/theorem review pass complete (the prose analogue of `verified`)
```

`generated_by` on `problems` (§6) gains a third value beyond `'template' | 'ai'`: `'imported'`.

### 10.8 Where This Sits in the Roadmap

Slots into **Phase 1**, alongside the deterministic core — the concept graph and diagnosis loop are far easier to validate against thousands of real imported problems than against a handful of hand-written ones. Tier 1 (dataset import) is Phase 1; Tier 2 (OCR) and the Lane-C conversion path can wait until the pipeline is proven, since they add the most complexity for the least immediate volume. Lesson generation (Lane A) is Phase 1; lesson import lanes (B/C) follow as needed.

---

## 11. Relationship to Glossa (the language platform)

Worth naming explicitly since both specs share the same conceptual shape: a continuously updated learner model driving dynamically generated content, rather than a fixed curriculum. The source material for this platform calls that out directly as "Learn Anything" — one engine, subjects as plugins.

Recommendation for now: **keep them separate codebases**, not a shared crate from day one. Premature abstraction between two products that haven't individually proven out yet is a real risk — the language platform's "knowledge graph" (flat lexeme mastery) and this platform's "concept graph" (DAG with prerequisite edges and decay-driven root-cause tracing) are structurally different enough that a forced-shared abstraction right now would likely be wrong in both directions. Once both have independent V1 traction, it's worth revisiting whether `lattice-graph`'s DAG/decay machinery is a generalization of `glossa-graph`'s flat model (it plausibly is) and extracting a shared core crate at that point, with real evidence about what the two actually need in common rather than guessing now.

---

## 12. Open Questions

1. **Starting scope within math** — K-12 math (larger market, "parents pay" per the source material), or your own current coursework (calc/linear algebra/stats, given your ML work) as the initial concept graph? The latter doubles as dogfooding, same as the language platform being scoped to a single learner = you.
2. **Submitted-work capture format** — free text, structured step-by-step input, or photo/OCR of handwritten work? This materially affects how good Pillar 4's diagnosis can be and is a real UI design decision, not a backend detail.
3. **Concept graph authoring** — hand-authored by you initially, or seeded from an existing open curriculum standard (e.g. Common Core concept sequencing) and refined from there?
4. **Decay rate** — fixed per concept-type, or learned/tuned empirically from your own usage data over time? Start fixed; revisit once there's enough attempt history to tune against.
5. **Template engine correctness** — for V1's parameterized problem templates, what's the verification step that a randomized instance is actually solvable and the stored solution is correct? Worth a property-based test approach (generate N instances, verify solver agreement) rather than spot-checking.
6. **Answer-leak guardrail (§5, §7)** — exact-string match against `problem.solution` is fragile for math (`x=2` vs `2` vs `x = 2.0`). Likely needs a symbolic/numeric equivalence check rather than string comparison — worth scoping before Phase 3 starts, not during it.
