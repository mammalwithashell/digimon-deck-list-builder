## Context

This design records the **action-space spike** run as task group 6 of the parent change `unblock-medusamon-partial-cards`. The parent change deferred two gaps — G-ACTIVATED-DIGIVOLVE-EXECUTION and G-LINK-OPTION-DUAL-PLAY-MODE — because both were feared to need a new action ID, which would move `ACTION_SPACE_SIZE` (2192) and force RL-model retraining (per `docs/MODEL_CATALOG.md` / `docs/TRAINING_RUNBOOK.md`).

The spike investigated the actual action-space layout (`code/digimon-engine/src/action/space.rs`) and the relevant engine sites. **Finding: neither gap needs the action space to grow.**

Action-space facts established by the spike:
- `DIGIVOLVE_START..DIGIVOLVE_END` = `400..1000` — digivolve actions, each encoding `(hand_index, field_index)` via `DIGIVOLVE_START + hand * FIELDS_PER_HAND + field` (`action/space.rs:139`). This is exactly "hand card digivolves onto a field permanent."
- `PLAY_HAND_START..PLAY_HAND_END` = `0..30` — one play-from-hand action per hand index.
- `classify_option_subtype` (`game_actions.rs:146`) is first-match-wins: `Delay` → `Training` → `Link` → `Standard`. Any effect with `link_cost.is_some()` reclassifies the whole card as `Link`.

## Goals / Non-Goals

**Goals:**
- Record the spike's conclusion so the follow-up implementation starts from a settled action-space decision.
- Close G-ACTIVATED-DIGIVOLVE-EXECUTION and G-LINK-OPTION-DUAL-PLAY-MODE reusing existing action IDs.
- Keep `ACTION_SPACE_SIZE` and `TENSOR_SIZE` unchanged — no RL retraining.

**Non-Goals:**
- Growing the action space. The spike ruled it out.
- Re-opening the parent change's 5 closed gaps.

## Decisions

### D1-REVISED (2026-05-22) — G-ACTIVATED-DIGIVOLVE-EXECUTION: re-model BT24-016 as a `main_from_hand` clause, no engine code

The task-1.1 investigation found `extra_cost` is unimplemented engine-wide and would need a from-scratch parking runner (see the D1 risk below). A second investigation found a far cleaner path: BT24-016's clause 1 is printed `[Hand][Main]` — a main-phase activation from hand — and the engine already fully supports that:
- `action/mask.rs` masks a Hand `[Main]` action for **any** hand card (no card-kind filter) with a `MainFromHand` effect whose `condition` passes.
- `Game::activate_hand_main` runs a `MainFromHand` effect for **any** card kind (the `Option | Dual` restriction applies only to `OptionMain`).
- `effect_initiated_digivolve` is a working DSL step that digivolves a hand card onto a target permanent at a given cost with `ignore_requirements`; `select_*` steps park and resume via the standard `run_steps` machinery.

**Resolution:** re-model BT24-016 clause 1 from a `kind: activated_digivolve` alt-path to a `when: main_from_hand` triggered clause whose body selects the Elizamon target, selects the Dimetromon from trash, `place_as_bottom_source`, then `effect_initiated_digivolve` (`from_hand` = self, cost 3, `ignore_requirements`). This is faithful to the printed `[Hand][Main]` text, uses only working machinery, and adds **zero engine code**. The `kind: activated_digivolve` DSL kind stays defined but unused by this archetype; a true engine execution route for that alt-path kind is only needed by the 3 out-of-scope cards (BT22-013/026, BT16-027) and is left as a separate open item under G-ACTIVATED-DIGIVOLVE-EXECUTION.

The original D1 (reuse the `DIGIVOLVE` range + build an `extra_cost` runner) is superseded — kept below for the record.

### D1 (superseded) — reuse the `DIGIVOLVE` action range
An activated digivolve (`[Hand][Main]` — BT24-016 Lamiamon: "it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements") is structurally a hand card digivolving onto a field permanent — the exact `(hand_index, field_index)` shape the `DIGIVOLVE` range already encodes. So reuse it:
- **Mask** (`action/mask.rs`): also mask in a `DIGIVOLVE` action for a hand card that has a satisfiable `activated_digivolve` alt-path — its `condition` passes and its `from:` source + `extra_cost` are satisfiable on the candidate field permanent.
- **Decode** (`action/decode.rs`): when a `DIGIVOLVE` action's hand card has no standard digivolution match onto the chosen field permanent but does have a satisfiable `activated_digivolve` alt-path, route to the activated path.
- **Execute**: add a `CompiledAltPathKind::ActivatedDigivolve` route — run `extra_cost`, then digivolve at the alt-path `cost` with `ignore_requirements`.
*Alternative considered:* a dedicated activated-digivolve action range. Rejected — it grows `ACTION_SPACE_SIZE` and forces retraining for zero benefit; the shape is identical to a normal digivolve.

