# Lattice

A concept-graph adaptive tutoring app. V1 is scoped to **the math underneath
machine learning** (linear algebra · calculus · probability) and is built as a
personal, dogfoodable tool — see [`lattice_product_spec.md`](./lattice_product_spec.md)
for the full product vision.

The V1 core is **deliberately AI-free** (spec §2.1): a deterministic engine that,
when you miss a problem, traces the failure *past the surface topic* to the
specific prerequisite that's actually weak — e.g. a failed integration problem
diagnosed down to a **factoring** skill that decayed a year ago.

## Architecture

A Cargo workspace (Rust) behind a Tauri 2 + SvelteKit shell. Dependencies point
inward to `lattice-core`; the Tauri app is a thin adapter (spec §9).

```
crates/
  lattice-core/      domain types (no I/O)
  lattice-graph/     DAG validation, decay-aware mastery, weakest-link, frontier   ← deterministic engine
  lattice-content/   parameterized problem templates + subject loader
  lattice-storage/   Storage trait + SQLite backend (Postgres-ready via sqlx)
  lattice-service/   transport-agnostic orchestration (the V1 loop)
subjects/math/       the concept graph + templates, as DATA (concepts.json, templates.json)
src-tauri/           Tauri shell — exposes lattice-service as IPC commands
src/                 SvelteKit frontend (KaTeX for math), built to ./build
```

**The loop:** `next_problem` → you submit work → `submit_attempt` grades it,
updates mastery, and on failure returns the diagnosed weak link plus a freshly
generated practice problem targeting it.

## How you learn with it, and the tutoring engine

You learn the ML/DL math by *using* Lattice — practicing the prerequisite graph
of probability, statistics, calculus, and linear algebra. Adding more of that
math is a content task (new concepts + templates under `subjects/`), not new code.

Separately, the **adaptive tutoring engine** decides what you're ready for and
diagnoses failures. The learner model is pluggable behind the `MasteryModel`
trait (`lattice-graph/src/mastery.rs`), which owns both the read-time estimate
and the write-time update. The default is **Bayesian Knowledge Tracing** (`Bkt`);
`ExponentialDecay` is an alternative. Roadmap: fit the BKT parameters from the
logged `attempts` (EM), then a Deep Knowledge Tracing LSTM (via `candle`) behind
the same trait.

## Running it

> **macOS + exFAT note:** this repo is on an exFAT external drive, which can't
> hard-link and scatters AppleDouble `._*` files that break Tauri's build. Builds
> go into a shared **APFS disk image** on the same drive instead: `./target` is a
> (gitignored) symlink to `/Volumes/Build/lattice`. **Mount the image once per login
> session before building** — run `mount-build` (on your PATH; idempotent). Unmount
> with `hdiutil detach /Volumes/Build`.

```bash
# Backend tests (fast — skips the Tauri app):
cargo test

# Launch the desktop app (starts Vite + builds + opens the window):
bun install (or npm install)
bun run tauri dev (or npm run tauri dev)
```

The SQLite database is created on first launch in the OS app-data dir
(`~/Library/Application Support/com.lattice.app/lattice.db`).

## Status

| Pillar (spec) | State |
|---|---|
| §2.1 Concept graph + deterministic diagnosis | ✅ V1 |
| §2.2 Mastery tracking + decay | ✅ V1 (`MasteryModel` seam for knowledge tracing) |
| §2.3 Adaptive generation (templates) | ✅ V1 (property-test verified) |
| §2.4 AI misconception diagnosis | ⬜ Phase 2 (`lattice-diagnosis`) |
| §2.5 Socratic tutoring | ⬜ Phase 3 (`lattice-tutor`) |

### Known V1 simplifications (flagged in spec open questions)

- **Answer checking** (`lattice-service::answer_is_correct`) is a normalized
  substring match — fragile for math (`2` vs `2.0`). Needs symbolic/numeric
  equivalence before Phase 3 (open Q6). Isolated to one function.
- **Subject data** is loaded from a dev path relative to `src-tauri`; bundling it
  as a Tauri resource is a release-time follow-up.
