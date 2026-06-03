export const meta = {
  name: 'author-archetype-combo-tests',
  description: 'Author multi-card interaction (combo) tests for implemented Rust archetypes',
  whenToUse:
    'After an archetype family has its cards implemented in Rust (DSL) and per-card tests are green, to add the cross-card combo tests the per-card TDD misses. Discovers which archetypes are implemented (coverage gate), then fans out the /archetype-interaction-test-author skill across them.',
  phases: [
    { title: 'Discover', detail: 'list archetypes + run the coverage gate to find implemented ones' },
    { title: 'Model', detail: 'scout each archetype as a system → model doc + ranked combos (Sonnet)' },
    { title: 'Author', detail: 'precondition-gate combos, then author tests/archetypes/<slug>.rs (Sonnet)' },
    { title: 'Review', detail: 'audit model + tests vs card text / rules / DCGO (Opus)' },
    { title: 'Revise', detail: 'fix blocker/major review findings (model doc, synthetic stand-ins, bypassed abilities)' },
    { title: 'Register & Run', detail: 'register mods in main.rs, run the suite + static harness (orchestrator-owned)' },
    { title: 'Triage', detail: 'confirm each failing test vs sources, route to gap trackers' },
  ],
};

// ── Tunables (all overridable via the `args` object) ─────────────────────────
// args = {
//   archetypes?: string[],  // explicit archetype names; skips discovery
//   threshold?: number,     // coverage-gate fraction to count as "implemented" (default 0.85)
//   minShare?: number,      // meta-share floor for discovery candidates (default 1.0 %)
//   topN?: number,          // cap on how many implemented archetypes to process (default 8)
//   comboCap?: number,      // cap on interaction tests authored per archetype (default 5)
//   focus?: string,         // emphasis woven into combo ranking (e.g. prioritise Option cards)
// }
// Tolerate `args` arriving as a JSON-encoded string (some launch paths stringify it).
const A = typeof args === 'string' ? (() => { try { return JSON.parse(args); } catch (e) { return {}; } })() : args || {};
const EXPLICIT = Array.isArray(A.archetypes) ? A.archetypes : null;
const THRESHOLD = typeof A.threshold === 'number' ? A.threshold : 0.85;
const MIN_SHARE = typeof A.minShare === 'number' ? A.minShare : 1.0;
const TOP_N = typeof A.topN === 'number' ? A.topN : 8;
const COMBO_CAP = typeof A.comboCap === 'number' ? A.comboCap : 5;
const FOCUS = typeof A.focus === 'string' && A.focus.trim() ? A.focus.trim() : null;
const FOCUS_DIRECTIVE = FOCUS
  ? `\nFOCUS (overrides default ranking emphasis): ${FOCUS}\nWhen ranking combos, give matching interactions the highest rank, and ensure they are represented within the per-archetype cap. Still only author combos whose pieces are all implemented.\n`
  : '';

// Rust module names can't contain dashes — derive an underscore slug used for
// BOTH the test file (tests/archetypes/<slug>.rs) and its `mod <slug>;` line.
function slugify(name) {
  return String(name).toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '');
}

const COMMON = `
You are working in the Digimon TCG simulator repo. Follow the conventions in the
\`/archetype-interaction-test-author\` skill — read \`.claude/skills/archetype-interaction-test-author/SKILL.md\`
first; it is the authoritative spec for this work.

Key facts:
- Source priority (CLAUDE.md): official \`Digimon TCG resources/general_rule.pdf\` (canonical, keyword
  semantics in §16) + DCGO C# (battle-tested) OUTRANK the card-text JSON. DCGO lives in the BASE repo —
  resolve it with: BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
  (C# at $BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs, underscores: BT17-102 -> BT17_102.cs).
  NEVER run \`git submodule update --init DCGO\` in this worktree (rule 29).
- Resolve a card pool: \`python code/tools/resolve_deck.py "<archetype>" --json\`.
- Static harness: \`cargo run -p archetype-static-tests -- "<archetype>" [--threshold F --smoke-games N
  --combo "name=A,B" --json --no-write]\` — emits the four invariants and (without --no-write) records
  qa/qa-reports/archetype_interactions.json.
- Interaction tests live in \`code/digimon-engine/tests/archetypes/<slug>.rs\`, fixtures in
  \`tests/archetypes/support.rs\` (dsl_builder, snapshot/BoardSnapshot, run_actions); the exemplar is
  \`tests/archetypes/rocks.rs\`. Run the suite:
  \`cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes\`.
- Per-card verdicts (read-only): qa/qa-reports/validated_cards_dsl.json.
- Use the Pinecone \`digimon-engine\` index (card-scripts / engine-api / card-metadata / rules-docs) for retrieval.
Your final message IS your return value (structured per the schema) — return data, not prose.
`;

