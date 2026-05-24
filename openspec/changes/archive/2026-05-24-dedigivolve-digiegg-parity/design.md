## Context

The Rust engine already has a generalized `EffectContext::de_digivolve(target, stop_at_level, amount)` primitive and the DSL already supports formula-valued `amount_fn`. The observed TS Olympos versus Imperialdramon game exposed a bad composition: BT24-041 Minervamon uses a formula-valued amount through the DSL, but the DSL lowered omitted `stop_at_level` to `None`, allowing a normal `<De-Digivolve>` effect to peel to the Digi-Egg base.

DCGO provides the intended shape for this card and mechanic. `BT24_041.cs` computes `deDigiCount` from the controller's battle-area Digimon, selects one opposing Digimon, then calls `new IDegeneration(permanent, 1, activateClass).Degeneration()` once per count. `IDegeneration` stops before trashing when the current permanent's level is 3. The Rust implementation should preserve that result without adding DCGO as a dependency.

## Goals / Non-Goals

**Goals:**
- Ensure standard `<De-Digivolve N>` effects stop at level 3, including when `N` comes from `amount_fn`.
- Ensure DSL-authored De-Digivolve uses the standard level 3 floor by default, including BT24-041 Minervamon's formula-valued De-Digivolve.
- Add a battle-area invariant that a bare Digi-Egg top card exposed by stack mutation is immediately removed from the field.
- Cover both the normal `[Digi-Egg, Lv3, higher]` case and the defensive bare-Digi-Egg cleanup case with Rust behavioral tests.
- Keep the debug MCP/headless/RL surfaces observing only legal post-resolution states.

**Non-Goals:**
- Change `ACTION_SPACE_SIZE`, tensor profiles, action IDs, or RL observation contracts.
- Rework optional De-Digivolve count selection across the whole engine.
- Rewrite the De-Digivolve primitive into a DCGO coroutine clone.
- Add DCGO as a runtime or build dependency.
- Broaden this work into unrelated TS Olympos card fixes.

## Decisions

### Decision: Fix DSL lowering and lock the engine invariant

The DSL compiler should lower omitted `stop_at_level` on `de_digivolve` steps to `Some(3)`. The existing primitive already accepts this floor, and representative YAML such as BT23-096 uses it for standard De-Digivolve. This addresses the observed Minervamon bug and any future standard De-Digivolve YAML omission without requiring card-local fixes.

Alternative considered: patch only BT24-041 YAML. That would fix the observed card but leave the same bug shape available for the next standard De-Digivolve authoring omission.

### Decision: Add post-stack-mutation Digi-Egg cleanup at the engine layer

Even with the DSL floor fixed, the engine should not allow battle-area state to contain a permanent whose top card is a Digi-Egg. Add a shared cleanup helper called after De-Digivolve and other stack mutation sites that can expose a new top card. If a battle-area permanent's top card is a Digi-Egg, route it through the rule-aligned leave-field path so the slot disappears and the card is moved to its owner's trash.

Alternative considered: rely only on De-Digivolve level floors. That fixes normal De-Digivolve, but leaves the same illegal state reachable through source-removal effects that strip the Lv3 before or during a later stack mutation.

### Decision: Preserve formula-valued De-Digivolve as a single resolved amount

Rust can keep evaluating Minervamon's amount once at resolution time and pass the resulting count to `ctx.de_digivolve(..., Some(3), Some(count))`. For a fixed level-3 floor, this reaches the same board state as DCGO's repeated `IDegeneration(..., 1, ...)` loop for this card.

Alternative considered: lower formula-valued De-Digivolve into repeated one-card primitive calls. That is closer to the DCGO call graph but adds more moving parts and does not change the expected end state for the current card.

### Decision: Test with real BT24-041 and synthetic invariant coverage

Use BT24-041 behavioral tests to prove the card no longer peels a normal stack past level 3. Add a separate primitive/invariant test that intentionally exposes a Digi-Egg top card so the cleanup behavior is covered even if standard De-Digivolve floors prevent the normal path.

Alternative considered: only replay the TS Olympos versus Imperialdramon game. The replay is useful smoke evidence, but focused Rust tests are smaller, deterministic, and easier to preserve as regression coverage.

## Risks / Trade-offs

- Risk: Digi-Egg cleanup placement could conflict with a rare effect that intentionally treats a Digi-Egg as an active battle-area permanent. Mitigation: scope cleanup to battle-area permanents whose top card is a Digi-Egg after stack mutation; breeding-area Digi-Eggs and Digi-Eggs used as sources remain valid.
- Risk: Removing the permanent can shift battle-area indices while an effect still holds handles. Mitigation: use existing handle-shift patterns from zombie permanent cleanup or run cleanup at mutation boundaries where no later same-target handle is reused.
- Risk: Some existing YAML omits `stop_at_level` for standard De-Digivolve. Mitigation: audit `code/digimon-engine/cards/**/*.yaml` for `de_digivolve` steps and add floors only where printed text is standard `<De-Digivolve>`.
- Risk: The exact leave-field cause for an exposed Digi-Egg could affect triggers. Mitigation: treat the cleanup as rules-based removal/deletion, document the cause, and verify no ordinary De-Digivolve path emits extra OnDeletion for a Lv3 that should have remained.

## Migration Plan

1. Add failing tests for BT24-041 on `[Digi-Egg, Lv3, Lv4]` and for direct Digi-Egg exposure cleanup.
2. Update DSL lowering so authored `de_digivolve` steps default to the standard level 3 floor, and audit standard De-Digivolve YAML for omissions.
3. Implement shared post-stack-mutation Digi-Egg cleanup in Rust and call it from De-Digivolve/source-removal sites that can expose a new top card.
4. Run focused Rust behavioral tests, De-Digivolve primitive tests, and a short MCP replay/smoke with TS Olympos versus Imperialdramon.
5. Update parity/gap notes if the implementation discovers broader standard De-Digivolve omissions.

Rollback is a normal code revert; no persisted data, tensor contract, model artifact, or database migration is involved.

## Open Questions

- Which source-removal helpers beyond `de_digivolve` can expose a Digi-Egg as the top battle-area card today, and should all be wired in this change or covered by a narrower first pass?
- Should the cleanup emit an explicit engine event for debug MCP visibility, or is the existing removal/deletion event surface sufficient?