### D2 — G-LINK-OPTION-DUAL-PLAY-MODE: reuse `PLAY_HAND` + a mode-select prompt
A Plug-In Option that is both a Standard `[Main]` Option and a Link Option cannot be expressed because `classify_option_subtype` returns a single subtype. Fix without a new action ID:
- `classify_option_subtype` returns a **set** of available play modes instead of one subtype.
- `play_option_from_hand` (reached via the existing `PLAY_HAND` action): when the mode set has more than one entry, install a mode-select pending selection ("Play as [Main] Option" vs "Plug in via Link"); each branch routes to its existing dispose path (`Standard` vs `Link`).
The mode choice surfaces as a normal `pending_selection`, so the no-approximations policy is satisfied — every legal play mode is exposed to the action space.
*Alternative considered:* a distinct "link from hand" action ID. Rejected — same retraining cost as D1's rejected alternative; a follow-up selection expresses the choice with no contract change.

### D3 — TDD, cards re-authored last
Both fixes land test-first. BT24-016 clause 1 (currently structural-only) and ST22-08's Link-Option mode (currently Standard-only) are re-authored once their substrate lands, and their `#[ignore]`/structural-only tests become behavioral.

## Risks / Trade-offs

- **[D1] `DIGIVOLVE` action ambiguity** → if a hand card has *both* a standard digivolve match and an activated-digivolve alt-path onto the *same* field permanent, one `DIGIVOLVE` action ID maps to two intents. Mitigation: the implementation must pick a rule — prefer the standard path, or (consistent with D2) install a small mode prompt. Decide during implementation with a card-shaped test.
- **[D1] `extra_cost` is entirely unimplemented — IMPLEMENTATION FINDING (2026-05-21)** → BT24-016's activated digivolve has an `extra_cost` (`select_trash` a Dimetromon, then `place_as_bottom_source`). The original D1 said to "mirror how other alt-paths with `extra_cost` resolve" — but a `grep` of the engine shows `extra_cost` appears at exactly 3 sites (`dna_digivolve.rs:238`, `:813`, `:831`), **all of them exclusions** (`!path.extra_cost.is_empty()` → skip the path). No alt-path execution anywhere runs `extra_cost`. So the activated-digivolve route is not a "routing tweak through existing digivolve execution"; it requires building an `extra_cost` runner from scratch — and BT24-016's `extra_cost` contains a **parking selection** (`select_trash`), so the runner must install a pending selection mid-action and resume into `place_as_bottom_source` and then the digivolve. This is a substantial new engine flow (a digivolve action that parks on a sub-selection before completing). The `DIGIVOLVE` action-ID reuse from D1 still holds; the execution-complexity estimate did not. Re-scope before implementing — see Open Questions Q3.
- **[D2] classify rework blast radius** → `classify_option_subtype` callers expect a single `OptionSubtype`. Returning a set touches every caller; audit `dispose_option` and `option_lifecycle.rs`. Keep single-mode cards behaviorally identical (a 1-element set).
- **[D2] IMPLEMENTATION FINDING (2026-05-22) — the mode-select must park `play_option_core` before cost-charging** → task-3.1 investigation: `play_option_core` (`game_actions.rs:984`) charges the play cost early (step 2, `printed_cost`), then runs `OnUseOption` → `OptionMain` → `dispose_option`. A dual-mode Plug-In costs **4 (Standard use cost)** vs **2 (Link cost)** — so the mode must be chosen *before* the cost is charged, and the entire remaining pipeline (cost → OnUseOption → OptionMain → dispose) forks on it. That means `play_option_core` must install a mode-select `pending_selection` near its start and resume the whole pipeline as a continuation — a parking refactor of the core option-play function (which today runs synchronously up to `dispose_option`). This is a genuine, sensitive engine refactor — larger than the spike's D2 estimate ("install a mode-select prompt"). Unlike D1, there is **no existing machinery to re-model onto** — dual-mode play has no precedent. Recommend gap 2 be implemented as a dedicated, careful pass rather than rushed; see Q4.
- **Deferring further** → if implementation uncovers that a reuse path genuinely cannot disambiguate, only then reconsider an action-space bump — but the spike found no such blocker.

## Open Questions

- **Q1** — D1 ambiguity: prefer-standard vs. mode-prompt. Resolve with a BT24-016-shaped fixture during implementation.
- **Q2** — Does any current card other than ST22-08 exercise the dual-mode Option path? If not, the D2 mode-prompt can be minimal (two fixed modes) rather than fully general.
- **Q3 (RESOLVED)** — the D1 `extra_cost` runner concern was sidestepped entirely: BT24-016 was re-modeled as a `main_from_hand` clause (D1-REVISED), using only existing machinery. Gap 1 is done.
- **Q4 (RESOLVED)** — gap 2's parking refactor of `play_option_core` landed as a dedicated focused pass. `play_option_core` took a `chosen_mode` parameter; for a dual-mode card it installs an `EffectChoice` mode-select **before** cost-charging and the callback re-enters with the chosen mode, so the whole pipeline (cost → OnUseOption → OptionMain → dispose) forks cleanly on the choice. `classify_option_subtype` → `classify_option_modes`; `OptionSubtype` moved to `selection.rs` and stored on `PendingOption.subtype`. The full engine suite is green and `ACTION_SPACE_SIZE` / `TENSOR_SIZE` are unchanged. Gap 2 is done.