// Appended to every schema-constrained prompt: the run discards work if the agent
// ends without the final tool call, so make the requirement impossible to miss.
const FINAL_CALL =
  '\n\nCRITICAL: your FINAL action MUST be the StructuredOutput tool call returning the required schema. Do NOT end your turn with a prose summary, and do NOT stop early — if you finish without calling StructuredOutput, ALL of your work this turn is discarded. Keep the returned payload concise: durable detail belongs in the files you wrote on disk, not in the return value.';

// ── Schemas ──────────────────────────────────────────────────────────────────
const DISCOVERY_SCHEMA = {
  type: 'object',
  required: ['archetypes'],
  properties: {
    archetypes: {
      type: 'array',
      items: {
        type: 'object',
        required: ['name'],
        properties: { name: { type: 'string' }, share: { type: 'number' } },
      },
    },
  },
};

const COVERAGE_SCHEMA = {
  type: 'object',
  required: ['archetype', 'implemented', 'ratio'],
  properties: {
    archetype: { type: 'string' },
    implemented: { type: 'boolean' },
    ratio: { type: 'number' },
    implementedCount: { type: 'number' },
    total: { type: 'number' },
    note: { type: 'string' },
  },
};

const SCOUT_SCHEMA = {
  type: 'object',
  required: ['canonicalName', 'modelPath', 'combos'],
  properties: {
    canonicalName: { type: 'string' },
    modelPath: { type: 'string' },
    combos: {
      type: 'array',
      items: {
        type: 'object',
        required: ['name', 'cards', 'expectedOutcome'],
        properties: {
          name: { type: 'string' },
          cards: { type: 'array', items: { type: 'string' } },
          expectedOutcome: { type: 'string' },
          sources: { type: 'string' },
          rank: { type: 'number' },
        },
      },
    },
    dropped: {
      type: 'array',
      items: {
        type: 'object',
        required: ['name', 'reason'],
        properties: { name: { type: 'string' }, reason: { type: 'string' } },
      },
    },
  },
};

const AUTHOR_SCHEMA = {
  type: 'object',
  required: ['testFile', 'testsAuthored'],
  properties: {
    testFile: { type: 'string' },
    testsAuthored: {
      type: 'array',
      items: {
        type: 'object',
        required: ['combo', 'fnName'],
        properties: { combo: { type: 'string' }, fnName: { type: 'string' } },
      },
    },
    blockedCombos: {
      type: 'array',
      items: {
        type: 'object',
        required: ['combo'],
        properties: {
          combo: { type: 'string' },
          missingCards: { type: 'array', items: { type: 'string' } },
        },
      },
    },
    notes: { type: 'string' },
  },
};

const REVIEW_SCHEMA = {
  type: 'object',
  required: ['verdict', 'modelOk', 'testsOk'],
  properties: {
    verdict: { type: 'string', enum: ['approve', 'revise'] },
    modelOk: { type: 'boolean' },
    testsOk: { type: 'boolean' },
    issues: {
      type: 'array',
      items: {
        type: 'object',
        required: ['problem'],
        properties: {
          combo: { type: 'string' },
          problem: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
        },
      },
    },
  },
};

const REGISTER_SCHEMA = {
  type: 'object',
  required: ['registered', 'compileOk', 'suiteResults'],
  properties: {
    registered: { type: 'array', items: { type: 'string' } },
    compileOk: { type: 'boolean' },
    excludedModules: { type: 'array', items: { type: 'string' } },
    suiteResults: {
      type: 'array',
      items: {
        type: 'object',
        required: ['slug', 'test', 'passed'],
        properties: {
          slug: { type: 'string' },
          test: { type: 'string' },
          passed: { type: 'boolean' },
          message: { type: 'string' },
        },
      },
    },
    staticHarness: {
      type: 'array',
      items: {
        type: 'object',
        properties: { archetype: { type: 'string' }, allGatesPassed: { type: 'boolean' } },
      },
    },
  },
};

