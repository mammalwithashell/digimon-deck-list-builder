## Context

ST-4 cards are present in `data/cards.json` and have metadata JSON under `code/digimon-engine/cards/st4/`, but there are no `ST4-*.yaml` DSL specs. The Rust engine registry is built from DSL-authored card specs, so metadata alone does not make these cards implemented for gameplay, training, or deck-building allowlists.

The worldwide ST-4 deck contains 16 unique cards and 54 playable cards total: 4 Digitama plus a 50-card main deck. The card set is mechanically small and mostly fits existing DSL vocabulary: reveal search, inherited DP modifiers, attack-triggered DP, Blocker, Piercing, Digi-Burst source trash, suspend, return-to-hand, security option activation, and tamer suspend-as-cost memory gain.

The only known faithful-authoring risk is `ST4-11 MegaKabuterimon`: its inherited once-per-turn effect triggers only when the source carrier deletes its battle opponent in battle and survives. Existing engine context exposes battle-opponent information, but DSL predicates need a reusable way to bind that condition without over-firing on unrelated battle deletions.

## Goals / Non-Goals

**Goals:**

- Author faithful Rust DSL YAML for `ST4-01` through `ST4-16`, including vanilla/no-op registration where needed.
- Add behavioral tests for all non-vanilla ST-4 effects, with TDD coverage for player-visible choices, target filters, timing, and once-per-turn behavior.
- Add a reusable DSL predicate/helper for inherited battle-deletion-survivor effects, then use it for ST4-11.
- Add a canonical ST-4 Giga Green starter deck recipe using the worldwide 54-card list.
- Reconcile ST-4 card readiness/gap trackers only after the tests prove the behavior.

**Non-Goals:**

- No action-space or tensor-profile changes.
- No legacy Python card scripts.
- No broad green-archetype audit beyond the 16 ST-4 cards.
- No approximation of ST4-11 by triggering from any battle deletion while the inherited source carrier is present.

## Decisions

### D1 - Treat ST-4 as a small complete deck batch

Implement all 16 cards together rather than only effect-bearing cards. The starter deck is useful as a playable unit, and vanilla cards still need to be visible to implemented-card discovery if the deck recipe is used by Rust-backed training or deck tooling.

Alternative considered: only author non-vanilla YAML. That risks a starter-deck recipe containing cards that remain filtered out by implemented-card loading.

### D2 - Use existing DSL primitives wherever possible

Most cards should be YAML-only over existing vocabulary:

- ST4-01 inherited level-gated DP aura.
- ST4-03 and ST4-10 reveal/search/remainder placement.
- ST4-04 and ST4-06 inherited attack-target DP modifiers.
- ST4-08 Blocker plus attack memory loss.
- ST4-12 attack/block suppression until the opponent's next turn.
- ST4-13 Piercing plus Digi-Burst 2 suspend.
- ST4-14 tamer suspend-as-cost memory gain plus security play.
- ST4-15 and ST4-16 option main/security effects.

Alternative considered: raw Rust implementations for the whole deck. That would bypass the DSL coverage goal and make later card audits less reusable.

### D3 - Make ST4-11 a reusable DSL vocabulary slice

Add a predicate/helper equivalent to "event target is the source carrier's battle opponent and the source carrier survived the battle" and combine it with existing owner/cause/once-per-turn gates. The helper should work for inherited effects, not just face-up top-card effects.

Alternative considered: model ST4-11 as `on_leave_field` with `event_cause: battle_deletion` and `event_target_owner: opponent`. That over-fires when a different friendly Digimon deletes an opponent's Digimon, which violates the no-approximations policy.

### D4 - Build tests from printed behavior, not current tracker state

Use `data/cards.json` as the card-text source of truth, with rule docs and existing YAML examples as implementation references. Tests should prove the exact printed distinctions: optional costs are declinable, attack-target filters do not apply to player attacks, security option add-to-hand differs between ST4-15 and ST4-16, and ST4-11 does not fire when the carrier dies.

Alternative considered: rely on YAML compiler snapshots only. That would not prove runtime timing, pending-selection, or battle-context behavior.

## Risks / Trade-offs

- **ST4-11 predicate shape may already exist under another name** -> First spike the current predicate/effect-context code and reuse any existing helper before adding vocabulary.
- **Optional tamer triggers can interact with the in-flight optional-prompt fix** -> Inspect `fix-outer-optional-prompt-trigger-ctx` before implementing ST4-14 and avoid duplicating or masking that active change.
- **Vanilla YAML registration format may differ from effect YAML examples** -> Confirm the minimal no-effect YAML accepted by the compiler with one vanilla card before bulk-authoring the remaining vanilla cards.
- **Starter-deck recipe location may not be `data/deck_library.json`** -> Follow the current deck-fixture convention discovered in the repo; the requirement is a canonical recipe, not a specific file path.
- **Security option handling is easy to conflate** -> Test ST4-15 and ST4-16 separately because only ST4-15 adds itself to hand after activating its main effect.

## Migration Plan

1. Spike minimal registration by adding or compiling one vanilla ST-4 YAML locally.
2. Add the ST4-11 DSL predicate/helper with focused DSL/engine tests.
3. Author ST4 cards in low-risk groups: vanillas, simple inherited/keyword effects, reveal/search effects, options/tamer, then ST4-11.
4. Add the ST-4 deck recipe after all referenced card IDs load as implemented.
5. Reconcile gap/readiness trackers and run the targeted Rust test suites.

Rollback is file-scoped: card YAMLs, the ST4 deck recipe, and the ST4-11 vocabulary slice can be reverted independently. No database migration or model artifact migration is involved.

## Open Questions

- Does an existing DSL predicate already expose `EffectContext::battle_opponent_of(source)` in a way ST4-11 can use?
- What is the preferred current home for starter-deck recipes: `data/deck_library.json` or a separate fixture/catalog?
- Does the active optional-prompt change alter the expected authoring pattern for ST4-14's "may suspend this Tamer" cost?
