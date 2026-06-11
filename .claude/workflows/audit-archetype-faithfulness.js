export const meta = {
  name: 'audit-archetype-faithfulness',
  description: 'Audit implemented archetypes for TCG faithfulness via multi-card combo verification',
  whenToUse:
    'When you want to AUDIT how faithfully implemented archetypes reproduce the real Digimon TCG — re-verifying the combo/interaction tests that already exist in tests/archetypes/, cross-examining combo behavior against card text + general_rule.pdf + DCGO, filling coverage gaps for top combos with no test, and emitting per-archetype faithfulness verdicts. For greenfield combo-test authoring on archetypes with no suite yet, prefer the author-archetype-combo-tests workflow.',
  phases: [
    { title: 'Discover', detail: 'list archetypes + coverage gate to find implemented ones' },
    { title: 'Inventory', detail: 'map existing model docs + interaction suites, run the current archetypes suite + static gates' },
    { title: 'Model', detail: 'create or refresh each archetype system model → ranked combos' },
    { title: 'Audit', detail: 'cross-examine existing tests + combo claims vs card text / rules / DCGO', model: 'opus' },
    { title: 'Author', detail: 'gap-fill: author tests for top combos with no coverage' },
    { title: 'Review', detail: 'audit newly authored tests vs sources', model: 'opus' },
    { title: 'Revise', detail: 'fix blocker/major review findings in new tests' },
    { title: 'Register & Run', detail: 'register new modules, run the suite + static harness (orchestrator-owned)' },
    { title: 'Triage', detail: 'confirm failures + divergences vs sources, route to gap trackers', model: 'opus' },
    { title: 'Report', detail: 'per-archetype faithfulness verdicts + dated audit report' },
  ],
};

// ── Tunables (all overridable via the `args` object) ─────────────────────────
// args = {
//   archetypes?: string[],   // explicit archetype names; skips discovery + coverage gate
//   threshold?: number,      // coverage-gate fraction to count as "implemented" (default 0.85)
//   minShare?: number,       // meta-share floor for discovery candidates (default 1.0 %)
//   topN?: number,           // cap on how many archetypes to audit per run (default 6)
//   comboCap?: number,       // ranked combos audited per archetype (default 6)
//   authorMissing?: boolean, // gap-fill tests for untested combos (default true)
//   focus?: string,          // emphasis woven into combo ranking (e.g. "Option card timing")
// }
// Tolerate `args` arriving as a JSON-encoded string (some launch paths stringify it).
const A = typeof args === 'string' ? (() => { try { return JSON.parse(args); } catch (e) { return {}; } })() : args || {};
const EXPLICIT = Array.isArray(A.archetypes) ? A.archetypes : null;
const THRESHOLD = typeof A.threshold === 'number' ? A.threshold : 0.85;
const MIN_SHARE = typeof A.minShare === 'number' ? A.minShare : 1.0;
const TOP_N = typeof A.topN === 'number' ? A.topN : 6;
const COMBO_CAP = typeof A.comboCap === 'number' ? A.comboCap : 6;
const AUTHOR_MISSING = A.authorMissing !== false;
const FOCUS = typeof A.focus === 'string' && A.focus.trim() ? A.focus.trim() : null;
const FOCUS_DIRECTIVE = FOCUS
  ? `\nFOCUS (overrides default ranking emphasis): ${FOCUS}\nWhen ranking combos, give matching interactions the highest rank and ensure they are represented within the per-archetype cap.\n`
  : '';