const TRIAGE_SCHEMA = {
  type: 'object',
  required: ['test', 'isEngineBug', 'summary'],
  properties: {
    test: { type: 'string' },
    isEngineBug: { type: 'boolean' },
    cause: { type: 'string', enum: ['engine_bug', 'wrong_model', 'wrong_test', 'flaky', 'unknown'] },
    tracker: { type: 'string' },
    entryWritten: { type: 'boolean' },
    summary: { type: 'string' },
  },
};

// ── Phase: Discover implemented archetypes ───────────────────────────────────
phase('Discover');
let candidates;
if (EXPLICIT) {
  candidates = EXPLICIT.map((name) => ({ name }));
  log(`Using ${candidates.length} explicit archetype(s): ${EXPLICIT.join(', ')}`);
} else {
  const disc = await agent(
    `${COMMON}\nTASK: list the archetypes worth considering for interaction-test authoring.\n` +
      `Run: \`python code/tools/resolve_deck.py --list-archetypes --min-share ${MIN_SHARE}\`.\n` +
      `Return every listed archetype (name + meta share %), preserving the meta-share order (highest first).`,
    { schema: DISCOVERY_SCHEMA, label: 'list-archetypes', phase: 'Discover' }
  );
  candidates = (disc && disc.archetypes) || [];
  log(`Discovery surfaced ${candidates.length} candidate archetype(s) at >= ${MIN_SHARE}% meta share.`);
}

if (!candidates.length) {
  log('No candidate archetypes — nothing to do.');
  return { selected: [], reason: 'no candidates' };
}

// Bound the discovery fan-out: at minShare 0 the library can list ~300 archetypes,
// and one coverage agent per entry both wastes tokens and saturates concurrency
// (the failure mode of an earlier full run). Cap how many we coverage-check.
const MAX_CHECK = 60;
if (!EXPLICIT && candidates.length > MAX_CHECK) {
  log(`Bounding coverage check to the top ${MAX_CHECK} of ${candidates.length} candidates (by meta share); the long tail is unlikely to clear the bar.`);
  candidates = candidates.slice(0, MAX_CHECK);
}

let implemented = [];
if (EXPLICIT) {
  // Caller asserted the list — trust it and skip the coverage gate entirely.
  implemented = candidates.map((c) => ({ name: c.name, slug: slugify(c.name), ratio: null }));
  log(`Explicit mode: skipping the coverage gate; treating all ${implemented.length} listed archetype(s) as implemented.`);
} else {
  // Coverage gate per candidate — "implemented" = unique-card coverage >= THRESHOLD.
  const coverage = await parallel(
    candidates.map((c) => () =>
      agent(
        `${COMMON}\nTASK: decide whether the archetype "${c.name}" is IMPLEMENTED enough for interaction tests.\n` +
          `Run: \`cargo run -p archetype-static-tests -- "${c.name}" --threshold ${THRESHOLD} --smoke-games 0 --no-write --json\`\n` +
          `(if --smoke-games 0 errors, retry without it). Read the JSON's coverage_gate result.\n` +
          `Return { archetype, implemented: coverage_gate.passed, ratio, implementedCount, total }.` +
          FINAL_CALL,
        { schema: COVERAGE_SCHEMA, label: `coverage:${c.name}`, phase: 'Discover' }
      )
    )
  );
  candidates.forEach((c, i) => {
    const r = coverage[i];
    if (r && r.implemented) implemented.push({ name: c.name, slug: slugify(c.name), ratio: r.ratio });
  });
}

if (!implemented.length) {
  log('No archetype cleared the coverage gate — none are implemented enough to test.');
  return { selected: [], coverage: coverage.filter(Boolean) };
}

const selected = implemented.slice(0, TOP_N);
log(`Implemented (coverage >= ${THRESHOLD}): ${implemented.map((x) => x.name).join(', ')}`);
if (implemented.length > TOP_N) {
  log(`Capping to top ${TOP_N} by meta share. DEFERRED: ${implemented.slice(TOP_N).map((x) => x.name).join(', ')}`);
}

