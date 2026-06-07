## Why

`G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` (tracked in `qa/dsl-vocab-gaps.md` since 2026-05-03) is the engine/DSL primitive that lets a card install a NEW triggered effect onto another permanent — typically an opponent's Digimon — with a turn-scoped expiry. The DSL today exposes grants for STATIC effects only (`grant_keyword`, `add_modifier`/`add_dp_modifier`, `grant_effect_immunity`); none install a clause that itself fires on a future trigger (`when_attacking`, `when_digivolving`, `[End of Your Turn]`, …) on the granted permanent.

The judge-quiz discovery wave confirmed this primitive blocks three scenarios, at the engine level, regardless of card authoring:

- **Q2** — Ice Wall! (EX1-068) "[Main] All of your opponent's Digimon gain '[When Attacking] lose 2 memory' until the end of their next turn." Its `[Main]` clause is OMITTED from the YAML today (only `[Security]` is implemented). The judge ruling hinges on Medusamon's `<Progress>` then making it immune to that GRANTED effect — but the grant can't even be installed.
- **Q16** — Lilithmon (EX6-057) gives "[End of Your Turn] Delete this Digimon." to the opponent's Paildramon. The judge: the granted effect "counts as belonging to that Digimon," so when Paildramon deletes itself, `<Partition>` does NOT trigger (leaving by its own effect).
- **Q17** — same Lilithmon grant onto Magnamon (X Antibody); Magnamon X's `[When Digivolving]` immunity then REMOVES the granted effect, so the end-of-turn delete does not activate.

Beyond the quiz, a DCGO grep for `UntilOpponentTurnEndEffects.Add` / `UntilOwnerTurnEndEffects.Add` returns ~20+ cards across sets that need this same "grant a triggered effect with turn-scoped expiry" shape.

## What Changes

- **Engine — granted-triggered-effect modifier slot.** Add a typed slot (e.g. `ModifierType::GrantedTrigger` carrying a `CompiledTriggeredClause` + `Expiry`) to `ModifierRegistry`, so a permanent can carry an installed triggered clause that expires on a turn boundary (reusing the existing expiry-tick infrastructure).
- **Engine — runtime dispatch consults granted clauses.** When the engine fires a timing on a permanent (e.g. `WhenAttacking`, `EndOfYourTurn`), it SHALL also enumerate that permanent's granted-trigger slots and resolve their bodies, with the granted clause's controller/cause attributed to the GRANTED permanent's controller (not the granter).
- **DSL — `grant_triggered_effect` step.** A new `CompiledStep::GrantTriggeredEffect` whose payload is a target selector (`of: opponent`, zone, kind/trait filter), a `when:` timing, an inline `process:` body lowered against the GRANTED permanent, and an `expiry:` (e.g. `end_of_opponents_turn`). Parse → compile → lowering.
- **Cause attribution — granted effects belong to the grantee.** An effect a permanent runs from a granted-trigger slot SHALL be attributed as that permanent's OWN effect for the purposes of departure-cause (`<Partition>` does not fire on own-effect departure — Q16) and immunity removal (an immune permanent does not carry the granted effect — Q17).
- **Card content & tests.** Author Ice Wall! (EX1-068) `[Main]` and Lilithmon (EX6-057) `[On Play]` grant clauses; un-ignore judge-quiz Q2/Q16/Q17 and write the per-card behavioral tests.

## Capabilities

### New Capabilities
- `granted-triggered-effects`: A card can install a triggered effect (timing + inline body) onto another permanent with a turn-scoped expiry, surfaced as a typed modifier slot consulted at trigger-fire time. Granted effects are attributed to the grantee's controller for departure-cause and immunity purposes, so `<Partition>` does not fire when a Digimon leaves via a granted self-effect, and an immune Digimon does not carry a granted effect.

### Modified Capabilities
<!-- If the spike finds that an existing capability (e.g. a modifier-registry or trigger-dispatch capability) must change its requirements rather than be extended, a MODIFIED delta will be added then. The granted-trigger slot, dispatch consultation, DSL step, and cause attribution are all NEW surface. -->

## Impact

- **Engine (Rust):** `code/digimon-engine/src/modifiers.rs` (new `GrantedTrigger` payload + slot), `code/digimon-engine/src/enums.rs` (`ModifierType`), the trigger-fire/dispatch path (`effect_queue.rs` / `game_actions.rs`) to consult granted slots, and the departure-cause + immunity attribution sites (`keyword_effects.rs` `<Partition>`, `game.rs` immunity).
- **DSL crate:** `code/digimon-dsl/src/` — `step.rs` (`GrantTriggeredEffect` shape), `compile.rs` + `compiled.rs` (inline `CompiledTriggeredClause` payload), `lower_*.rs` (lower against the granted permanent).
- **Card content:** `code/digimon-engine/cards/ex1/EX1-068.yaml` (Ice Wall! `[Main]`), `code/digimon-engine/cards/ex6/EX6-057.yaml` (Lilithmon `[On Play]`).
- **Tests:** un-ignore `judge_quiz` Q2/Q16/Q17; new per-card behavioral tests; engine unit tests for the granted slot + expiry + cause attribution.
- **Trackers:** move `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md`.
- **RL contract:** no action-space/tensor change expected; granted triggered effects resolve through existing queue/selection paths (any choice surfaces additively per the existing contract).

## Non-Goals

- The three engine gaps in `fix-judge-quiz-engine-gaps` (≤0-DP rules-check, Digi-Egg routing, on-trash sync/defer). Q16 additionally needs the `<Partition>` cause-filter (already present) plus this change's cause-attribution; no overlap with that change's deletion work.
- A general "grant any effect" framework — scope is triggered effects with a turn-scoped expiry (the `UntilOpponentTurnEndEffects`/`UntilOwnerTurnEndEffects` family). One-shot or permanent grants are out of scope.
- Authoring the other ~20 consumer cards — this change authors only Ice Wall! and Lilithmon (the Q2/Q16/Q17 cards); the rest land via normal card authoring once the primitive exists.