// Rust module names can't contain dashes — derive an underscore slug used for
// any NEW test file (existing suites keep whatever slug they already have).
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
  NEVER run \`git submodule update --init DCGO\` in this worktree (rule 29). $BASE_DCGO is READ-ONLY:
  do NOT cd there or treat the base repo as your project root — write all files relative to your
  starting cwd (verify \`git rev-parse --show-toplevel\` is your starting worktree before writing).
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

// Shared faithfulness contract — referenced by the audit, author, and review prompts
// so all three waves judge tests by the same bar.
const FAITHFULNESS_RULES =
  'FAITHFULNESS RULES: (a) Tests must use REAL implemented DSL cards for EVERY role — named combo pieces (enablers, payoffs, keyword sources) AND fillers, neutral targets, opponents, and stack bases — loaded by real card ID via dsl_card/dsl_builder. Synthetic make_test_card is a LAST RESORT, allowed only when NO real implemented DSL card can fill a role (then a one-line comment names the role + why); filler/target roles prefer effectless/vanilla real DSL Digimon so their effects cannot perturb the asserted outcome. (b) Each card\'s ACTUAL ability must be exercised — the Option played through its real action, the keyword fired via its real trigger path — never a low-level engine helper (e.g. delete_permanent_with_effects) that bypasses the named card\'s own effect. (c) Assertions must match the outcome the SOURCES claim — an assertion weakened to pass, or one asserting a plausible-but-wrong outcome, is a faithfulness divergence even when the test is green. (d) If the faithful outcome cannot be produced, the test is #[ignore]d with the reason and the gap routed to the trackers.';

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

const INVENTORY_SCHEMA = {
  type: 'object',
  required: ['archetypes'],
  properties: {
    archetypes: {
      type: 'array',
      items: {
        type: 'object',
        required: ['name', 'hasTestFile', 'hasModelDoc'],
        properties: {
          name: { type: 'string' },
          hasTestFile: { type: 'boolean' },
          testFile: { type: 'string' },
          slug: { type: 'string' },
          hasModelDoc: { type: 'boolean' },
          modelDoc: { type: 'string' },
          suiteResults: {
            type: 'array',
            items: {
              type: 'object',
              required: ['test', 'passed'],
              properties: {
                test: { type: 'string' },
                passed: { type: 'boolean' },
                message: { type: 'string' },
              },
            },
          },
          staticGatesPassed: { type: 'boolean' },
          staticNote: { type: 'string' },
        },
      },
    },
  },
};

const MODEL_SCHEMA = {
  type: 'object',
  required: ['canonicalName', 'modelPath', 'combos'],
  properties: {
    canonicalName: { type: 'string' },
    modelPath: { type: 'string' },
    modelExisted: { type: 'boolean' },
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

const AUDIT_SCHEMA = {
  type: 'object',
  required: ['archetype', 'comboVerdicts'],
  properties: {
    archetype: { type: 'string' },
    comboVerdicts: {
      type: 'array',
      items: {
        type: 'object',
        required: ['combo', 'status'],
        properties: {
          combo: { type: 'string' },
          status: { type: 'string', enum: ['faithful', 'divergent', 'untested', 'blocked'] },
          coveringTests: { type: 'array', items: { type: 'string' } },
          evidence: { type: 'string' },
          sources: { type: 'string' },
          missingCards: { type: 'array', items: { type: 'string' } },
        },
      },
    },
    testIssues: {
      type: 'array',
      items: {
        type: 'object',
        required: ['test', 'problem', 'severity'],
        properties: {
          test: { type: 'string' },
          problem: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor'] },
        },
      },
    },
    summary: { type: 'string' },
  },
};

const AUTHOR_SCHEMA = {
  type: 'object',
  required: ['testFile', 'testsAuthored'],
  properties: {
    testFile: { type: 'string' },
    createdNewFile: { type: 'boolean' },
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
  required: ['verdict', 'testsOk'],
  properties: {
    verdict: { type: 'string', enum: ['approve', 'revise'] },
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
  required: ['finding', 'confirmed', 'summary'],
  properties: {
    finding: { type: 'string' },
    confirmed: { type: 'boolean' },
    cause: {
      type: 'string',
      enum: ['engine_bug', 'card_effect_gap', 'wrong_model', 'wrong_test', 'flaky', 'unknown'],
    },
    tracker: { type: 'string' },
    entryWritten: { type: 'boolean' },
    summary: { type: 'string' },
  },
};

const REPORT_SCHEMA = {
  type: 'object',
  required: ['reportPath', 'verdicts'],
  properties: {
    reportPath: { type: 'string' },
    verdicts: {
      type: 'array',
      items: {
        type: 'object',
        required: ['archetype', 'verdict'],
        properties: {
          archetype: { type: 'string' },
          verdict: {
            type: 'string',
            enum: ['FAITHFUL', 'DIVERGENCES_FOUND', 'INSUFFICIENT_COVERAGE', 'BLOCKED'],
          },
          combosFaithful: { type: 'number' },
          combosDivergent: { type: 'number' },
          combosUntested: { type: 'number' },
          findingsFiled: { type: 'number' },
        },
      },
    },
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
    `${COMMON}\nTASK: list the archetypes worth considering for a faithfulness audit.\n` +
      `Run: \`python code/tools/resolve_deck.py --list-archetypes --min-share ${MIN_SHARE}\`.\n` +
      `Return every listed archetype (name + meta share %), preserving the meta-share order (highest first).` +
      FINAL_CALL,
    { schema: DISCOVERY_SCHEMA, label: 'list-archetypes', phase: 'Discover' }
  );
  candidates = (disc && disc.archetypes) || [];
  log(`Discovery surfaced ${candidates.length} candidate archetype(s) at >= ${MIN_SHARE}% meta share.`);
}

