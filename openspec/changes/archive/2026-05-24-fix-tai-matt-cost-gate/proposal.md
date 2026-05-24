## Why

BT17-081 Tai Kamiya & Matt Ishida's `[All Turns]` "by suspending this Tamer, … gain 1 memory per Greymon-name / Garurumon-name on field" trigger currently grants memory from **every** queued copy of the trigger in the same event chain, even when the Tamer is already suspended from the first copy's cost payment. A May 24 2026 engine-MCP QA pass exercising the BT17 cost-reduction + EoT DNA digivolve chain into AD1-025 Omnimon observed **+4 memory** from a single MetalGarurumon play (two triggers × 2 names) rather than the printed **+2** (one trigger pays the suspend cost; the second trigger cannot, so it inerts).

The printed text reads "by suspending this Tamer" — that's a strict cost. DCGO (`DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_081.cs`) gates the trigger with `CanActivateSuspendCostEffect(card)` which returns `false` when the Tamer is already suspended, so a second simultaneous trigger cannot activate. The substrate to express this gate correctly already shipped in the 2026-05-20 Puppets sweep as `activation_cost: { suspend_self: true }` + `EffectContext::suspend_self_as_cost`. BT17-081 simply predates that sweep and still uses the older `suspend: { target: source }` body step, which suspends unconditionally and grants memory regardless of cost-payability.

This is a YAML-only fix; the engine machinery is in place and tested. The faithfulness regression is concrete (verified live against the running engine) and the no-approximations contract demands per-trigger cost gating.

## What Changes

- **BT17-081 YAML migrates the suspend cost from a body step to a clause-level `activation_cost`.** Move the cost from `process: - suspend: { target: source }` to the new `activation_cost: { suspend_self: true }` clause-level flag on the `[All Turns]` triggered clause. The body retains only the conditional `gain_memory: 1` arms for Greymon / Garurumon presence checks.
- **The `[All Turns]` trigger now inerts on simultaneous re-fires** when BT17-081 is already suspended. Picking a queued T&M trigger from the TriggerOrder prompt re-evaluates the activation cost; if the Tamer is suspended, the trigger silently fails to activate and no memory is granted. This matches DCGO's `CanActivateCondition` gate exactly and makes the printed "by suspending" semantics enforceable per-trigger.
- **Mechanical "deny first / accept second"** becomes expressible without a new decline UI: the player orders triggers in the TriggerOrder prompt by accepting the trigger they want to resolve first; subsequent T&M triggers inert because the cost can't be paid. This closes the practical user-visible ordering need surfaced by the QA pass.
- **Regression coverage** in `code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs`: a new behavioral test stages two simultaneous T&M All Turns triggers in the same event chain (MG play + Agumon→WG digivolve in MG's mandatory effect chain). Asserts: memory delta is +2 (not +4), BT17-081 is suspended exactly once, the second trigger's effect-queue entry resolves with no body run.

## Capabilities

### New Capabilities

(none — all changes modify existing capabilities)

### Modified Capabilities

- `dna-omnimon-archetype-coverage`: BT17-081's `[All Turns]` clause now declares its suspend cost as a clause-level `activation_cost`, and the per-trigger cost gate matches DCGO's `CanActivateSuspendCostEffect` exactly so simultaneous triggers don't double-pay the reward.

## Impact

- **Card YAML** — `code/digimon-engine/cards/bt17/BT17-081.yaml` (clause 1 only — drop the `suspend` body step, add `activation_cost: { suspend_self: true }`).
- **Behavioral tests** — `code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs` (new test covering simultaneous-trigger cost gating).
- **No engine changes** — `activation_cost: { suspend_self: true }` substrate already shipped with the Puppets sweep (2026-05-20). `EffectContext::suspend_self_as_cost` returns `false` on already-suspended sources, providing the per-trigger gate.
- **No new dependencies**. No breaking API changes — printed-text semantics get more faithful, players who relied on the bug get less (correct) memory.
- **Sibling cards** — BT13-101 / P-136 already use this pattern (per `qa/dsl-vocab-gaps.md` PUPPETS-G023 resolved entry). BT17-081 is one of the last cost-bearing All Turns triggers still on the old body-step pattern.
