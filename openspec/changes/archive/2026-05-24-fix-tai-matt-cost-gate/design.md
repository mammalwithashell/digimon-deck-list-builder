## Context

BT17-081 Tai Kamiya & Matt Ishida's `[All Turns]` trigger fires "When one of your Digimon is played or digivolves, by suspending this Tamer, … gain 1 memory per Greymon-named Digimon, gain 1 memory per Garurumon-named Digimon." The "by suspending" clause is the trigger's activation cost — DCGO (`BT17_081.cs:58-86`) implements this as `CanActivateCondition: CanActivateSuspendCostEffect(card)` which returns `false` when the Tamer is already suspended.

The current BT17-081 YAML (`code/digimon-engine/cards/bt17/BT17-081.yaml:99-133`) was authored before the engine grew first-class triggered-activation-cost support. It encodes the cost as a body step (`process: - suspend: { target: source }`) which always runs and then unconditionally evaluates the memory-grant arms. The engine grew `activation_cost: { suspend_self: true }` + `EffectContext::suspend_self_as_cost` as part of the 2026-05-20 Puppets sweep (`qa/dsl-vocab-gaps.md` PUPPETS-G023), but BT17-081 never migrated.

The user-visible bug shows up when two T&M triggers queue from the same event chain (e.g. MetalGarurumon's `[On Play]` plus the Agumon→WG digivolve fired by MG's mandatory effect). The Rust engine's TriggerOrder lets the player pick which to resolve first; whichever they pick suspends T&M and grants memory, then the next pick also suspends (no-op because already suspended) and **still** grants memory. The MCP-driven QA replay observed +4 memory from a sequence that prints +2.

The engine machinery to fix this is in place; this change is a YAML migration plus a behavioral regression test.

## Goals / Non-Goals

**Goals:**

- BT17-081's `[All Turns]` trigger grants memory **at most once per event chain** when its activation cost can only be paid once (i.e. the Tamer suspends only once).
- The "by suspending" cost is enforced **per-queued-trigger** by `EffectContext::suspend_self_as_cost`, matching DCGO's `CanActivateSuspendCostEffect` gate.
- Mechanical "deny first, accept second" semantics become expressible via TriggerOrder ordering: picking the trigger you want to accept first pays the cost; picks of later T&M triggers in the same bundle inert because the cost can't be repaid.
- Add a behavioral test pinning the new behavior so the regression doesn't return.

**Non-Goals:**

- Adding a separate per-trigger accept/decline UI inside `TriggerOrder` bundles. The engine intentionally limits the pre-cost decline prompt to single-trigger bundles (`effect_queue.rs:893-907`); this change does NOT alter that limitation. Once `activation_cost` is wired, the cost gate handles the multi-trigger case correctly without a UI change.
- Migrating other cards still on the body-step `suspend` pattern. Other cards (if any remain) get audited in a follow-up sweep. This change scopes to BT17-081 only.
- Changing the order in which BT17-081 evaluates the Greymon / Garurumon name checks. The body remains "if Greymon-name on field, +1 memory; if Garurumon-name on field, +1 memory" with independent conditional arms. Only the cost step moves.
- Changing BT17-081's `[End of Your Turn]` Omnimon-attack clause or its `[Security]` clause. Those clauses are correct and unaffected.

## Decisions

**Decision 1: Use clause-level `activation_cost: { suspend_self: true }` instead of the older body-step pattern.**

This is the post-Puppets-sweep idiom for "by suspending this Tamer/Digimon" activation costs. The substrate (`EffectContext::suspend_self_as_cost`) returns `false` when the source is already suspended, which causes the engine's `run_queued_effect` path to skip the body. BT13-101 and P-136 already use this idiom; BT17-081 becomes a third consumer.

Alternative considered: leave the body-step `suspend` and add an inline check (e.g. `if not_suspended: { source } then [ suspend, gain_memory, ... ]`). Rejected because (a) it duplicates the cost-gating logic the substrate already provides; (b) it doesn't compose with the engine's pre-cost decline machinery for single-trigger bundles; (c) it diverges further from DCGO's `CanActivateCondition` shape.

**Decision 2: Drop the `suspend: { target: source }` body step entirely.**