if (!candidates.length) {
  log('No candidate archetypes — nothing to audit.');
  return { selected: [], reason: 'no candidates' };
}

// Bound the discovery fan-out (the deck library can list ~300 archetypes at
// minShare 0; one coverage agent per entry saturates concurrency for nothing).
const MAX_CHECK = 60;
if (!EXPLICIT && candidates.length > MAX_CHECK) {
  log(`Bounding coverage check to the top ${MAX_CHECK} of ${candidates.length} candidates (by meta share).`);
  candidates = candidates.slice(0, MAX_CHECK);
}

let implemented = [];
if (EXPLICIT) {
  // Caller asserted the list — trust it and skip the coverage gate entirely.
  implemented = candidates.map((c) => ({ name: c.name, slug: slugify(c.name), ratio: null }));
  log(`Explicit mode: skipping the coverage gate; auditing all ${implemented.length} listed archetype(s).`);
} else {
  const coverage = await parallel(
    candidates.map((c) => () =>
      agent(
        `${COMMON}\nTASK: decide whether the archetype "${c.name}" is IMPLEMENTED enough to audit.\n` +
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
  log('No archetype cleared the coverage gate — none are implemented enough to audit.');
  return { selected: [] };
}

const selected = implemented.slice(0, TOP_N);
log(`Auditing: ${selected.map((x) => x.name).join(', ')}`);
if (implemented.length > TOP_N) {
  log(`Capping to top ${TOP_N}. DEFERRED: ${implemented.slice(TOP_N).map((x) => x.name).join(', ')}`);
}

// ── Phase: Inventory (one agent — single cargo build, no races) ──────────────
phase('Inventory');
const inv = await agent(
  `${COMMON}\nTASK: inventory the existing interaction-test estate for these archetypes:\n` +
    selected.map((a) => `  - ${a.name}`).join('\n') +
    `\n\nFor EACH archetype:\n` +
    `1. Find its interaction-test file under \`code/digimon-engine/tests/archetypes/\` if one exists. Slugs do not always equal slugify(name) (e.g. "Machine Black" -> st5_machine_black.rs / st5.rs) — check \`tests/archetypes/main.rs\` mod list, file doc-comments, and \`qa/qa-reports/archetype_interactions.json\` (interaction_test_file fields) to map name -> file. hasTestFile=false when no suite exists.\n` +
    `2. Find its model doc \`qa/archetype-qa/<archetype>-model.md\` (case/slug variants count; a per-card faithfulness doc like \`<archetype>.md\` does NOT count as a model).\n` +
    `3. Run the archetypes suite ONCE for everything: \`cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes\`, then attribute each #[test] result in the mapped files to its archetype (suiteResults per archetype; empty array when no file).\n` +
    `4. Run the static harness read-only: \`cargo run -p archetype-static-tests -- "<name>" --threshold ${THRESHOLD} --no-write --json\` -> staticGatesPassed + a one-line staticNote.\n` +
    `Return { archetypes: [...] } with one entry per listed archetype (exact input names).` +
    FINAL_CALL,
  { schema: INVENTORY_SCHEMA, label: 'inventory', phase: 'Inventory' }
);
const invByName = {};
((inv && inv.archetypes) || []).forEach((e) => { invByName[e.name] = e; });
selected.forEach((a) => {
  a.inv = invByName[a.name] || { name: a.name, hasTestFile: false, hasModelDoc: false, suiteResults: [] };
  if (a.inv.slug) a.slug = a.inv.slug;
  log(`  ${a.name}: ${a.inv.hasTestFile ? `suite ${a.inv.testFile} (${(a.inv.suiteResults || []).length} tests)` : 'NO suite'}; model doc ${a.inv.hasModelDoc ? 'present' : 'MISSING'}.`);
});

// ── Phases: Model → Audit → Author → Review → Revise (pipelined per archetype)
const audited = await pipeline(
  selected,
  // Stage 1 — Model: create the system model if missing, else read + refresh it.
  (a) =>
    agent(
      `${COMMON}\nTASK: produce the system model + ranked combos for the archetype "${a.name}" (skill Phases 0-3).\n` +
        (a.inv.hasModelDoc
          ? `A model doc EXISTS at \`${a.inv.modelDoc || `qa/archetype-qa/${a.name}-model.md`}\`. Read it, spot-check its combo claims against the sources (printed text, $BASE_DCGO C#, general_rule.pdf §16), refresh anything stale or uncited, and extract the ranked combos from it.\n`
          : `NO model doc exists. Resolve the pool (\`resolve_deck.py "${a.name}" --json\`), research the meaningful cards (printed text + DCGO C# at $BASE_DCGO + relevant general_rule.pdf §16 keyword/timing rules; Pinecone for prior context), and WRITE the durable model doc to \`qa/archetype-qa/${a.name}-model.md\` in the fixed structure from the skill (card pool & roles, digivolution lines, named combos with cards + expected mechanical outcome + cited sources + rank, playstyle, win conditions, ranked interactions). This file is a MANDATORY deliverable — re-Read it after writing to confirm it exists and is non-empty.\n`) +
        `Return up to ${COMBO_CAP} top-ranked combos (play-frequency x payoff-centrality). For EACH: name, cards (card IDs), the precise expected mechanical outcome (a checkable board diff), sources (general_rule.pdf §/DCGO path). List combos ranked but dropped under the cap in \`dropped\`.` +
        FOCUS_DIRECTIVE +
        `\nDo NOT author or edit any Rust test. canonicalName must be the resolved canonical archetype name.` +
        FINAL_CALL,
      { schema: MODEL_SCHEMA, label: `model:${a.name}`, phase: 'Model' }
    ).then((m) => (m ? { ...a, model: m } : null)),
  // Stage 2 — Audit (Opus): cross-examine existing tests + combo claims vs sources.
  (prev, a) => {
    if (!prev || !prev.model) return prev;
    const combos = prev.model.combos || [];
    return agent(
      `${COMMON}\nTASK: AUDIT the archetype "${prev.model.canonicalName}" for faithfulness to the real Digimon TCG.\n` +
        `Model doc: \`${prev.model.modelPath}\`. Existing interaction-test file: ${prev.inv.hasTestFile ? `\`${prev.inv.testFile}\`` : 'NONE'}.\n` +
        `Current suite results: ${JSON.stringify(prev.inv.suiteResults || [])}\n` +
        `Ranked combos to verify:\n` +
        combos.map((c) => `  - ${c.name}: cards [${c.cards.join(', ')}] — expected: ${c.expectedOutcome} (sources: ${c.sources || 'see model'})`).join('\n') +
        `\n\nFor EACH combo decide a status, judging against the SOURCES (printed card text via /digimon-card-lookup or cards.json+overrides, general_rule.pdf §16, DCGO C# at $BASE_DCGO) — not against the model alone:\n` +
        `- "faithful": one or more existing tests exercise this combo AND their assertions match the outcome the sources claim AND the tests pass. List them in coveringTests.\n` +
        `- "divergent": an existing test fails, OR a green test asserts a wrong/weakened outcome, OR the engine behavior the test pins contradicts the sources. Give concrete evidence + the source citation.\n` +
        `- "untested": no existing test exercises this combo (gap-fill candidate).\n` +
        `- "blocked": a combo piece is unimplemented (check qa/qa-reports/validated_cards_dsl.json or the static harness combo-presence gate); name missingCards.\n` +
        `Also audit the existing test FILE itself against the contract below and report violations in testIssues (synthetic stand-ins for fillable roles, bypassing engine helpers, assertions weakened to pass, untraceable doc-comments).\n` +
        FAITHFULNESS_RULES +
        `\nBe adversarial: a passing suite is NOT evidence of faithfulness — the assertion itself is on trial. Cite a source for every divergent verdict. Do NOT edit any file.` +
        FINAL_CALL,
      { schema: AUDIT_SCHEMA, model: 'opus', label: `audit:${a.name}`, phase: 'Audit' }
    ).then((aud) => {
      if (!aud) return prev;
      const counts = { faithful: 0, divergent: 0, untested: 0, blocked: 0 };
      (aud.comboVerdicts || []).forEach((v) => { counts[v.status] = (counts[v.status] || 0) + 1; });
      log(`AUDIT ${prev.model.canonicalName}: ${counts.faithful} faithful, ${counts.divergent} divergent, ${counts.untested} untested, ${counts.blocked} blocked; ${(aud.testIssues || []).length} test issue(s).`);
      return { ...prev, audit: aud };
    });
  },
  // Stage 3 — Author (gap-fill): tests for untested, unblocked combos.
  (prev, a) => {
    if (!prev || !prev.audit || !AUTHOR_MISSING) return prev;
    const verdicts = prev.audit.comboVerdicts || [];
    const gaps = (prev.model.combos || []).filter((c) =>
      verdicts.some((v) => v.combo === c.name && v.status === 'untested')
    );
    if (!gaps.length) return prev;
    const targetFile = prev.inv.hasTestFile
      ? prev.inv.testFile
      : `code/digimon-engine/tests/archetypes/${prev.slug}.rs`;
    return agent(
      `${COMMON}\nTASK: gap-fill interaction tests for "${prev.model.canonicalName}" (skill Phases 4-5). Untested combos:\n` +
        gaps.map((c) => `  - ${c.name}: cards [${c.cards.join(', ')}] — expect: ${c.expectedOutcome}`).join('\n') +
        `\n\nSTEP A (precondition gate): run \`cargo run -p archetype-static-tests -- "${prev.model.canonicalName}" ` +
        gaps.map((c) => `--combo "${c.name}=${c.cards.join(',')}"`).join(' ') +
        ` --threshold ${THRESHOLD} --no-write --json\`. Any combo naming an UNIMPLEMENTED card is BLOCKED — do NOT author its test; record it in blockedCombos and route the missing card per the skill.\n` +
        `STEP B (author): for each surviving combo write ONE #[test] in \`${targetFile}\`` +
        (prev.inv.hasTestFile
          ? ' (APPEND to the existing file, matching its style; do not rename or rewrite existing tests)'
          : ' (NEW file, modeled on tests/archetypes/rocks.rs)') +
        `, using support.rs fixtures (dsl_builder, snapshot/BoardSnapshot, run_actions). Assert the claimed mechanical outcome via a before/after BoardSnapshot diff; include the unhappy/enabler-absent path where it expresses a system-level fact; carry a doc-comment traceable to the model combo (name, cards, expected outcome, sources).\n` +
        FAITHFULNESS_RULES +
        `\nDO NOT edit \`tests/archetypes/main.rs\` — module registration is the orchestrator's job. You therefore cannot run a NEW file yourself; just write a clean, compilable file. (Appended tests in an already-registered file MAY be run.)\n` +
        `Return testFile, createdNewFile, testsAuthored [{combo, fnName}], blockedCombos, notes.` +
        FINAL_CALL,
      { schema: AUTHOR_SCHEMA, label: `author:${a.name}`, phase: 'Author' }
    ).then((au) => (au ? { ...prev, author: au } : prev));
  },
  // Stage 4 — Review (Opus): audit newly authored tests before they're trusted.
  (prev, a) => {
    if (!prev || !prev.author || !(prev.author.testsAuthored || []).length) return prev;
    return agent(
      `${COMMON}\nTASK: review the NEWLY AUTHORED interaction tests for "${prev.model.canonicalName}" before they are trusted.\n` +
        `File: \`${prev.author.testFile}\`. New tests: ${prev.author.testsAuthored.map((t) => t.fnName).join(', ')}.\n` +
        `Audit them against the card text + general_rule.pdf + DCGO C# ($BASE_DCGO) and the contract below. Catch: a wrong combo claim, a test asserting the wrong outcome, an over-/under-specified DP window, a missed "may"/"by-cost"/"or" nuance, a synthetic card standing in for a real combo piece.\n` +
        FAITHFULNESS_RULES +
        `\nVerdict "approve" only if every new test faithfully matches the sources. Return verdict, testsOk, issues [{combo, problem, severity}].` +
        FINAL_CALL,
      { schema: REVIEW_SCHEMA, model: 'opus', label: `review:${a.name}`, phase: 'Review' }
    ).then((rev) => (rev ? { ...prev, review: rev } : prev));
  },
  // Stage 5 — Revise: fix blocker/major review findings (registration findings excluded).
  (prev, a) => {
    if (!prev || !prev.author || !prev.review || prev.review.verdict !== 'revise') return prev;
    const issues = (prev.review.issues || []).filter((i) => {
      if (i.severity !== 'blocker' && i.severity !== 'major') return false;
      const p = (i.problem || '').toLowerCase();
      return !(p.includes('main.rs') || p.includes('mod ') || p.includes('never run') || p.includes('never executes'));
    });
    if (!issues.length) return prev;
    return agent(
      `${COMMON}\nTASK: REVISE the newly authored interaction tests for "${prev.model.canonicalName}" to resolve the reviewer's blocker/major findings. Do NOT re-implement any card or weaken assertions.\n` +
        `File: \`${prev.author.testFile}\`. Model doc: \`${prev.model.modelPath}\`.\n` +
        `Reviewer findings to fix:\n` +
        issues.map((i) => `  - [${i.severity}] ${i.combo ? i.combo + ': ' : ''}${i.problem}`).join('\n') +
        `\n\n` + FAITHFULNESS_RULES +
        `\nIf a faithful outcome genuinely can't be produced, #[ignore] the test with the reason and file the gap. Do NOT edit \`tests/archetypes/main.rs\`. Do NOT touch tests that pre-dated this run.\n` +
        `Return the updated testFile, createdNewFile, testsAuthored, blockedCombos, notes.` +
        FINAL_CALL,
      { schema: AUTHOR_SCHEMA, label: `revise:${a.name}`, phase: 'Revise' }
    ).then((fix) => (fix ? { ...prev, author: { ...fix, createdNewFile: prev.author.createdNewFile }, revised: true } : prev));
  }
);

const results = audited.filter(Boolean);
if (!results.length) {
  log('All archetypes dropped during model/audit — nothing to report.');
  return { selected: selected.map((a) => a.name) };
}

// ── Phase: Register & Run (orchestrator-owned, single agent → no main.rs race)
phase('Register & Run');
const newFiles = results.filter((r) => r.author && r.author.createdNewFile);
const touched = results.filter((r) => r.author && (r.author.testsAuthored || []).length);
const reg = await agent(
  `${COMMON}\nTASK: register any new interaction-test modules and run the suite.\n` +
    (newFiles.length
      ? `New files to register in \`code/digimon-engine/tests/archetypes/main.rs\` (add \`mod <slug>;\` under the "Per-archetype interaction suites" section if not already present):\n` +
        newFiles.map((r) => `  - mod ${r.slug};  (file ${r.author.testFile})`).join('\n')
      : `No new files were created this run (gap-fill appended to existing registered files, or authored nothing) — skip registration.`) +
    `\n\nThen run \`cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes\`.\n` +
    `If the binary FAILS TO COMPILE, bisect: comment out the most recently added \`mod\` lines one at a time to isolate which file(s) don't compile, leave the good ones registered, and report broken slugs in excludedModules with the compiler error in the matching suiteResults message.\n` +
    `Report suiteResults for every #[test] in the audited archetypes' files:\n` +
    results.map((r) => `  - ${r.slug}: ${r.inv.hasTestFile ? r.inv.testFile : (r.author ? r.author.testFile : '(no file)')}`).join('\n') +
    `\nAlso refresh the verdict tracker per audited archetype (writes qa/qa-reports/archetype_interactions.json): ` +
    results
      .map(
        (r) =>
          `\`cargo run -p archetype-static-tests -- "${r.model.canonicalName}" ` +
          (r.model.combos || []).map((c) => `--combo "${c.name}=${c.cards.join(',')}"`).join(' ') +
          ` --threshold ${THRESHOLD}\``
      )
      .join('; ') +
    `\nReturn registered (slugs), compileOk, excludedModules, suiteResults [{slug, test, passed, message}], staticHarness [{archetype, allGatesPassed}].` +
    FINAL_CALL,
  { schema: REGISTER_SCHEMA, label: 'register-and-run', phase: 'Register & Run' }
);

// ── Phase: Triage (confirm vs sources, route to trackers) ────────────────────
// Findings = failing tests from the run + divergent combo verdicts from the audit.
const failures = ((reg && reg.suiteResults) || []).filter((r) => !r.passed);
const divergences = [];
results.forEach((r) => {
  (r.audit && r.audit.comboVerdicts ? r.audit.comboVerdicts : [])
    .filter((v) => v.status === 'divergent')
    .forEach((v) => divergences.push({ archetype: r.model.canonicalName, slug: r.slug, verdict: v }));
});

let triaged = [];
if (failures.length || divergences.length) {
  phase('Triage');
  log(`Triaging ${failures.length} failing test(s) + ${divergences.length} divergent combo verdict(s).`);
  // Sequential, not parallel: triage agents APPEND to the shared gap trackers
  // (docs/RUST_ENGINE_GAPS.md, qa/archetype-qa/engine-gaps.md) — concurrent
  // appends to the same file race and clobber each other.
  const items = [
    ...failures.map((f) => ({
      kind: 'failing_test',
      title: `failing test ${f.test} (slug ${f.slug})`,
      detail: `Failure message: ${f.message || '(see suite output)'}`,
    })),
    ...divergences.map((d) => ({
      kind: 'divergent_combo',
      title: `divergent combo "${d.verdict.combo}" (${d.archetype})`,
      detail: `Audit evidence: ${d.verdict.evidence || '(see audit)'}; sources cited: ${d.verdict.sources || '(see model doc)'}; covering tests: ${(d.verdict.coveringTests || []).join(', ') || 'none'}`,
    })),
  ];
  for (const it of items) {
    const t = await agent(
      `${COMMON}\nTASK: triage a CANDIDATE faithfulness finding from the archetype audit — ${it.title}.\n${it.detail}\n` +
        `Treat it as a candidate engine bug, NOT a test to weaken. CONFIRM the discrepancy against the card's printed text, general_rule.pdf, and DCGO C# ($BASE_DCGO) exactly like a replay divergence, then decide the cause: engine_bug | card_effect_gap | wrong_model (model doc wrong — fix the model doc and say so) | wrong_test (assertion wrong — note the fix needed) | flaky | unknown.\n` +
        `If CONFIRMED as real: an engine-PRIMITIVE gap -> append to \`docs/RUST_ENGINE_GAPS.md\`; a card-effect faithfulness gap -> append to \`qa/archetype-qa/engine-gaps.md\`. Cite the combo, the test, and the source consulted. DO NOT edit engine code or any Rust test.\n` +
        `Return finding (echo the title), confirmed, cause, tracker (path written, if any), entryWritten, summary.` +
        FINAL_CALL,
      { schema: TRIAGE_SCHEMA, model: 'opus', label: `triage:${it.kind}`, phase: 'Triage' }
    );
    if (t) triaged.push(t);
  }
}

// ── Phase: Report (per-archetype faithfulness verdicts + dated audit doc) ────
phase('Report');
const reportInput = results.map((r) => ({
  archetype: r.model.canonicalName,
  modelDoc: r.model.modelPath,
  testFile: r.author ? r.author.testFile : r.inv.testFile || null,
  comboVerdicts: (r.audit && r.audit.comboVerdicts) || [],
  testIssues: (r.audit && r.audit.testIssues) || [],
  newTests: r.author ? r.author.testsAuthored : [],
  blocked: r.author ? r.author.blockedCombos || [] : [],
}));
const report = await agent(
  `${COMMON}\nTASK: write the archetype-faithfulness audit report and assign per-archetype verdicts.\n` +
    `Audit data (per archetype):\n${JSON.stringify(reportInput, null, 2)}\n` +
    `Suite run: ${JSON.stringify((reg && reg.suiteResults) || [])}\n` +
    `Triage outcomes: ${JSON.stringify(triaged)}\n\n` +
    `1. Compute today's date (e.g. PowerShell \`Get-Date -Format yyyy-MM-dd\`) and WRITE the report to \`qa/qa-reports/<date>-archetype-faithfulness-audit.md\`: per archetype — verdict, combo-by-combo table (status + evidence + covering tests), test issues, new tests authored, findings filed (with tracker paths), and what was deferred/blocked. Lead with a summary table.\n` +
    `2. MERGE the audit outcome into \`qa/qa-reports/archetype_interactions.json\` per the skill: under each archetype's entry update combos_tested (combo, status PASS/FAIL/UNTESTED/BLOCKED, tests, note) and findings — preserve the static_tests data the harness already wrote; do not drop other archetypes' entries.\n` +
    `3. Verdict rules: FAITHFUL = every audited combo faithful (and no unresolved blocker/major test issue); DIVERGENCES_FOUND = any confirmed divergence/finding; INSUFFICIENT_COVERAGE = no divergence but untested combos remain (gap-fill blocked or disabled); BLOCKED = audit couldn't proceed (missing cards dominate).\n` +
    `Return reportPath + verdicts [{archetype, verdict, combosFaithful, combosDivergent, combosUntested, findingsFiled}].` +
    FINAL_CALL,
  { schema: REPORT_SCHEMA, label: 'report', phase: 'Report' }
);

if (report) {
  log(`Report: ${report.reportPath}`);
  (report.verdicts || []).forEach((v) => log(`  ${v.archetype}: ${v.verdict}`));
}

// ── Final summary ────────────────────────────────────────────────────────────
return {
  selectedArchetypes: selected.map((a) => a.name),
  verdicts: (report && report.verdicts) || [],
  reportPath: report ? report.reportPath : null,
  perArchetype: reportInput,
  registration: reg || null,
  failures: failures.map((f) => f.test),
  triage: triaged,
};
