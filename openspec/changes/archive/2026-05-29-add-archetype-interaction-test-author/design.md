## Context

The archetype-QA skill family is entirely per-card:

- `/assess-archetype-rust` → engine-gap audit (`docs/RUST_ENGINE_GAPS.md`), no tests.
- `/batch-implement-cards-rust-dsl` → YAML specs + **per-card** DebugRunner tests (`code/digimon-engine/tests/cards_behavioral/<set>/<card>.rs`), verdicts in `validated_cards_dsl.json`.
- `/review-archetype`, `/implement-archetype` → per-card faithfulness / Python scripts.

Confirmed gaps (from a landscape sweep): nothing models an archetype as a *system*; there are no multi-card interaction tests (every test home is per-set, per-card); there is no notion of a "static archetype test." Archetype resolution exists (`code/tools/resolve_deck.py::resolve_archetype` over `data/deck_library.json` + `data/archetype_aliases.json`, returning a manifest with per-card text + frequency). DebugRunner already supports multi-card setups (`builder().dsl_card(id).add_card(...).memory(n).start()`, `place_on_field`, `play`, `pending_selection`, `execute_action`, `auto_resolve`, `snapshot`) — what is missing is the *authoring intelligence* and a *home* for interaction tests, plus the static-invariant harness.

This is the proactive counterpart to the reactive replay modes in `add-interactive-replay-bug-hunter`: instead of investigating a game that happened, Claude forms hypotheses about how the archetype *should* behave and tests them deterministically. All three modes feed the same finding trackers.

Decisions confirmed with the user: this is a **new sibling change**; "static archetype test" means **all four** of deck-legality, coverage-gate, smoke-games, and combo-presence; the skill is a **capstone** that runs after cards are implemented and per-card tests are green.

## Goals / Non-Goals

**Goals:**

- A skill that builds a durable, reviewable **archetype-model** and uses it to author interaction tests that exercise the archetype's real combos.
- A home + fixture conventions for multi-card interaction tests.
- A static archetype-test harness covering the four invariant types, with per-archetype verdict tracking.
- Compose with the existing per-card skills (assume cards implemented; do not re-implement or duplicate per-card tests).
- Route interaction/static-test failures to the same finding trackers as the replay modes.

**Non-Goals:**

- No card implementation or per-card test authoring (owned by `/batch-implement-cards-rust-dsl`).
- No engine refactor; this rides existing DebugRunner + `resolve_deck.py`.
- No hard dependency on the replay change's engine/MCP work (the MCP is an optional combo-prototyping aid).
- No automated engine bug-fixing — this ships the test author + harness, not the repairs they surface.
- No opponent-modeling / meta analysis beyond what the archetype-model needs to choose interactions to test.

## Decisions

### D1. The archetype-model doc is a first-class artifact, not just scratch reasoning

