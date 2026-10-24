# Construct semantics sheet — research brief (READ-ONLY)

Repository worktree: `D:/wt/p3-constructs` (branch `claude/p3-construct-tests`, HEAD f24f26dfa).
Do NOT edit any file under the repo except your single output file named below. Do NOT run cargo
(no builds, no tests). Do NOT dispatch subagents. Pure reading + writing one markdown sheet.

## Why

RL training admits a card only when every DSL construct its YAML references has a behavioural
mechanics test (`qa/impact/lowering_coverage.json`, built by `code/tools/lowering_coverage.py`).
51 constructs used by the DigiLab top-10 tournament decks have none. The owner's rule: test the
MECHANIC (the DSL lowering) with synthetic fixture cards, not the production cards that use it.
A plan will be written from your sheet; the plan's implementers will write the tests. Your sheet
must give them everything they need to write a correct test without re-deriving the semantics.

## Where things are

- Lowering: `code/digimon-dsl/src/**` (parse/compile → `CompiledStep`/`CompiledPredicate`/timings)
  and `code/digimon-engine/src/dsl_cards/**` (`predicate.rs`, `step/*.rs`, `timing_map.rs`,
  `lower_triggered.rs`, `lower_replacement.rs`, `formula_eval.rs`).
- YAML syntax: the DSL vocabulary doc `docs/DSL_VOCABULARY.md` (or the nearest file matching
  `docs/*DSL*VOCAB*` / `docs/RUST_DSL_TEST_API.md`) and the parse structs in `code/digimon-dsl/src`.
- Which production cards use a construct and which existing tests touch it without crediting it:
  `qa/impact/lowering_coverage.json` → `constructs[<kind:name>]` (look at every field present,
  e.g. parsed-only tests, near-miss files) and `cards[<ID>].constructs`. Card YAML lives in
  `code/digimon-engine/cards/<set lowercase>/<ID>.yaml`; printed text in `data/cards.json`
  (`effect_text`, `inherited_text`, `security_text`) and `data/card_bundles/<ID>.md`.
- Rules: `docs/digimon-rules/cr-4.3.txt` (grep by rule number or keyword; cite 4.3 numbers).
- How a mechanics test drives the engine from inline YAML: read
  `code/digimon-engine/tests/dsl/effect_battle.rs` (DebugRunner::builder().from_dsl_yaml(..)
  .add_card(..).security(..).start(); place_on_field; enqueue_triggered; drain_effect_queue;
  pending_selection; execute_action) and `code/digimon-engine/tests/dsl/folded_may_opt_refund.rs`.
  Skim `docs/RUST_DSL_TEST_API.md` for the DebugRunner surface (builder, zones, actions, memory,
  turn/phase helpers). Tests count only in `code/digimon-engine/tests/dsl`, `tests/rules_conformance`,
  `tests/keyword_phase_*`, `tests/timing_dispatch*`, `tests/cost_hooks`, `tests/replacements*`,
  `tests/effect_context*` (never `cards_behavioral`), and only when the test body itself runs the engine.

## For EACH construct assigned to you, write this section

### `<kind>:<name>`
- **Lowering site(s):** `path:line` for the parse struct/enum variant, and `path:line` for the
  runtime evaluation (predicate eval / step executor / timing dispatch). Quote the key lines of the
  runtime body (<= 15 lines) so the test author sees the exact semantics.
- **YAML shape:** the exact key(s) and argument shape as a card would write them (copy from a real
  card YAML).
- **Semantics (2-6 sentences):** what it evaluates / does, including edge cases visible in the code
  (empty zone, no binding, face-down cards, tokens, opponent vs you, suspended state, what happens on
  failure, whether a selection is surfaced as `pending_selection` and whether it is optional).
- **Rule basis:** CR 4.3 § numbers that govern it, one line each (or "card-text-only idiom" if none).
- **Production users:** 2 card IDs (prefer ones in the top-10 decks: see
  `.superpowers/sdd/2026-10-22-construct-tests/top10_untested_union.json` for which decks block on it,
  and `qa/impact/lowering_coverage.json` cards[*].constructs for the users) with the printed clause
  text that the construct implements, and the YAML snippet from that card that uses it.
- **Existing near-miss tests:** file paths from the coverage index that name the construct but earn
  no behavioural credit, and one line on what each actually checks.
- **Proposed behavioural test (for the plan):**
  - fixture: a minimal synthetic card YAML (inline string) that exercises ONLY this construct plus
    the minimum scaffolding (give the YAML);
  - setup: DebugRunner builder calls (zones, memory, turn) in prose or code;
  - positive case: the exact observable the test asserts (zone sizes, memory, a pending_selection
    with N valid actions, a modifier present, a trigger that fired) and how;
  - negative case: the condition under which the construct must NOT fire / must evaluate false /
    must surface nothing, and the assertion;
  - selection surface: if the construct surfaces a choice, the assertion that it is a real
    `pending_selection` (no auto-pick), with `is_optional` matching the card text;
  - for timings: the adjacent timing at which it must NOT fire.
- **Suspected engine/DSL defects (read-only):** anything in the code that looks wrong against the
  rule basis or the card text. Say "none seen" if none. Do not fix anything.

## Output

Write ONE file: `D:/wt/p3-constructs/.superpowers/sdd/2026-10-22-construct-tests/semantics/<your-family>.md`
with a short header (family name, construct list, date 2026-10-22) and one section per construct in
the order given. Return only: the output path, the construct count written, and up to 5 bullet
points of suspected defects or constructs whose semantics you could not pin down (name them).