// ── Phase: Model (scout each archetype as a system) ──────────────────────────
phase('Model');
const scouts = (
  await parallel(
    selected.map((a) => () =>
      agent(
        `${COMMON}\nTASK: Phases 0-3 of the skill for the archetype "${a.name}".\n` +
          `1. Resolve the pool (\`resolve_deck.py "${a.name}" --json\`).\n` +
          `2. Research the meaningful cards: printed text + DCGO C# ($BASE_DCGO) + relevant general_rule.pdf §16 keyword/timing rules; retrieve prior context from Pinecone.\n` +
          `3. WRITE the durable model doc to \`qa/archetype-qa/${a.name}-model.md\` in the fixed structure from the skill (card pool & roles, digivolution lines, named combos with cards + expected mechanical outcome + rules/DCGO sources + rank, playstyle, win conditions, ranked interactions). This file is a MANDATORY deliverable: after writing it, re-Read it to confirm it exists and is non-empty. Do NOT reuse or cite a pre-existing per-card faithfulness doc (e.g. \`${a.name}.md\`) as the model — create the \`-model.md\` system model. Returning combos WITHOUT having written \`${a.name}-model.md\` is an incomplete task.\n` +
          `4. Return up to ${COMBO_CAP} top-ranked combos (by play-frequency x payoff-centrality). For EACH: name, cards (card IDs), the precise expected mechanical outcome (a checkable board diff), sources (general_rule.pdf §/DCGO path). List any combos you ranked but dropped under the cap in \`dropped\`.\n` +
          FOCUS_DIRECTIVE +
          `Cite sources inline in the model doc. Do NOT author any Rust test yet. canonicalName must be the resolved canonical archetype name.` +
          FINAL_CALL,
        { schema: SCOUT_SCHEMA, label: `model:${a.name}`, phase: 'Model' }
      ).then((s) => (s ? { ...a, scout: s } : null))
    )
  )
).filter(Boolean);

// Consolidated plan (the skill's plan-approval gate). The user opted into this
// run; this log makes the plan reviewable and is a natural interrupt point.
log('PLAN — interaction tests to author (interrupt now to adjust scope):');
scouts.forEach((s) => {
  const combos = s.scout.combos.map((c) => c.name).join('; ') || '(none)';
  const dropped = (s.scout.dropped || []).map((d) => d.name).join(', ');
  log(`  ${s.scout.canonicalName} [${s.slug}.rs]: ${combos}${dropped ? `  (dropped: ${dropped})` : ''}`);
});

const withCombos = scouts.filter((s) => s.scout.combos && s.scout.combos.length);
if (!withCombos.length) {
  log('No archetype produced authorable combos.');
  return { selected: selected.map((a) => a.name), scouts: scouts.map((s) => s.scout) };
}

