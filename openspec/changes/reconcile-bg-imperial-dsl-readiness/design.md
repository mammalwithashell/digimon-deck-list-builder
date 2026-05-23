## Context

The existing `bg-imperial-archetype-coverage` spec describes the work needed to close BG Imperial Rust DSL gaps. Follow-up investigation shows the implementation has moved past much of the older audit text: the deck-library pool contains 25 unique BG Imperial cards, all 25 YAML files exist, and the active YAML contains no live `raw_rust` clauses or steps. Focused tests also show key disputed cards such as `BT21-037` and `BT17-077` passing their behavioral coverage.

The remaining problem is not a new engine primitive. It is reconciliation: stale `PARTIAL`, `BLOCKED`, `approximation`, ignored-test, and raw-rust references now disagree with current source, ledgers, and resolved-gap notes. This change should make the repo's readiness signals tell one coherent story.

## Goals / Non-Goals

**Goals:**

- Define the current BG Imperial card pool from `data/deck_library.json` and reconcile it with `qa/qa-reports/validated_cards_dsl.json`.
- Remove or rewrite stale BG Imperial comments and tracker entries that cite resolved gaps as open blockers.
- Update card verdicts only when source inspection plus focused tests support the new state.
- Verify that BG Imperial remains free of live `raw_rust` YAML calls.
- Keep any real non-BG Imperial raw-rust work visible as separate follow-up.

**Non-Goals:**

- Do not change `ACTION_SPACE_SIZE`, tensor profiles, PyO3 exports, frontend constants, or model metadata.
- Do not introduce new card-effect behavior unless reconciliation discovers a genuine, currently failing BG Imperial clause.
- Do not rewrite adjacent cards outside the BG Imperial pool, including `BT13-040 Magnamon`.
- Do not add new `raw_rust` functions or raw-rust placeholders.

## Decisions

1. Treat `data/deck_library.json` as the BG Imperial pool source for this reconciliation.

   The validated ledger currently lists 24 BG Imperial cards, while the deck-library pool includes `BT17-077` as a 25th card. The reconciliation should explicitly account for all 25 cards so the final audit does not depend on an already-stale ledger slice.

   Alternative considered: use only `qa/qa-reports/validated_cards_dsl.json`. That would miss cards present in the archetype's actual deck-library pool and preserve the mismatch.

2. Keep this change documentation-ledger-test focused.

   The previous closeout work appears to have landed the required DSL substrate and card YAML. This change should update the observable readiness record and run focused verification rather than refactor working card behavior.

   Alternative considered: sweep broader raw-rust registry cleanup. That would expand into non-BG Imperial cards and require new DSL surface for hand-or-material union play, which is a separate capability.

3. Use DCGO only as a reference check, not an authority over printed card text.

   DCGO is useful for confirming intent around selection shape, optionality, and sequencing. Printed text in `data/cards.json` and local rules docs remain authoritative when resolving apparent discrepancies.

4. Preserve no-approximations review gates.

   If a stale comment turns out to mask a real missing behavior, the implementation work must stop and route that behavior through a reusable DSL or engine gap. It must not patch the docs to say "implemented" without action-mask-visible choices and regression tests.

## Risks / Trade-offs

- Ledger drift could reappear if only one tracker is updated. Mitigation: tasks require checking deck library, validated DSL ledger, archetype QA docs, YAML comments, test annotations, and gap trackers together.
- Some comments may use historical gap language for context rather than current status. Mitigation: preserve useful history only when it is clearly marked resolved and does not read as a live blocker.
- Focused tests may pass while a full suite has unrelated warnings or failures. Mitigation: require focused card tests for affected cards and record any broader-suite limitation separately.
- The DCGO submodule is initialized at a readable but non-pinned checkout in the current workspace. Mitigation: use it only for reference during reconciliation, and avoid making this proposal depend on the submodule state.
