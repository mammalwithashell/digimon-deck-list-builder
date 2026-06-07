## Context

The Rust engine already ships static JSON metadata for `ST3-01` through `ST3-16` under `code/digimon-engine/cards/st3/`, and the global `data/cards.json` contains the printed text. The production Rust card-effect registry, however, is built from DSL YAML in the embedded card pack; JSON-only cards are available as metadata but are not considered implemented by `load_implemented_card_ids()`.

ST-3 is a compact early yellow starter deck. Its behavior is mostly foundational: vanilla Digimon, inherited memory and DP effects, Blocker, Recovery, temporary DP modifiers, Security Attack modifiers, security-option resolution, and Tamer play from security. This makes it a good candidate for a contained DSL pass, with care around exact trigger causes for "deleted by dropping to 0 DP" and security-Digimon DP modifiers.

## Goals / Non-Goals

**Goals:**

- Author faithful production DSL YAML for all 16 ST3 cards.
- Add behavioral tests for every effectful printed clause and load/structural checks for vanilla cards.
- Add or update a canonical ST-3 deck fixture/library entry matching the worldwide 54-card product list.
- Verify the full ST3 card pool registers through the Rust DSL embedded pack and can be used by Rust-backed gameplay/training paths.
- Keep all legal player choices exposed through normal action/pending-selection flow.

**Non-Goals:**

- No action-space, observation tensor, PyO3 API, model metadata, or frontend contract changes.
- No legacy Python card-script authoring.
- No Korean-only promo-card implementation or region-specific product variants.
- No approximation of blocked clauses; blocked behavior must be tracked as reusable engine/DSL gaps.

## Decisions

1. Production YAML is the implementation source for ST3.

   Rationale: the current registry loads production cards from `code/digimon-engine/cards/<set>/<CARD-ID>.yaml` via the embedded DSL pack. Adding hand-written Rust `CardEffect` structs or Python scripts would bypass the migration direction and make `load_implemented_card_ids()` less representative.

   Alternative considered: implement only the effectful cards and let vanilla JSON-only cards stand. That would leave vanilla ST3 cards unregistered as implemented card IDs and make full-deck validation fail for an otherwise usable starter.

2. Test coverage is per-card, with structural tests for vanilla cards.

   Rationale: effectful cards need behavior assertions tied to printed text; vanilla cards still need embedded-pack/load assertions so the whole starter deck is registered. This matches the existing `tests/cards_behavioral/<set>/` pattern while keeping the testing burden proportional.

   Alternative considered: one starter-deck smoke test only. That would prove the deck loads but would miss clause-level regressions in options, inherited effects, and security effects.

3. Starter deck loadability is a first-class deliverable.

   Rationale: the user asked for the starter deck, not just isolated card YAML. The implementation should include a canonical 54-card ST-3 list in the repository's established deck-fixture or deck-library path, then verify it can be parsed and filtered against implemented cards.

   Alternative considered: document the list only in tests. That would help test the change but not make the starter naturally selectable by local tooling.

4. Gap handling stays capability-centric.

   Rationale: ST3 may surface reusable needs such as DP-deletion trigger predicates or security-Digimon DP modifiers. If a primitive is missing, the implementation must mark that reusable primitive in `docs/RUST_ENGINE_GAPS.md` or QA gap trackers and keep affected tests ignored with accurate reasons until the primitive exists.

   Alternative considered: implement a coarser proxy, such as any deletion instead of deletion by DP reaching zero. That violates the repository no-approximations rule and would teach agents incorrect legality/behavior.

## Risks / Trade-offs

- DP-deletion trigger cause may not be directly expressible in DSL -> Mitigation: inspect current deletion event causes and predicates before authoring `ST3-01` and `ST3-04`; file a reusable gap if exact "deleted by dropping to 0 DP" cannot be expressed.
- Security-Digimon DP modification may be easy to confuse with battle-area DP modification -> Mitigation: use existing security-DP precedents and add tests that distinguish security battle DP from field DP.
- Option security disposition can regress card-zone flow -> Mitigation: explicitly test "activate main effect" security options and "add this card to hand" options through security resolution.
- Full-deck smoke tests can become brittle if using shuffled gameplay state -> Mitigation: keep smoke tests focused on parsing, registration, and short deterministic load/reset behavior rather than game outcome.

## Migration Plan

1. Add ST3 YAML and tests in one change, preserving existing JSON metadata.
2. Add the canonical deck fixture/library entry after card registration is in place, so validation can reject missing implemented IDs during development.
3. Run focused Rust card tests, DSL pack/build checks, and a Rust-backed deck smoke check.
4. Rollback is deleting the added ST3 YAML/tests/deck fixture; no persisted data migration is required.

## Open Questions

- Which existing deck-list surface should own the canonical starter fixture: `data/deck_library.json`, a dedicated starter fixture file, or an existing Rust/Python test fixture path? Implementation should inspect current loaders before editing.
- Can the current DSL precisely filter deletion triggers to "opponent Digimon deleted by DP reaching zero", or does that need a reusable predicate/gap first?
