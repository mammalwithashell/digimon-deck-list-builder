export const meta = {
  name: 'author-set-v2',
  description: 'Mass-implement + combo-test an approved release-set slice partition (Phases 4-6, schema-free)',
  phases: [
    { title: 'Implement', detail: 'one batch-implement-cards-rust-dsl run per slice, Lv2->Lv7 order' },
    { title: 'ComboTest', detail: 'one archetype-interaction-test-author run per non-orphan slice' },
    { title: 'SetGate', detail: 'coverage + cargo test rollup' },
  ],
}

// SCHEMA-FREE author-set workflow. See author-set.mjs header for rationale: forcing
// a trailing StructuredOutput call on an agent that runs the large DSL skill is
// fragile (it finishes the work but forgets the tool call), which aborted 7/16
// slices on two BT25 runs. Every stage here is schema-free; the agent's final text
// IS the return value. The orchestrator reconciles against ground truth (YAML on
// disk + cargo test + the verdict tracker) afterward.
// args = { set_prefix, slices: [{ name, card_ids: [...Lv-ordered], is_orphan }] }

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

log(`author-set ${SET}: ${slices.length} slices (Phases 4-6, schema-free)`)

const results = await pipeline(
  slices,
  (slice) => agent(
    `Follow the /batch-implement-cards-rust-dsl skill to implement these ${SET} cards, ` +
    `already ordered low->high digivolution stage: ${slice.card_ids.join(', ')}. ` +
    `Slice: "${slice.name}". TDD-first (DebugRunner tests before YAML). No approximations — ` +
    `every choice via pending_selection; if a card needs an engine/DSL primitive that does not ` +
    `exist, mark it BLOCKED and log to the gap tracker rather than stubbing. Cards already ` +
    `IMPLEMENTED/AUDITED-OK in validated_cards_dsl.json are SKIPped (idempotent rerun). ` +
    `Do NOT call any StructuredOutput tool. When done, END your final message with ONE line:\n` +
    `RESULT slice="${slice.name}" implemented=[ID,...] blocked=[ID,...] skipped=[ID,...]`,
    { label: `impl:${slice.name}`, phase: 'Implement' }
  ),
  (impl, slice) => slice.is_orphan
    ? `RESULT slice="${slice.name}" tests_added=0 note="orphan staple — combo-test case-by-case, skipped by default"`
    : agent(
        `Follow the /archetype-interaction-test-author skill for the "${slice.name}" slice of ${SET} ` +
        `(cards: ${slice.card_ids.join(', ')}). Author multi-card interaction tests for the combos whose ` +
        `pieces are all IMPLEMENTED (skip any combo naming a BLOCKED/unimplemented card and log it). Use ` +
        `synthesized DebugRunner fixtures for cross-set evolution prerequisites; pull a cross-set card's ` +
        `implementation ONLY if a combo test fires its actual printed effect and it is not already ` +
        `implemented (lazy, no eager closure). Do NOT call any StructuredOutput tool. When done, END your ` +
        `final message with ONE line:\nRESULT slice="${slice.name}" tests_added=<n> blocked_combos=[...]`,
        { label: `combo:${slice.name}`, phase: 'ComboTest' }
      )
)

phase('SetGate')
const gate = await agent(
  `Set coverage gate for ${SET}. Run \`cargo test --manifest-path code/digimon-engine/Cargo.toml ` +
  `--test cards_behavioral\` and confirm it is green. Enumerate every ${SET} card still ` +
  `BLOCKED/PARTIAL/untested from qa/qa-reports/validated_cards_dsl.json. Do NOT call any ` +
  `StructuredOutput tool. END your final message with ONE line:\n` +
  `GATE green=<true|false> implemented=<n>/<total> outstanding=[ID,...]`,
  { label: `gate:${SET}`, phase: 'SetGate' }
)

return { set: SET, slices: results, gate }
