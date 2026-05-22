## Why

The `unblock-medusamon-partial-cards` change closed 5 of the 7 substrate gaps blocking PARTIAL Medusamon cards. Two were deferred to a design spike because both were feared to need a new action ID — which would move `ACTION_SPACE_SIZE` and force RL-model retraining. That spike (task group 6 of the parent change) is now done. **Its key finding: neither gap needs the action space to grow.** Both can reuse existing action IDs, so this follow-up is far lighter than the parent change's design assumed — no tensor/decoder-width change, no retraining.

This change closes the last 2 gaps and unblocks **BT24-016 Lamiamon** and **ST22-08 Offensive Plug-In V** (plus BT22-013, BT22-026, BT16-027 outside the archetype).

## What Changes

- **G-ACTIVATED-DIGIVOLVE-EXECUTION** (engine) — `CompiledAltPathKind::ActivatedDigivolve` has no engine execution route; `dna_digivolve.rs` matches only `Digivolve` / `DnaDigivolve` / `BlastDnaDigivolve`. **Spike finding: reuse the existing `DIGIVOLVE` action range** (`400..1000`, encoding `(hand_index, field_index)`) — an activated digivolve is exactly "a hand card digivolves onto a field permanent," the shape that range already encodes. Work: extend `action/mask.rs` to mask a `DIGIVOLVE` action in for a hand card with a satisfiable `activated_digivolve` alt-path; extend `action/decode.rs` to route it; add the execution route (run `extra_cost`, then digivolve at the alt-path `cost` with `ignore_requirements`). **No `ACTION_SPACE_SIZE` change.** Unblocks **BT24-016**; also BT22-013, BT22-026, BT16-027.
- **G-LINK-OPTION-DUAL-PLAY-MODE** (engine) — `classify_option_subtype` (`game_actions.rs:146`) is first-match-wins (`Delay` → `Training` → `Link` → `Standard`), so a card is exactly one subtype; a Plug-In Option with both a `[Main]` effect and Link Requirements cannot be both. **Spike finding: reuse the existing `PLAY_HAND` action** — when a played Option supports more than one play mode, `play_option_from_hand` installs a mode-select pending selection (`[Main] Option` vs `Plug in via Link`); each branch routes to its existing dispose path. The mode choice surfaces as a normal selection, satisfying the no-approximations policy. Work: `classify_option_subtype` returns a mode *set*; `play_option_from_hand` installs the mode prompt when the set has >1 entry. **No `ACTION_SPACE_SIZE` change.** Unblocks **ST22-08**.

Both fixes are TDD-first, with the two cards' currently-omitted clauses re-authored and their structural/behavioral tests un-blocked.

## Capabilities

### New Capabilities
- `activated-digivolve-execution`: a `[Hand]`-timed activated-digivolve alt-path is offered to the action space and executes — the hand card digivolves onto a chosen field permanent at the alt-path's cost, ignoring printed digivolution requirements, after paying any `extra_cost`.
- `option-dual-play-mode`: an Option card that supports more than one play mode (a Standard `[Main]` Option *and* a Link Option) surfaces a mode-select choice when played, and each mode resolves through its own disposal path.

### Modified Capabilities
<!-- None — both are new engine capabilities; no existing OpenSpec spec changes at the requirement level. -->

## Impact

- **Engine** (`code/digimon-engine/`): `action/mask.rs`, `action/decode.rs` (route the reused IDs), `dna_digivolve.rs` or a sibling (activated-digivolve execution), `game_actions.rs` (`classify_option_subtype` rework + dual-mode play prompt), `option_lifecycle.rs` (dispose routing).
- **No RL-contract impact** — `ACTION_SPACE_SIZE` (2192) and `TENSOR_SIZE` are unchanged; both gaps reuse existing action IDs. No model retraining.
- **Cards**: re-authoring BT24-016 clause 1 (currently structural-only) and ST22-08's Link-Option mode (currently modelled as Standard-only) → both move from `PARTIAL` to `IMPLEMENTED`.
- **Risk to flag in design**: a `DIGIVOLVE` action could be ambiguous if a hand card has *both* a standard digivolve match and an activated-digivolve alt-path onto the same field permanent — the design must pick a disambiguation (prefer one, or install a mode prompt as in gap 2).
- **Trackers**: on completion, `G-ACTIVATED-DIGIVOLVE-EXECUTION` and `G-LINK-OPTION-DUAL-PLAY-MODE` move to `qa/resolved-gaps.md`.
