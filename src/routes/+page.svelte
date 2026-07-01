<script>
  import { onMount } from 'svelte';
  // Imported as `Katex` so it doesn't shadow the global `Math` used below.
  import Katex from '$lib/Math.svelte';
  import Lesson from '$lib/Lesson.svelte';
  import {
    listSubjects,
    selectSubject,
    resolveRefs,
    subjectInfo,
    conceptMap,
    nextProblem,
    submitAttempt,
    practiceConcept,
    lesson as fetchLesson,
    draftLesson,
    saveLesson,
    modelParams,
    refitModel,
    getDiagnosisSettings,
    setDiagnosisSettings,
    setApiKey,
    diagnoseAttempt,
    generateAiProblem
  } from '$lib/api.js';

  let subject = $state(null);
  let subjects = $state([]); // all available subjects (the switcher)
  let currentSubjectId = $state(null);
  let view = $state('practice'); // 'practice' | 'map'
  let problem = $state(null);
  let work = $state('');
  let outcome = $state(null);
  let concepts = $state([]);
  let busy = $state(false);
  let error = $state(null);
  let showSolution = $state(false);
  let notice = $state(null);
  let model = $state(null);

  // Phase 2: AI diagnosis (BYOK).
  let settings = $state(null);
  let diagnosis = $state(null);
  let diagnosing = $state(false);
  let selProvider = $state('anthropic');
  let selModel = $state('');
  let keyDraft = $state('');
  let settingsSaved = $state(false);
  let aiDifficulty = $state('medium');
  let generating = $state(false);

  // Learn view: the "learn the concept" side. `learnSel` is the open concept
  // (null => the concept list); lessons are authored as data and can be
  // AI-drafted, edited, and saved from here.
  let learnSel = $state(null);
  let lessonData = $state(null);
  let externalPrereqs = $state([]); // resolved cross-subject prerequisites
  let lessonBusy = $state(false);
  let editing = $state(false);
  let draft = $state('');
  let editOriginal = $state(''); // what the editor loaded, to detect unsaved edits
  let drafting = $state(false);
  let savingLesson = $state(false);
  let lessonSaved = $state(false);
  let lessonDirty = $derived(editing && draft !== editOriginal);

  // Friendly phrasing for the deterministic diagnosis reason (spec §2.2).
  const REASON = {
    never_learned: 'never practiced',
    decayed: 'learned before, but it has decayed',
    weak: 'still shaky'
  };

  onMount(async () => {
    try {
      subjects = await listSubjects();
      settings = await getDiagnosisSettings();
      selProvider = settings.provider;
      selModel = settings.model;
      await loadSubject();
    } catch (e) {
      error = String(e);
    }
  });

  // Load (or reload) everything tied to the active subject.
  async function loadSubject() {
    subject = await subjectInfo();
    currentSubjectId = subject?.id ?? null;
    model = await modelParams();
    await Promise.all([refreshMap(), loadProblem()]);
  }

  // Switch subjects: tell the backend, then reload its graph, lesson, problem.
  async function switchSubject(id) {
    if (!id || id === currentSubjectId || busy) return;
    busy = true;
    error = null;
    try {
      await selectSubject(id);
      // Reset per-subject view state so nothing leaks across subjects.
      problem = null;
      outcome = null;
      diagnosis = null;
      learnSel = null;
      lessonData = null;
      view = 'practice';
      await loadSubject();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function refreshMap() {
    concepts = await conceptMap();
  }

  async function loadProblem() {
    busy = true;
    error = null;
    notice = null;
    outcome = null;
    diagnosis = null;
    showSolution = false;
    work = '';
    try {
      problem = await nextProblem();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  // Advance within the concept you're working on (don't jump back to the
  // auto-picked foundation). Switch topics via the Concept map.
  async function nextInConcept() {
    const cid = problem?.concepts?.[0];
    if (cid) {
      await practice(cid);
    } else {
      await loadProblem();
    }
  }

  // Phase 3: generate a fresh, varied AI problem for the current concept.
  async function generateAi() {
    const cid = problem?.concepts?.[0];
    if (!cid || generating) return;
    generating = true;
    error = null;
    notice = null;
    outcome = null;
    diagnosis = null;
    showSolution = false;
    work = '';
    try {
      problem = await generateAiProblem(cid, aiDifficulty);
    } catch (e) {
      error = String(e);
    } finally {
      generating = false;
    }
  }

  async function submit(event) {
    event?.preventDefault();
    if (!problem || busy) return;
    busy = true;
    error = null;
    diagnosis = null;
    try {
      outcome = await submitAttempt(problem.id, work);
      await refreshMap();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function practiceWeakLink() {
    if (outcome?.practice) {
      problem = outcome.practice;
      work = '';
      outcome = null;
      diagnosis = null;
      showSolution = false;
      view = 'practice';
    }
  }

  // Phase 2: ask the configured LLM why this wrong attempt was wrong.
  async function diagnose() {
    if (!outcome || !problem || diagnosing) return;
    diagnosing = true;
    error = null;
    try {
      diagnosis = await diagnoseAttempt(outcome.attempt_id, problem.id, work);
    } catch (e) {
      error = String(e);
    } finally {
      diagnosing = false;
    }
  }

  function chooseProvider(id) {
    selProvider = id;
    const opt = settings?.providers.find((p) => p.id === id);
    if (opt) selModel = opt.default_model;
  }

  async function saveSettings() {
    error = null;
    try {
      await setDiagnosisSettings(selProvider, selModel);
      if (keyDraft.trim()) {
        await setApiKey(selProvider, keyDraft);
        keyDraft = '';
      }
      settings = await getDiagnosisSettings();
      selProvider = settings.provider;
      selModel = settings.model;
      settingsSaved = true;
      setTimeout(() => (settingsSaved = false), 2000);
    } catch (e) {
      error = String(e);
    }
  }

  function conceptLabel(id) {
    return concepts.find((c) => c.id === id)?.label ?? id;
  }

  // Which concepts have exercises (the backend `practiceable` flag).
  let practiceableIds = $derived(
    new Set(concepts.filter((c) => c.practiceable).map((c) => c.id))
  );

  // Prerequisites of the current problem's concept(s) — the "related topics that
  // help you master it", offered as clickable practice.
  let problemPrereqs = $derived.by(() => {
    if (!problem) return [];
    const seen = new Set();
    for (const cid of problem.concepts) {
      const c = concepts.find((x) => x.id === cid);
      for (const p of c?.prerequisites ?? []) seen.add(p);
    }
    return [...seen];
  });

  // Generate a problem for a specific concept (from the map or a topic chip).
  async function practice(conceptId) {
    if (busy) return;
    if (!practiceableIds.has(conceptId)) {
      notice = `No exercises for ${conceptLabel(conceptId)} yet — it's on the roadmap.`;
      return;
    }
    busy = true;
    error = null;
    notice = null;
    outcome = null;
    diagnosis = null;
    showSolution = false;
    work = '';
    try {
      problem = await practiceConcept(conceptId);
      view = 'practice';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  // --- Learn view ---

  async function openLesson(id) {
    learnSel = id;
    lessonData = null;
    externalPrereqs = [];
    editing = false;
    draft = '';
    error = null;
    lessonBusy = true;
    try {
      lessonData = await fetchLesson(id);
      // Resolve cross-subject prerequisites (labels + subject names) for display.
      const refs = lessonData?.external_prerequisites ?? [];
      externalPrereqs = refs.length ? await resolveRefs(refs) : [];
    } catch (e) {
      error = String(e);
    } finally {
      lessonBusy = false;
    }
  }

  function closeLesson() {
    learnSel = null;
    lessonData = null;
    externalPrereqs = [];
    editing = false;
    draft = '';
  }

  // Jump to a prerequisite that lives in another subject: switch subjects, then
  // practise that concept there.
  async function crossPractice(ref) {
    if (busy) return;
    await switchSubject(ref.subject_id);
    await practice(ref.concept_id);
  }

  // Draft an original lesson with the configured LLM, dropped into the editor
  // for review before it's saved (we never auto-commit AI prose).
  async function draftWithAi() {
    if (!learnSel || drafting) return;
    drafting = true;
    error = null;
    try {
      draft = await draftLesson(learnSel);
      editOriginal = ''; // a fresh AI draft is all-new relative to what's saved
      editing = true;
    } catch (e) {
      error = String(e);
    } finally {
      drafting = false;
    }
  }

  // Edit the actual on-disk file (frontmatter included), so saving preserves any
  // source/license attribution. A brand-new lesson starts from a title scaffold.
  function startEdit() {
    draft = lessonData?.raw ?? (lessonData ? `# ${lessonData.label}\n\n` : '');
    editOriginal = draft;
    editing = true;
  }

  // ⌘/Ctrl-S saves from inside the editor.
  function editorKeydown(e) {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault();
      saveLessonDraft();
    }
  }

  async function saveLessonDraft() {
    if (!learnSel || savingLesson || !draft.trim()) return;
    savingLesson = true;
    error = null;
    try {
      await saveLesson(learnSel, draft);
      lessonData = await fetchLesson(learnSel);
      editing = false;
      lessonSaved = true;
      setTimeout(() => (lessonSaved = false), 2000);
      await refreshMap(); // refresh the has-lesson flags on the concept list
    } catch (e) {
      error = String(e);
    } finally {
      savingLesson = false;
    }
  }

  // Jump from a lesson straight into practising that concept.
  function practiceFromLesson() {
    if (lessonData?.practiceable) practice(learnSel);
  }

  // Open the lesson for the concept the current problem targets.
  function learnCurrentConcept() {
    const cid = problem?.concepts?.[0];
    if (!cid) return;
    view = 'learn';
    openLesson(cid);
  }

  // Group concepts by curriculum area (ordered per subject.groups); within each
  // group, foundations (fewer prerequisites) come first.
  let grouped = $derived.by(() => {
    const order = subject?.groups ?? [];
    const byGroup = new Map();
    for (const c of concepts) {
      const g = c.group || 'Other';
      if (!byGroup.has(g)) byGroup.set(g, []);
      byGroup.get(g).push(c);
    }
    for (const items of byGroup.values()) {
      items.sort(
        (a, b) =>
          a.prerequisites.length - b.prerequisites.length || a.label.localeCompare(b.label)
      );
    }
    const ordered = [];
    for (const g of order) {
      if (byGroup.has(g)) {
        ordered.push({ name: g, items: byGroup.get(g) });
        byGroup.delete(g);
      }
    }
    for (const [g, items] of byGroup) ordered.push({ name: g, items });
    return ordered;
  });

  const groupAvgRaw = (items) =>
    items.length ? items.reduce((s, c) => s + c.estimated_mastery, 0) / items.length : 0;

  function masteryClass(value) {
    if (value >= 0.85) return 'mastered';
    if (value >= 0.6) return 'familiar';
    if (value >= 0.3) return 'rusty';
    return 'weak';
  }

  const pct = (v) => Math.round(v * 100);
  const fmt = (v) => v.toFixed(2);

  // Rung 2: refit the BKT parameters from the attempt log and apply them live.
  async function retrain() {
    if (busy) return;
    busy = true;
    error = null;
    notice = null;
    try {
      const before = model;
      model = await refitModel();
      await refreshMap();
      const changed =
        !before ||
        ['p_init', 'p_learn', 'p_slip', 'p_guess'].some(
          (k) => Math.abs(before[k] - model[k]) > 0.005
        );
      notice = changed
        ? 'Learner model re-fit from your attempt history — updated values are shown below.'
        : 'Re-fit done, but your attempt history is still too small to shift the model. Solve more problems, then retrain.';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

{#snippet conceptChip(id, extra)}
  {#if practiceableIds.has(id)}
    <button class="chip clickable {extra}" onclick={() => practice(id)} title="Practice this">
      {conceptLabel(id)}
    </button>
  {:else}
    <span class="chip muted-chip {extra}" title="No exercises yet">{conceptLabel(id)}</span>
  {/if}
{/snippet}

<div class="app">
  <header>
    <div class="brand">
      <span class="logo">▦</span>
      <div>
        <h1>Lattice</h1>
        {#if subjects.length > 1}
          <select
            class="subject-switch"
            value={currentSubjectId}
            onchange={(e) => switchSubject(e.currentTarget.value)}
            disabled={busy}
          >
            {#each subjects as s}
              <option value={s.id}>{s.name}</option>
            {/each}
          </select>
        {:else}
          <p class="muted">{subject?.name ?? 'Loading…'}</p>
        {/if}
      </div>
    </div>
    <nav>
      <button class:active={view === 'practice'} onclick={() => (view = 'practice')}>
        Practice
      </button>
      <button class:active={view === 'learn'} onclick={() => (view = 'learn')}>
        Learn
      </button>
      <button
        class:active={view === 'map'}
        onclick={() => {
          view = 'map';
          refreshMap();
        }}
      >
        Concept map
      </button>
      <button class:active={view === 'settings'} onclick={() => (view = 'settings')}>
        Settings
      </button>
    </nav>
  </header>

  {#if error}
    <div class="banner error">{error}</div>
  {/if}
  {#if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  {#if view === 'practice'}
    <main class="practice">
      {#if problem}
        <section class="card problem">
          <div class="tags">
            <span class="badge {problem.difficulty}">{problem.difficulty}</span>
            {#if problem.generated_by === 'ai'}<span class="badge ai">AI</span>{/if}
            {#if problem.generated_by === 'static'}<span class="badge curated">curated</span>{/if}
            {#each problem.concepts as c}
              {@render conceptChip(c, '')}
            {/each}
          </div>

          <div class="statement">
            <Katex tex={problem.content} display />
          </div>

          {#if problem.attribution}
            <p class="attribution muted">
              Source: {problem.attribution.source}{#if problem.attribution.license}
                · {problem.attribution.license}{/if}
            </p>
          {/if}

          {#if problemPrereqs.length}
            <div class="builds-on">
              <span class="muted">Builds on:</span>
              {#each problemPrereqs as p}
                {@render conceptChip(p, 'small')}
              {/each}
            </div>
          {/if}

          <form onsubmit={submit}>
            <label for="work">
              Your work <span class="muted">— show the steps, end with your final answer</span>
            </label>
            <textarea
              id="work"
              rows="4"
              bind:value={work}
              placeholder={'e.g.  subtract 1 from both sides → 2x = 6 → x = 3'}
              disabled={busy}
            ></textarea>
            <div class="actions">
              <button type="submit" class="primary" disabled={busy || !work.trim()}>Submit</button>
              <button type="button" onclick={nextInConcept} disabled={busy}>New problem</button>
              <button type="button" class="ghost" onclick={learnCurrentConcept} disabled={busy}>
                Learn this concept
              </button>
              <button type="button" class="ghost" onclick={() => (showSolution = !showSolution)}>
                {showSolution ? 'Hide' : 'Show'} solution
              </button>
            </div>
          </form>

          {#if settings?.has_key}
            <div class="ai-gen">
              <span class="muted">Need a fresh one?</span>
              <select bind:value={aiDifficulty} disabled={generating}>
                <option value="easy">easy</option>
                <option value="medium">medium</option>
                <option value="hard">hard</option>
              </select>
              <button type="button" onclick={generateAi} disabled={generating}>
                {generating ? 'Generating…' : 'Generate (AI)'}
              </button>
            </div>
          {/if}

          {#if showSolution}
            <div class="solution">
              <span class="muted">Solution:</span> <Katex tex={problem.solution} />
            </div>
          {/if}
        </section>

        {#if outcome}
          {#if outcome.is_correct}
            <section class="card verdict correct">
              <h2>✓ Correct</h2>
              <p class="muted">Mastery updated. Ready for the next one?</p>
              <button class="primary" onclick={nextInConcept}>Next problem</button>
            </section>
          {:else}
            <section class="card verdict wrong">
              <h2>Not quite — but here's the useful part</h2>
              {#if outcome.weak_link}
                <p class="diagnosis">
                  The weak link under this problem is
                  <strong>{conceptLabel(outcome.weak_link.concept_id)}</strong>
                  <span class="muted">({REASON[outcome.weak_link.reason] ?? outcome.weak_link.reason})</span>.
                  Estimated mastery there is
                  <strong>{pct(outcome.weak_link.estimated_mastery)}%</strong>.
                </p>
              {/if}

              {#if outcome.cross_weak_links?.length}
                <div class="cross-links">
                  <p class="diagnosis">
                    This also builds on skills in another subject that look shaky —
                    the root cause may be there:
                  </p>
                  {#each outcome.cross_weak_links as l}
                    <button
                      class="chip clickable small cross"
                      onclick={() => crossPractice(l)}
                      title="Practise in {l.subject_name}"
                    >
                      {l.label}
                      <span class="cross-subj">↗ {l.subject_name} · {pct(l.mastery)}%</span>
                    </button>
                  {/each}
                </div>
              {/if}

              {#if diagnosis}
                <div class="ai-diagnosis">
                  <div class="ai-head">
                    <span class="ai-badge">AI</span>
                    <strong>{diagnosis.misconception_label}</strong>
                    <span class="muted">· {pct(diagnosis.confidence)}% confident</span>
                  </div>
                  <p>{diagnosis.explanation}</p>
                </div>
              {/if}

              <div class="actions">
                {#if outcome.practice}
                  <button class="primary" onclick={practiceWeakLink}>
                    Practice {conceptLabel(outcome.weak_link?.concept_id)}
                  </button>
                {/if}
                {#if settings?.has_key && !diagnosis}
                  <button onclick={diagnose} disabled={diagnosing}>
                    {diagnosing ? 'Diagnosing…' : 'Diagnose my mistake (AI)'}
                  </button>
                {:else if !settings?.has_key}
                  <button class="ghost" onclick={() => (view = 'settings')}>
                    Set up AI diagnosis
                  </button>
                {/if}
                <button onclick={nextInConcept}>Try another</button>
              </div>
            </section>
          {/if}
        {/if}
      {:else}
        <p class="muted">{busy ? 'Generating a problem…' : 'No problem loaded.'}</p>
      {/if}
    </main>
  {:else if view === 'map'}
    <main class="map">
      <p class="muted intro">
        Decay-adjusted mastery across the prerequisite graph, grouped by area.
      </p>

      {#if model}
        <section class="model-card">
          <div class="model-head">
            <div>
              <strong>Learner model</strong>
              <span class="muted">— Bayesian Knowledge Tracing</span>
            </div>
            <button onclick={retrain} disabled={busy}>Retrain from history</button>
          </div>
          <div class="model-params">
            <span><span class="muted">init</span> {fmt(model.p_init)}</span>
            <span><span class="muted">learn</span> {fmt(model.p_learn)}</span>
            <span><span class="muted">slip</span> {fmt(model.p_slip)}</span>
            <span><span class="muted">guess</span> {fmt(model.p_guess)}</span>
          </div>
          <p class="muted model-caption">
            How the tutor models learning (Bayesian Knowledge Tracing): the chance you
            already know a concept, how fast you pick it up, how often you slip on one
            you know, and how often a guess is lucky. <strong>Retrain from history</strong>
            re-estimates these four from the answers you've logged — it needs a fair
            number of attempts before the numbers move much.
          </p>
        </section>
      {/if}
      {#each grouped as group}
        <section class="group">
          <div class="group-head">
            <h3>{group.name}</h3>
            <span class="group-avg {masteryClass(groupAvgRaw(group.items))}">
              {pct(groupAvgRaw(group.items))}%
            </span>
          </div>
          <ul class="concepts">
            {#each group.items as c}
              <li class="concept">
                <div class="concept-head">
                  {#if c.practiceable}
                    <button
                      class="concept-label link"
                      onclick={() => practice(c.id)}
                      title="Practice this"
                    >
                      {c.label}
                    </button>
                  {:else}
                    <span class="concept-label">{c.label}</span>
                  {/if}
                  <span class="pctlabel {masteryClass(c.estimated_mastery)}">
                    {pct(c.estimated_mastery)}%
                  </span>
                </div>
                <div class="bar">
                  <div
                    class="fill {masteryClass(c.estimated_mastery)}"
                    style="width:{Math.max(2, c.estimated_mastery * 100)}%"
                  ></div>
                </div>
                {#if c.prerequisites.length}
                  <div class="prereqs">
                    <span class="muted">needs</span>
                    {#each c.prerequisites as p}
                      {@render conceptChip(p, 'small')}
                    {/each}
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </main>
  {:else if view === 'learn'}
    <main class="learn">
      {#if !learnSel}
        <p class="muted intro">
          Learn each concept, then practice it. Lessons are original notes (Markdown + math)
          stored with the subject — write your own, or draft one with AI and edit it.
        </p>
        {#each grouped as group}
          <section class="group">
            <div class="group-head">
              <h3>{group.name}</h3>
            </div>
            <ul class="lesson-list">
              {#each group.items as c}
                <li>
                  <button class="lesson-row" onclick={() => openLesson(c.id)}>
                    <span class="lesson-name">{c.label}</span>
                    {#if c.has_notes}
                      <span class="lesson-tag has">📖 lesson</span>
                    {:else}
                      <span class="lesson-tag none">no lesson yet</span>
                    {/if}
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/each}
      {:else}
        <div class="lesson-detail">
          <button class="ghost back" onclick={closeLesson}>← All concepts</button>

          {#if lessonBusy}
            <p class="muted">Loading lesson…</p>
          {:else if lessonData}
            <header class="lesson-header">
              <div>
                <span class="muted lesson-group">{lessonData.group}</span>
                <h2>{lessonData.label}</h2>
              </div>
              {#if lessonData.practiceable}
                <button class="primary" onclick={practiceFromLesson}>Practice this concept</button>
              {/if}
            </header>

            {#if lessonData.prerequisites.length || externalPrereqs.length}
              <div class="builds-on">
                <span class="muted">Builds on:</span>
                {#each lessonData.prerequisites as p}
                  {@render conceptChip(p, 'small')}
                {/each}
                {#each externalPrereqs as r}
                  <button
                    class="chip clickable small cross"
                    onclick={() => crossPractice(r)}
                    title="Practise in {r.subject_name}"
                  >
                    {r.label} <span class="cross-subj">↗ {r.subject_name}</span>
                  </button>
                {/each}
              </div>
            {/if}

            {#if editing}
              <section class="card lesson-editor">
                <label for="draft" class="field-label">
                  Lesson &amp; notes
                  <span class="muted">
                    — your Markdown. Math as $inline$ / $$display$$; a
                    <code>---</code> frontmatter block with <code>source:</code> /
                    <code>license:</code> credits a textbook.
                  </span>
                </label>
                <textarea
                  id="draft"
                  rows="22"
                  bind:value={draft}
                  onkeydown={editorKeydown}
                  placeholder={'# Title\n\nWrite the concept in your own words, paste notes from what you’re reading, add $x^2$ math…'}
                ></textarea>
                <div class="actions">
                  <button
                    class="primary"
                    onclick={saveLessonDraft}
                    disabled={savingLesson || !draft.trim() || !lessonDirty}
                  >
                    {savingLesson ? 'Saving…' : 'Save'}
                  </button>
                  <button class="ghost" onclick={() => (editing = false)} disabled={savingLesson}>
                    {lessonDirty ? 'Discard changes' : 'Close'}
                  </button>
                  {#if settings?.has_key}
                    <button onclick={draftWithAi} disabled={drafting}>
                      {drafting ? 'Drafting…' : 'Re-draft with AI'}
                    </button>
                  {/if}
                  {#if lessonDirty}
                    <span class="muted dirty">• unsaved changes <kbd>⌘S</kbd></span>
                  {:else if lessonSaved}
                    <span class="key-set">Saved ✓</span>
                  {/if}
                </div>
                {#if draft.trim()}
                  <div class="preview">
                    <span class="muted preview-label">Preview</span>
                    <Lesson source={draft} />
                  </div>
                {/if}
              </section>
            {:else if lessonData.notes}
              <article class="card lesson-content">
                <Lesson source={lessonData.notes} />
                {#if lessonData.source}
                  <p class="attribution muted">
                    Adapted from {lessonData.source}{#if lessonData.license}
                      · {lessonData.license}{/if}
                  </p>
                {/if}
              </article>
              <div class="actions">
                <button class="ghost" onclick={startEdit}>Edit / add notes</button>
              </div>
            {:else}
              <section class="card empty-lesson">
                <p>No lesson written for <strong>{lessonData.label}</strong> yet.</p>
                <div class="actions">
                  {#if settings?.has_key}
                    <button class="primary" onclick={draftWithAi} disabled={drafting}>
                      {drafting ? 'Drafting…' : 'Draft with AI'}
                    </button>
                  {/if}
                  <button onclick={startEdit}>Write it myself</button>
                </div>
                {#if !settings?.has_key}
                  <p class="muted hint">Tip: add an API key in Settings to draft lessons with AI.</p>
                {/if}
              </section>
            {/if}
          {/if}
        </div>
      {/if}
    </main>
  {:else}
    <main class="settings">
      <p class="muted intro">
        Bring your own API key for AI misconception diagnosis (Phase 2). Your key is
        stored in the OS keychain, never on disk, and only sent to the provider you pick.
      </p>

      {#if settings}
        <section class="card">
          <div class="field">
            <span class="field-label">Provider</span>
            <div class="provider-row">
              {#each settings.providers as p}
                <button
                  type="button"
                  class="provider-pill"
                  class:active={selProvider === p.id}
                  onclick={() => chooseProvider(p.id)}
                >
                  {p.label}
                </button>
              {/each}
            </div>
          </div>

          <label class="field">
            <span class="field-label">Model</span>
            <input type="text" bind:value={selModel} placeholder="model id" />
          </label>

          <label class="field">
            <span class="field-label">
              API key
              {#if settings.has_key && selProvider === settings.provider}
                <span class="key-set">— a key is saved</span>
              {/if}
            </span>
            <input
              type="password"
              bind:value={keyDraft}
              autocomplete="off"
              placeholder={settings.has_key && selProvider === settings.provider
                ? 'leave blank to keep the saved key'
                : 'paste your API key'}
            />
          </label>

          <div class="actions">
            <button class="primary" onclick={saveSettings}>Save</button>
            {#if settingsSaved}<span class="key-set">Saved ✓</span>{/if}
          </div>
        </section>
      {/if}
    </main>
  {/if}
</div>