// ── Phases: Author (gate + write file) → Review (audit) — pipelined ──────────
const built = await pipeline(
  withCombos,
  // Stage 1: precondition-gate the combos, then author tests/archetypes/<slug>.rs
  (a) =>
    agent(
      `${COMMON}\nTASK: Phases 4-5 of the skill for "${a.scout.canonicalName}" (slug "${a.slug}").\n` +
        `Proposed combos (from the model doc qa/archetype-qa/${a.scout.canonicalName}-model.md):\n` +
        a.scout.combos.map((c) => `  - ${c.name}: cards [${c.cards.join(', ')}] — expect: ${c.expectedOutcome}`).join('\n') +
        `\n\nSTEP A (precondition gate): run \`cargo run -p archetype-static-tests -- "${a.scout.canonicalName}" ` +
        a.scout.combos.map((c) => `--combo "${c.name}=${c.cards.join(',')}"`).join(' ') +
        ` --threshold ${THRESHOLD} --json\`. Any combo that names an UNIMPLEMENTED card is BLOCKED — do NOT author its test; record it in blockedCombos and route the missing card to the gap trackers per the skill.\n` +
        `STEP B (author): for each surviving combo write ONE #[test] in \`code/digimon-engine/tests/archetypes/${a.slug}.rs\`, modeled on tests/archetypes/rocks.rs and using support.rs fixtures (dsl_builder, snapshot/BoardSnapshot, run_actions). Assert the claimed mechanical outcome via a before/after BoardSnapshot diff. Include the unhappy/enabler-absent path where it expresses a system-level fact. Each test carries a doc-comment traceable to the model combo (name, cards, expected outcome, sources).\n` +
        `FAITHFULNESS RULES (the reviewer rejects violations): (a) NEVER substitute a synthetic make_test_card for a card the combo NAMES — load EVERY named card (enablers, payoffs, keyword sources) by its real ID via dsl_card/dsl_builder. Synthetic make_test_card is allowed ONLY for unnamed neutral fillers/targets. (b) Exercise each card's ACTUAL ability — play the Option through its real action, trigger the keyword via the real trigger path — do NOT call a low-level engine helper (e.g. \`delete_permanent_with_effects\`) that bypasses the named card's own effect, or the combo is not really tested. (c) Do NOT weaken an assertion to make it pass; if the faithful outcome can't be produced, mark the test \`#[ignore]\` with the reason and route the gap to the trackers.\n` +
        `DO NOT edit \`tests/archetypes/main.rs\` — module registration is the orchestrator's job (a later step adds \`mod ${a.slug};\`). You therefore cannot run this suite yourself; just write a clean, compilable file.\n` +
        `Return testFile, testsAuthored [{combo, fnName}], blockedCombos, notes.` +
        FINAL_CALL,
      { schema: AUTHOR_SCHEMA, label: `author:${a.slug}`, phase: 'Author' }
    ).then((au) => (au ? { ...a, author: au } : null)),
  // Stage 2: Opus review of the model + the authored tests (read-only audit)
  (prev, a) => {
    if (!prev || !prev.author) return prev;
    return agent(
      `${COMMON}\nTASK: Phase 6 audit (the Reviewer wave) for "${a.scout.canonicalName}".\n` +
        `Read the model doc \`qa/archetype-qa/${a.scout.canonicalName}-model.md\` and the test file \`${prev.author.testFile}\`.\n` +
        `Audit BOTH against the card text + general_rule.pdf + DCGO C# ($BASE_DCGO). Catch: a wrong combo claim, a test asserting the wrong outcome, an over-/under-specified DP window, a missed "may"/"by-cost"/"or" nuance, a synthetic card standing in for a real combo piece.\n` +
        `Verdict "approve" only if both the model and every test faithfully match the sources. Return verdict, modelOk, testsOk, issues [{combo, problem, severity}].`,
      { schema: REVIEW_SCHEMA, model: 'opus', label: `review:${a.slug}`, phase: 'Review' }
    ).then((rev) => ({ ...prev, review: rev }));
  },
  // Stage 3: act on the review — if it said "revise" with blocker/major issues,
  // dispatch a fix agent. (A bare unregistered-`mod` blocker is ignored: the
  // Register & Run phase below owns main.rs and adds it.)
  (prev, a) => {
    if (!prev || !prev.author || !prev.review || prev.review.verdict !== 'revise') return prev;
    const issues = (prev.review.issues || []).filter((i) => {
      if (i.severity !== 'blocker' && i.severity !== 'major') return false;
      const p = (i.problem || '').toLowerCase();
      // The "not registered in main.rs" finding is resolved by the registrar; skip it.
      return !(p.includes('main.rs') || p.includes('mod ') || p.includes('never run') || p.includes('never executes'));
    });
    if (!issues.length) return prev;
    return agent(
      `${COMMON}\nTASK: REVISE the interaction tests + model for "${a.scout.canonicalName}" (slug "${a.slug}") to resolve the reviewer's blocker/major findings. Do NOT re-implement any card or weaken assertions.\n` +
        `File: \`${prev.author.testFile}\`. Model doc: \`qa/archetype-qa/${a.scout.canonicalName}-model.md\`.\n` +
        `Reviewer findings to fix:\n` +
        issues.map((i) => `  - [${i.severity}] ${i.combo ? i.combo + ': ' : ''}${i.problem}`).join('\n') +
        `\n\nApply the FAITHFULNESS RULES: write the \`-model.md\` system model if it is missing (do not cite a per-card doc); replace any synthetic stand-in for a NAMED combo card with the real card by ID; exercise the card's ACTUAL ability rather than a bypassing engine helper. If a faithful outcome genuinely can't be produced, \`#[ignore]\` the test with the reason and file the gap. Do NOT edit \`tests/archetypes/main.rs\`.\n` +
        `Return the updated testFile, testsAuthored, blockedCombos, notes (summarise what you changed).` +
        FINAL_CALL,
      { schema: AUTHOR_SCHEMA, label: `revise:${a.slug}`, phase: 'Revise' }
    ).then((fix) => (fix ? { ...prev, author: fix, revised: true } : prev));
  }
);

