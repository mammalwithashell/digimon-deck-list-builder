## Context

Rust headless training currently depends on `DigimonEnv -> OpponentWrapper -> ActionMasker` to expose exactly the next legal engine decision. A smoke generalist run showed the greedy baseline playing a BG Imperial setup Digimon, crossing memory to `-3`, and installing a mandatory On Play reveal selection. The engine then moved to `GamePhase::EndTurn` while `pending_selection` still existed, leaving the mask in a pass-only fallback phase and making pass a no-op until the safety cap forced a draw.

DCGO's turn processing reference runs end-turn checks around automatic processing and stacked effects; it does not bury unresolved player choices behind a generic end phase. In this repository, printed card text, rules docs, and the Rust pending-selection contract remain authoritative; DCGO is only a reference for phase-flow shape.

## Goals / Non-Goals

**Goals:**
- Preserve every pending selection as the next headless/RL decision until it resolves.
- Defer turn-end rotation when a card action crosses memory but creates a pending selection or follow-up effect chain.
- Keep action masks and action decoding aligned with `pending_selection.valid_action_ids`.
- Verify the greedy baseline can continue after playing a setup card with a mandatory On Play choice.
- Verify a short Rust-backed generalist smoke run no longer draws due to a pass loop caused by `EndTurn`/pending-selection mismatch.

**Non-Goals:**
- Redesign the whole Rust phase state machine.
- Change `ACTION_SPACE_SIZE`, tensor profiles, or RL observation/action contracts.
- Improve greedy strategic quality beyond avoiding engine-induced soft-locks.
- Implement missing card effects or DSL vocabulary unrelated to turn progression.
- Add DCGO as a production dependency.

## Decisions

### Decision: Fix the engine invariant, not the greedy heuristic

Greedy returned pass only after the engine exposed a pass-only state while a mandatory selection was still pending. The fix should make the engine preserve pending selections as active decisions; greedy should not need special-case logic for a broken phase.

Alternative considered: teach greedy to inspect pending selections even in `EndTurn`. This would mask the symptom but leave masks, model training, and other policies exposed to an invalid decision state.

### Decision: Gate turn-end checks on unresolved pending selections

`Game::check_turn_end()` should not call `end_turn()` while `pending_selection` is present. Selection resolution and effect-queue resumption paths already have natural boundaries where they can call `check_turn_end()` again after choices resolve.

Alternative considered: allow `EndTurn` to route pending-selection actions. This is broader and risks making `EndTurn` a second selection phase. The cleaner invariant is that selection phases remain selection phases, and turn-end rotation waits.

### Decision: Add regression coverage at the Rust behavior layer and verify through Python/RL smoke

The primary regression should live under Rust engine tests and assert phase/mask/turn progression directly. A Python smoke should confirm the PyO3 and `OpponentWrapper` path behaves as training expects.

Alternative considered: only add a Python training test. That would catch the symptom but make it harder to isolate whether the Rust state machine, bindings, wrapper, or policy caused the failure.

### Decision: Keep DCGO reference use explicit and bounded

Use DCGO to compare phase-flow sequencing around memory crossing, automatic processing, and end turn. Do not copy DCGO card-specific optionality or implementation flags without checking printed text and repository rules docs.

## Risks / Trade-offs

- Pending-selection deferral might reveal another missing follow-up `check_turn_end()` call after a selection resolves. Mitigation: include a test where memory is negative before selection resolution and assert the turn rotates after the selection completes.
- Some card/effect paths might intentionally park in `EndOfTurnAction`. Mitigation: scope the fix to unresolved `pending_selection`, and preserve existing `EndOfTurnAction` handling for Vortex/Overclock/MayAttack.
- A narrow test using one BG Imperial card could miss other selection kinds. Mitigation: test the generic pending-selection invariant and include the observed reveal-bucket case as a representative regression.
- DCGO submodule checkout may not match the pinned commit exactly in the current worktree. Mitigation: cite DCGO only as phase-flow reference and rely on local Rust tests for acceptance.

## Migration Plan

1. Add a failing Rust regression that reproduces memory crossing plus mandatory pending selection.
2. Adjust the turn-end/pending-selection invariant in Rust.
3. Verify Rust action masks expose selection actions while the selection is pending.
4. Verify selection resolution resumes turn-end progression when memory remains crossed.
5. Rebuild/use existing PyO3 bindings as needed and rerun a small Rust-backed generalist smoke.

Rollback is a normal code revert; no persisted data, tensor, action-space, or model metadata migration is required.

## Open Questions

- Which existing Rust test fixture gives the smallest stable reproduction: a real BG Imperial deck/card path (`BT12-047`) or a synthetic test card that installs a mandatory selection after paying memory?
- Do any delayed-option or pending-option paths require the same deferral beyond `pending_selection`, or is `pending_selection` sufficient for this regression?