Research produces `qa/archetype-qa/<archetype>-model.md` with a fixed structure: card pool (with roles: payoff / enabler / engine / tech), digivolution lines, named combos (the cards involved + the expected mechanical outcome + the rules/keyword basis), playstyle (aggro/control/combo, tempo, memory curve), win conditions, and a ranked list of interactions-to-test. Sources consulted (DCGO C# locations, `general_rule.pdf` rule numbers) are cited inline.

**Why:** it makes test authoring principled and auditable, is reusable across sessions, and is reviewable by a human or an Opus reviewer before a single test is written. **Alternative considered:** go straight from card text to tests. Rejected — produces arbitrary, low-signal tests and no durable understanding.

### D2. Interaction tests get their own home, separate from per-card tests

Multi-card / combo tests live in `code/digimon-engine/tests/archetypes/<archetype_slug>.rs` (a new tree), with shared multi-card fixture helpers (build a board of N permanents, seed hands/trash/security, drive a sequence, assert the combined outcome). Per-card tests stay in `cards_behavioral/<set>/`.

**Why:** interaction tests span sets and cards and don't belong under any one set; a dedicated tree keeps them discoverable and lets `cargo test --test archetypes` scope them. **Alternative considered:** put combos under one card's per-card file. Rejected — combos have no single owner and would be lost.

### D3. Each authored interaction test is a falsifiable hypothesis tied to a model combo

A test maps 1:1 to a named combo in the model. It asserts the *mechanical* outcome the combo claims (e.g. "digivolving Partner triggers DNAPartner's When-Digivolving and reduces target DP by X"), and where useful covers the unhappy path (combo broken if a piece is absent, or disrupted by the opponent). A failing interaction test is a candidate engine bug, triaged like a replay divergence.

**Why:** keeps tests meaningful and directly traceable to a claim a human can check against card text.

### D4. Static archetype tests are a harness of four invariant types

1. **Deck legality / construction** — the archetype's best/meta decklist(s) (from the manifest) build into a rules-legal deck and `Game::new` (or the debug builder) constructs from them without error.
2. **Coverage gate** — all (or a configurable threshold of) the archetype's unique cards are implemented in the DSL and have passing per-card behavioral tests (cross-referenced against `validated_cards_dsl.json`). A sub-threshold archetype is reported, not silently passed.
3. **Per-archetype smoke games** — N self-play games on the archetype's deck run to completion without panic or illegal state (reuses the existing headless smoke path).
4. **Combo-presence** — the specific cards named in the model's combos are all implemented, so the interaction tests are writable; a missing piece is reported as a blocker on the combo.

These are runnable independently of the authoring skill (CI-able, and a precondition the capstone checks before authoring interaction tests).

**Why all four:** they cover the deck-level invariants that no per-card test sees, and gates 2/4 are the precondition that makes the capstone's interaction authoring possible.

### D5. Capstone positioning + precondition gating

The skill runs after `/batch-implement-cards-rust-dsl` (cards implemented, per-card green). Before authoring interaction tests it runs the **coverage gate** and **combo-presence** static checks; if pieces are missing it reports them (routing to the implementation backlog / engine-gap trackers) rather than authoring tests that cannot pass. This makes the skill safe to run on a partially-implemented archetype: it degrades to "here's what's missing before interaction testing is possible."

**Alternative considered (rejected, per user):** test-first at archetype scale — author failing interaction tests as a spec that drives implementation. Overlaps the per-card TDD skills and produces large red suites; deferred.

### D6. Findings routing shared with the replay modes

A confirmed interaction/static-test failure routes to `docs/RUST_ENGINE_GAPS.md` (engine primitive) or `qa/archetype-qa/engine-gaps.md` (card effect), and the per-archetype run records verdicts in `qa/qa-reports/archetype_interactions.json` (archetype, combos tested, pass/fail, static-gate results, findings filed).

### D7. Sub-agent structure mirrors the existing family

A scout/Sonnet pass drafts the archetype-model and the interaction plan; an implementer pass authors the tests; an Opus reviewer audits the model + tests for faithfulness against card text + rules before they're committed — matching the three-wave pattern in `/batch-implement-cards-rust-dsl`. Pinecone (`digimon-engine` index) is used for card-script / engine-API / rules retrieval per the existing sub-agent convention.

### D8. Optional MCP-assisted combo prototyping

When the engine MCP is registered (from the replay change), the skill MAY prototype a combo interactively (`new_game_debug` → `legal_actions` → `step`) to confirm the action sequence before writing the Rust test, reducing authoring churn. This is an optimization, not a dependency: tests are authored directly in Rust regardless.

## Risks / Trade-offs

- **Model quality drives test quality.** A wrong archetype-model yields wrong tests. → Opus-review the model against card text + rules before authoring; cite sources inline so review is concrete.
- **False-positive "bugs" from misread card interactions.** → Treat a failing interaction test like a replay divergence: confirm against DCGO C# + `general_rule.pdf` before filing; the reviewer pass gates this.
- **Interaction-test flakiness / nondeterminism.** DebugRunner is deterministic, but RNG-consuming effects can vary. → Prefer deterministic setups; document any RNG-sensitive test; mirror the replay change's RNG caveat.
- **Combinatorial blow-up of interactions.** → Rank by the model (play frequency, payoff centrality); cap at top-N with the cap `log()`ged, never silently truncated.
- **Coverage-gate coupling to `validated_cards_dsl.json`.** → Treat that tracker as source of truth for per-card status; if absent for a card, report "unknown" rather than assume pass.
- **Smoke games can mask bugs (a green smoke ≠ correctness).** Aligns with existing memory note. → Smoke is a static *liveness* gate only; correctness comes from interaction tests + the judge mode.

## Migration Plan

1. Land the interaction-test home + multi-card fixture helpers; add a couple of hand-written exemplar interaction tests for a known archetype to pin the pattern.
2. Land the static-test harness (four invariant types) + the `archetype_interactions.json` tracker; make it runnable standalone (CI-able).
3. Author the skill (research → model → plan → author → execute → triage), composing with the existing family and gating on the static checks.
4. Dry-run end-to-end on one fully-implemented archetype; verify a real interaction bug (or a clean pass) is produced and routed.
5. Docs: `RUST_DSL_TEST_API.md` interaction pattern; `INDEX.md` pointer; note the three-mode bug-discovery picture.

Rollback: additive (new skill, new test tree, new harness, new trackers); reverting removes them with no impact on existing per-card tests or skills.

## Open Questions

- Coverage-gate threshold default (100% vs e.g. 90%) and whether it varies by archetype maturity.
- Smoke-game count `N` per archetype and whether mirror-match or vs-a-baseline-deck.
- Whether the interaction-test tree is `tests/archetypes/<slug>.rs` (per-archetype file) or `tests/interactions/<theme>.rs` (per-combo-theme); leaning per-archetype for traceability to the model.
- Whether to auto-invoke `/assess-archetype-rust` / `/batch-implement-cards-rust-dsl` when the precondition gates fail, or just report and stop.
