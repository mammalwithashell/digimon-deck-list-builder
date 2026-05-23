## Why

The Rocks archetype's verdict ledger (`qa/qa-reports/validated_cards_dsl.json`) shows 2 BLOCKED + 30 PARTIAL cards, but those verdicts were frozen at the 2026-05-04 pool pass and are now badly stale: Phase 2 Tracks E/I/J have since closed most of the cited substrate gaps. Verification of actual post-merge test state shows EX10-003 (tracked BLOCKED) is fully implemented and passing, and the entire 40-card processed pool has only **9 ignored tests across 4 cards**. The real remaining work is an authoring re-audit plus **5 genuine substrate primitives** — not 32 blocked cards. This change drives the archetype to a verified, fully-faithful end state.

## What Changes

- **Authoring re-audit** of all 30 PARTIAL Rocks cards: reclassify stale-PARTIAL cards whose substrate already landed, and author the omitted clauses whose primitives now exist. No engine code required for this block.
- **Close 5 substrate gaps** that genuinely block faithful implementation:
  - **B1** — `carrier_trait_has` predicate so an inherited aura `condition` can gate on the carrier's traits (BT21-021 inherited Rush).
  - **B2** — `move_from_breeding` DSL verb + optional level-filtered prompt wrapper over the existing `EffectContext::move_from_breeding_by_effect` engine method (P-130 `[On Play]`).
  - **B3** — union-zone cost selector spanning hand ∪ own digivolution-stack sources, with a per-card trait filter (EX11-065 `[Start of Your Main Phase]`).
  - **B4** — `flip_security_face_up` no-choice primitive + a "when your Digimon checks a face-up security card" observer timing (BT20-055).
  - **B5** — Delay-on-attack support: delay-lowering for attack timings, combat-dispatch fan-out to event-gated delays, and a delay-context attacker predicate (BT23-096 `<Delay>` clause).
- **Author the final clause** of the 5 substrate-blocked cards once their primitive lands, with TDD behavioral tests.
- **Reconcile the trackers** to verified state: update `validated_cards_dsl.json` Rocks verdicts, prune stale `#[ignore]` markers, and move closed gaps to `qa/resolved-gaps.md`.

## Capabilities

### New Capabilities
- `rocks-archetype-coverage`: End-state coverage guarantee for the Rocks archetype — every card in the resolved pool has a faithful DSL implementation and behavioral tests, the 5 substrate primitives (B1–B5) exist and are exercised, the verdict ledger reflects verified state, and no `#[ignore]` marker cites an already-closed gap.

### Modified Capabilities
<!-- None. The B1–B5 substrate primitives are new DSL/engine surface introduced under this capability; no existing spec's requirements change. -->

## Impact

- **Card content:** `code/digimon-engine/cards/<set>/*.yaml` — Rocks card specs (re-authored / completed clauses).
- **Tests:** `code/digimon-engine/tests/cards_behavioral/<set>/*.rs` — Rocks behavioral tests; `#[ignore]` markers pruned.
- **Engine (Rust):** `code/digimon-engine/src/` — `dsl_cards/predicate.rs` (B1, B5), `effect_context/` (B2, B3, B4), `combat.rs` + `effect_queue.rs` (B5), security primitives (B4), timing/observer wiring (B4).
- **DSL crate:** `code/digimon-dsl/src/` — new step/predicate vocabulary and lowering (`step.rs`, `lower_*.rs`, `compiled.rs`) for B1–B5.
- **Trackers:** `qa/qa-reports/validated_cards_dsl.json`, `qa/dsl-vocab-gaps.md`, `qa/archetype-qa/engine-gaps.md`, `qa/resolved-gaps.md`, `qa/archetype-qa/dsl/rocks.md`.
- **Cross-archetype reuse:** B3 (union-zone cost) overlaps the Royal Knights `G-UNION-HAND-TRASH-SOURCE-COST`; B4 (face-up security lifecycle) overlaps a Dark Masters audit item. Scoping these generically benefits multiple archetypes.
- **No breaking changes.** Action space and tensor shape are unaffected unless B3/B4 require a new pending-selection sub-range; if so, that is an additive `ACTION_SPACE_SIZE` change handled per the existing Group-5 contract note.