The clause-level `activation_cost` is the single source of truth for both the cost-payability gate AND the actual suspend mutation. The engine's cost machinery applies the suspend as part of running `activation_cost_fn`; there's no need to repeat it in the body.

Alternative considered: keep both for "defense in depth" (activation_cost gates and body step double-applies). Rejected because (a) `suspend` is idempotent on an already-suspended card so it's truly redundant, not defensive; (b) it confuses the printed-text reading (the printed cost appears once, encoded once); (c) it would break the body's conditional structure (the `if Greymon … then gain_memory` arms would still need to run even though the cost is satisfied).

**Decision 3: Preserve the body's two independent `if any_permanent` arms unchanged.**

The printed text grants memory **per name present on field**, not as a combined check. Two independent arms (one for Greymon, one for Garurumon) correctly express this. The body stays exactly as-is minus the leading `suspend` step.

**Decision 4: Don't migrate BT17-081's `[End of Your Turn]` clause.**

That clause has no "by suspending" cost (the printed text reads "1 of your Digimon with [Omnimon] in its name may attack a player" — the outer "may" controls activation, no cost). Its `optional: true` flag is correct as-is. Leaving it unchanged keeps this change minimally invasive.

## Risks / Trade-offs

**[Risk] An existing behavioral test pins the +4 memory bug as observed behavior.** → Mitigation: Run the BT17-081 test file before the change; verify which assertions cover the simultaneous-trigger case. Any test pinning observed (buggy) memory should be updated to the printed (correct) +2 outcome. Mark such updates explicitly in the test diff.

**[Risk] Picking the optional trigger from a TriggerOrder bundle no longer auto-grants memory.** → This is a behavioral correction, not a regression. Players who rely on the old behavior were getting more memory than the card prints. Documented in the proposal's "What Changes" section so reviewers see it.

**[Risk] The cost gate now silently inerts subsequent T&M triggers in a bundle.** → DCGO has identical behavior. The engine's TriggerOrder UI does not currently expose a "this trigger was inert because cost could not be paid" diagnostic; if needed, a follow-up could log a `TriggerInert` event for observability. Not in scope here.

**[Risk] The `activation_cost: { suspend_self: true }` flag is at the clause level but the Greymon/Garurumon checks are inside the body — could a reader misread the printed-text mapping?** → Mitigation: keep the YAML comments explicit about which fragment of printed text maps to which YAML construct. The existing BT17-081 YAML has rich header comments; preserve and extend them.

## Migration Plan

1. Edit `code/digimon-engine/cards/bt17/BT17-081.yaml` — drop the `suspend` body step, add `activation_cost: { suspend_self: true }` to the `[All Turns]` clause header.
2. Update the YAML's section header comment ("Cost: suspend this Tamer …") to point at the new clause-level construct.
3. Add the simultaneous-trigger behavioral test in `code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs`. The test should:
   - Construct a scenario that queues two simultaneous T&M triggers (DebugRunner builder; play MG + trigger Agumon→WG via MG's effect).
   - Resolve them via `TriggerOrder` picks.
   - Assert: P0 memory delta from BT17-081 triggers is exactly +2 (one trigger paid the cost and granted memory; the second trigger inerted).
   - Assert: BT17-081 is `is_suspended: true` after both triggers resolve (suspended exactly once).
4. Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_081` and verify the new test passes.
5. Run the full `cards_behavioral` suite to catch any regressions on tests that may have indirectly pinned the buggy +4 memory.
6. No engine code changes — `activation_cost: { suspend_self: true }` is fully implemented and tested via the Puppets sweep.

**Rollback:** revert the YAML diff. The substrate machinery is shared and stays in the engine regardless of whether BT17-081 uses it.

## Open Questions

- Does any other archetype currently rely on the +4 memory behavior in a deck-level optimization? (Unlikely — the bug doesn't appear in any qa/qa-reports/ entry — but worth a `git log --diff-filter=M -- code/digimon-engine/cards/bt17/BT17-081.yaml` audit before merge to see if the bug was ever surfaced and "fixed" by working around it.)
- Should the new behavioral test also assert the per-trigger inert state via `effect_queue` inspection (e.g. confirm the second trigger's queued entry resolves with no body run)? → Probably yes for stronger pinning; left for the implementer.