const authored = built.filter((b) => b && b.author && b.author.testFile);
if (!authored.length) {
  log('No test files were authored (all combos blocked or authoring failed).');
  return { selected: selected.map((a) => a.name), scouts: scouts.map((s) => s.scout) };
}

// ── Phase: Register & Run (orchestrator-owned, single agent → no main.rs race)
phase('Register & Run');
const reg = await agent(
  `${COMMON}\nTASK: register the new interaction-test modules and run the suite.\n` +
    `New files to register in \`code/digimon-engine/tests/archetypes/main.rs\` (add a \`mod <slug>;\` line under the "Per-archetype interaction suites" section if not already present):\n` +
    authored.map((a) => `  - mod ${a.slug};  (file ${a.author.testFile})`).join('\n') +
    `\n\nThen run \`cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes\`.\n` +
    `If the binary FAILS TO COMPILE, bisect: comment out the most recently added \`mod\` lines one at a time (or use #[path]) to isolate which file(s) don't compile, leave the good ones registered, and report the broken slugs in excludedModules with the compiler error in the matching suiteResults message.\n` +
    `Also run the static harness per archetype to refresh the verdict tracker (writes qa/qa-reports/archetype_interactions.json), passing each archetype's combos: ` +
    authored
      .map(
        (a) =>
          `\`cargo run -p archetype-static-tests -- "${a.scout.canonicalName}" ` +
          a.scout.combos.map((c) => `--combo "${c.name}=${c.cards.join(',')}"`).join(' ') +
          ` --threshold ${THRESHOLD}\``
      )
      .join('; ') +
    `\nReturn registered (slugs), compileOk, excludedModules, suiteResults [{slug, test, passed, message}] for every #[test] in the new files, and staticHarness [{archetype, allGatesPassed}].`,
  { schema: REGISTER_SCHEMA, label: 'register-and-run', phase: 'Register & Run' }
);

// ── Phase: Triage failing tests (confirm vs sources, route to trackers) ──────
const failures = ((reg && reg.suiteResults) || []).filter((r) => !r.passed);
let triaged = [];
if (failures.length) {
  phase('Triage');
  log(`${failures.length} failing interaction test(s) — confirming each against card text + rules + DCGO.`);
  triaged = (
    await parallel(
      failures.map((f) => () =>
        agent(
          `${COMMON}\nTASK: triage the FAILING interaction test \`${f.test}\` (archetype slug ${f.slug}).\n` +
            `Failure message: ${f.message || '(see suite output)'}\n` +
            `Treat the failure as a CANDIDATE engine bug, NOT a test to weaken. Confirm the discrepancy against the card's printed text, general_rule.pdf, and DCGO C# ($BASE_DCGO) exactly like a replay divergence.\n` +
            `Decide the cause: engine_bug | wrong_model (model doc was wrong — note the fix needed) | wrong_test (assertion wrong — note the fix) | flaky | unknown.\n` +
            `If it is a CONFIRMED engine bug, route it: an engine-PRIMITIVE gap -> append to \`docs/RUST_ENGINE_GAPS.md\`; a card-effect faithfulness gap -> append to \`qa/archetype-qa/engine-gaps.md\`. Cite the combo, the test, and the source consulted. DO NOT edit engine code.\n` +
            `Return test, isEngineBug, cause, tracker (path written, if any), entryWritten, summary.`,
          { schema: TRIAGE_SCHEMA, label: `triage:${f.slug}`, phase: 'Triage' }
        )
      )
    )
  ).filter(Boolean);
}

// ── Final summary ────────────────────────────────────────────────────────────
return {
  selectedArchetypes: selected.map((a) => a.name),
  authored: authored.map((a) => ({
    archetype: a.scout.canonicalName,
    file: a.author.testFile,
    tests: a.author.testsAuthored,
    blocked: a.author.blockedCombos || [],
    review: a.review || null,
  })),
  registration: reg || null,
  failures: failures.map((f) => f.test),
  triage: triaged,
};
