## Context

ST-6: Starter Deck Venomous Violet has complete local card metadata in `data/cards.json` and mirrored JSON under `code/digimon-engine/cards/st6/`, but those JSON files are not executable card behavior. The Rust engine registers production card effects from YAML specs embedded by `code/digimon-engine/build.rs`, so ST6 currently needs authored DSL YAML and behavioral coverage before it can be treated as an implemented Rust starter deck.

The deck is a narrow purple starter product: sixteen unique cards, four Digi-Eggs, fifty main-deck cards, and effects centered on trashing, trash recursion, Blocker, Retaliation, Digi-Burst, Security effects, and purple Option play. Prior exploration found these behaviors map to existing DSL and engine primitives; no action-space, tensor, PyO3, or frontend contract change is expected.

## Goals / Non-Goals

**Goals:**

- Implement every ST6 card's printed behavior faithfully in production Rust YAML.
- Add enabled Rust behavioral tests for each effect-bearing ST6 card.
- Add or update a Venomous Violet starter-deck fixture/library entry using the official product counts.
- Verify the starter deck enters the Rust implemented-card registry and can be used in a headless smoke game.
- Keep every player-visible choice surfaced through existing action masks or `PendingSelection`.

**Non-Goals:**

- Do not expand `ACTION_SPACE_SIZE`, tensor layouts, observation profiles, PyO3 APIs, or frontend constants.
- Do not add Python legacy card scripts for ST6.
- Do not use no-op placeholders, hidden auto-selection, or broad `raw_rust` escapes to claim readiness.
- Do not implement unrelated purple archetype cards outside the ST6 product list.

## Decisions

1. Author ST6 as production YAML, not Rust hand-written card structs.

   The current engine source of truth is the DSL pack generated from `code/digimon-engine/cards/**/*.yaml`. Production YAML keeps ST6 aligned with current card-authoring practice and avoids growing the Rust registry with bespoke card structs.

   Alternative considered: leave JSON metadata only and rely on vanilla play/digivolve behavior. That would make the cards visible but not faithful, and would violate the no-approximations policy for every effect-bearing card.

2. Test behavior at the per-card level under `cards_behavioral/st6/`.

   ST6 has small, mostly independent card clauses. Per-card tests let each printed effect be verified directly with `DebugRunner`, while a final deck smoke test proves the starter product is usable end to end.

   Alternative considered: only add a starter-deck smoke test. That would miss most card-specific regressions because a smoke game may never trigger optional recursion, Security effects, or Digi-Burst paths.

3. Treat the starter-deck fixture as product data, not tournament meta data.

   Venomous Violet should be available as a deterministic starter deck even though it is not a scraped competitive archetype. The fixture should be labeled as a starter/manual/file source with no statistical meta-share claims.

   Alternative considered: add it as a normal meta archetype with synthetic stats. That could pollute gauntlet weighting and training assumptions, so product identity and statistical data should stay separate.

4. Start with existing primitives and file a gap only if tests prove a missing reusable capability.

   Known ST6 effects appear expressible with current DSL: inherited timing, `draw`, `trash_from_hand_by_index`, `add_to_hand_from_trash`, `select_*`, `delete_permanent`, `grant_keyword`, `digi_burst`, `play_from_trash_free`, and `suppress_on_play`.

   Alternative considered: pre-plan new DSL vocabulary. That is unnecessary until a failing TDD case identifies a real missing primitive.

## Risks / Trade-offs

- **Risk:** JSON metadata presence may be mistaken for executable implementation.
  **Mitigation:** Acceptance checks must use `load_implemented_card_ids()` or the Rust effect registry, not file presence alone.

- **Risk:** Security and Option "Then" tails can be modeled with incorrect optionality.
  **Mitigation:** Tests must cover decline paths and mandatory tails for `ST6-14`, `ST6-15`, and `ST6-16`.

- **Risk:** `ST6-16` plays from trash while suppressing On Play effects, which is easy to over-suppress.
  **Mitigation:** Tests must assert the played Digimon's On Play effect is suppressed while unrelated sibling effects remain eligible through normal engine behavior.

- **Risk:** Multi-target choices such as `ST6-12` "up to 2" could collapse into an auto-pick.
  **Mitigation:** Tests must inspect pending selections/action masks and assert legal target choices are surfaced to the player.

- **Risk:** Starter-deck data could accidentally affect meta sampling.
  **Mitigation:** Store the fixture with explicit starter/manual source metadata and no fabricated DigiLab stats.

## Migration Plan

1. Add ST6 YAML and tests in small TDD batches, prioritizing effect-bearing cards before vanilla metadata-only YAML.
2. Register the `st6` behavioral test module.
3. Add the Venomous Violet starter-deck fixture/library entry.
4. Run focused DSL/card tests, implemented-card registry checks, and a Rust headless starter-deck smoke test.
5. Update any ST6 readiness or gap notes only when source inspection and tests prove the end state.

Rollback is straightforward: revert the ST6 YAML, tests, and starter-deck fixture. No persisted data migration or runtime contract migration is required.

## Open Questions

- Which existing deck fixture location should be the canonical home for starter products: `data/deck_library.json`, a dedicated starter fixture file, or both?
- Should vanilla ST6 cards with no printed effects receive YAML specs solely to register them as implemented, or should the implemented-card registry gain a separate metadata-only registration path? For this change, production YAML with metadata and no effects is the simplest local path.
