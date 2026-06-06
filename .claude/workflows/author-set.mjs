export const meta = {
  name: 'author-set',
  description: 'Mass-implement + combo-test an approved release-set slice partition (Phases 4-6)',
  phases: [
    { title: 'Implement', detail: 'one batch-implement-cards-rust-dsl run per slice, Lv2->Lv7 order' },
    { title: 'ComboTest', detail: 'one archetype-interaction-test-author run per non-orphan slice' },
    { title: 'SetGate', detail: 'coverage + cargo test rollup' },
  ],
}

// Phases 1-3 (ingest-diff, keyword-gate barrier, cluster + approval) are owned by
// the /author-set SKILL, because they need a live pull, possible human halt on a
// flagged keyword, and interactive partition approval — none of which belong in a
// headless workflow. This workflow runs ONLY after the skill has:
//   - confirmed no flag_for_human keyword remains (or the user resolved it),
//   - auto-ingested or confirmed every needed keyword primitive exists, and
//   - gotten the user's approval on the slice partition.
// args = { set_prefix: "BT17", slices: [{ name, card_ids: [...Lv-ordered], is_orphan }] }

const IMPL_SCHEMA = {
  type: 'object',
  required: ['slice', 'implemented', 'blocked'],
  properties: {
    slice: { type: 'string' },
    implemented: { type: 'array', items: { type: 'string' } },
    blocked: { type: 'array', items: { type: 'string' } },
    notes: { type: 'string' },
  },
}
const COMBO_SCHEMA = {
  type: 'object',
  required: ['slice', 'tests_added'],
  properties: {
    slice: { type: 'string' },
    tests_added: { type: 'number' },
    cross_set_pulled: { type: 'array', items: { type: 'string' } },
    notes: { type: 'string' },
  },
}
const GATE_SCHEMA = {
  type: 'object',
  required: ['green', 'implemented', 'total', 'outstanding'],
  properties: {
    green: { type: 'boolean' },
    implemented: { type: 'number' },
    total: { type: 'number' },
    outstanding: { type: 'array', items: { type: 'string' } },
  },
}

// The runtime may deliver `args` as a JSON-encoded string OR a parsed object;
// normalize to an object.
let A = args
if (typeof A === 'string') {
  try { A = JSON.parse(A) } catch (e) { A = {} }
}

const SET = A && A.set_prefix
const slices = (A && A.slices) || []
if (!SET || !slices.length) {
  log('author-set: missing args.set_prefix or args.slices — run the /author-set skill (Phases 1-3) first.')
  return { error: 'missing approved partition' }
}

log(`author-set ${SET}: ${slices.length} slices (Phases 4-6 over an approved partition)`)

// Phases 4 + 5 pipelined per slice: a slice combo-tests as soon as it finishes
// implementing — slice A can be in ComboTest while slice B is still Implement.
const results = await pipeline(
  slices,
  // Stage 1 (Phase 4): implement the slice's cards via the DSL skill, Lv-ordered.
  (slice) => agent(
    `Follow the /batch-implement-cards-rust-dsl skill to implement these ${SET} cards, ` +
    `already ordered low->high digivolution stage: ${slice.card_ids.join(', ')}. ` +
    `Slice: "${slice.name}". TDD-first (DebugRunner tests before YAML). No approximations — ` +
    `every choice via pending_selection. Return the JSON summary.`,
    { label: `impl:${slice.name}`, phase: 'Implement', schema: IMPL_SCHEMA }
  ),
  // Stage 2 (Phase 5): combo-test non-orphan slices; orphans evaluated case-by-case.
  (impl, slice) => slice.is_orphan
    ? { slice: slice.name, tests_added: 0, notes: 'orphan staple — combo-test case-by-case, skipped by default' }
    : agent(
        `Follow the /archetype-interaction-test-author skill for the "${slice.name}" slice of ${SET} ` +
        `(cards: ${slice.card_ids.join(', ')}). Author multi-card interaction tests. Use synthesized ` +
        `DebugRunner fixtures for cross-set evolution prerequisites; pull a cross-set card's ` +
        `implementation ONLY if a combo test fires its actual printed effect and it is not already ` +
        `implemented (lazy, no eager closure). Return the JSON summary.`,
        { label: `combo:${slice.name}`, phase: 'ComboTest', schema: COMBO_SCHEMA }
      )
)

// Phase 6: set-level coverage gate.
phase('SetGate')
const gate = await agent(
  `Set coverage gate for ${SET}. Run \`cargo test --manifest-path code/digimon-engine/Cargo.toml ` +
  `--test cards_behavioral\` and confirm every ${SET} card ID resolves in load_implemented_card_ids(). ` +
  `Enumerate any card still BLOCKED/PARTIAL/untested. Return the JSON gate result.`,
  { label: `gate:${SET}`, phase: 'SetGate', schema: GATE_SCHEMA }
)

return { set: SET, slices: results, gate }
