## 1. G-ACTIVATED-DIGIVOLVE-EXECUTION — investigation + failing test

- [x] 1.1 Investigated — **finding:** `digivolve_route_match` (`dna_digivolve.rs:234,237`) excludes both `ActivatedDigivolve` and every `extra_cost` path; `extra_cost` appears at exactly 3 engine sites, all exclusions — it is never executed anywhere. BT24-016's `extra_cost` contains a parking `select_trash`. The `DIGIVOLVE` action-ID reuse holds, but the execution needs a from-scratch parking `extra_cost` runner. Design.md D1 risk + Q3 updated. **Paused for re-scoping.**
- [x] 1.2 Wrote behavioral tests for clause 1 — `bt24_016_hand_main_activated_digivolve` + two condition-gate tests (no Owen, no Elizamon)

## 2. G-ACTIVATED-DIGIVOLVE-EXECUTION — implementation (re-model, no engine code)

- [x] 2.1 N/A — D1-REVISED: no `action/mask.rs` change. BT24-016 clause 1 is a `main_from_hand` triggered clause; the engine already masks a Hand `[Main]` action for any card kind whose `MainFromHand` condition passes
- [x] 2.2 N/A — no `action/decode.rs` change and no `ActivatedDigivolve` execution route. `activate_hand_main` runs the clause; `effect_initiated_digivolve` does the digivolve, `run_steps` handles the parking `select_*` cost steps
- [x] 2.3 N/A — no `DIGIVOLVE`-action ambiguity: the re-model uses the distinct Hand `[Main]` action, not the `DIGIVOLVE` range. BT24-016's standard `kind: digivolve` alt-path is unaffected
- [x] 2.4 Re-authored BT24-016 — removed the `kind: activated_digivolve` alt-path; clause 1 is now a `when: main_from_hand` clause (select Elizamon → select Dimetromon → place_as_bottom_source → effect_initiated_digivolve, cost 3, ignore_requirements)
- [x] 2.5 Verified green: `cards_behavioral -- bt24_016` — 24 passed, 0 failed, 0 ignored (3 new clause-1 behavioral tests)

## 3. G-LINK-OPTION-DUAL-PLAY-MODE — investigation + failing test

- [x] 3.1 Investigated — **finding:** `play_option_core` (`game_actions.rs:984`) charges the play cost early, before `OnUseOption`/`OptionMain`/`dispose_option`. A dual-mode Plug-In costs 4 (Standard) vs 2 (Link), so the mode-select must park `play_option_core` *before* cost-charging and the whole pipeline forks on the choice — a genuine parking refactor of the sensitive option-play core, with no existing machinery to re-model onto. Design.md D2 risk + Q4 updated. **Paused — see status below.**
- [x] 3.2 Behavioral tests authored in `st22_08.rs` §6 — `st22_08_dual_mode_installs_mode_select` (a dual-mode Option played from hand installs a 2-choice mode-select `EffectChoice`), plus Standard/Link cost + routing tests. DCGO `ST22_08.cs` confirmed the dual-mode shape (LinkCondition `linkCost: 2` + LinkAction vs MainEffect / SecurityEffect)

## 4. G-LINK-OPTION-DUAL-PLAY-MODE — implementation

- [x] 4.1 Replaced `classify_option_subtype` with `classify_option_modes` (`game_actions.rs`) → returns a `Vec<OptionPlayMode>` of available modes; `[Standard, Link]` for a dual-mode Plug-In, 1-element for every other Option (single-mode cards behaviorally identical). `OptionSubtype` moved to `selection.rs` (pub) + stored on `PendingOption.subtype` so `dispose_option` reads the resolved mode instead of re-classifying
- [x] 4.2 `play_option_core` re-signed with a `chosen_mode` param: when `option_legal_play_modes` returns >1 affordable mode it installs an `EffectChoice` mode-select (`install_option_mode_select`) and returns `Pending`; the callback re-enters with the chosen mode. Cost forks (Standard use cost vs flat Link cost), `OptionMain` firing is skipped for Link, dispose routes on `pending.subtype`. Mask (`action/mask.rs`) lights `PLAY_HAND` when any mode is affordable
- [x] 4.3 Re-authored `ST22-08.yaml` — added clause 4 `kind: link_requirement` (cost 2, `filter: { level_gte: 3 }`, `scope: inherited`) alongside the `[Main]` clause; rewrote `st22_08.rs` (34 behavioral tests, no `#[ignore]`) incl. §6 dual-mode coverage
- [x] 4.4 Verified green: `cards_behavioral -- st22_08` — 34 passed, 0 failed; `option_flow` — 93 passed, 0 failed

## 5. Wrap-up

- [x] 5.1 Full engine suite green: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader` — every test target `ok`, 0 failed (`cards_behavioral` 2722 passed / 129 ignored, `option_flow` 93, `mask_and_tensor` 157, `dsl` 635, `combat` 206, `replacements` 110, `selection` 73, …). `ACTION_SPACE_SIZE` / `TENSOR_SIZE` unchanged — the mode-select reuses the existing `EffectChoice` / `HAND_EFFECT_START` action range; no `action/space.rs` or `tensor.rs` change
- [x] 5.2 Added a "Medusamon PARTIAL-card unblock (Tier 3)" section to `qa/resolved-gaps.md`. `G-LINK-OPTION-DUAL-PLAY-MODE` struck through in `engine-gaps.md` (fully resolved). `G-ACTIVATED-DIGIVOLVE-EXECUTION` is **not** fully struck — its entry is updated with a "BT24-016 UNBLOCKED" status and the card struck through, but the entry stays open as a residual for the 3 out-of-scope `activated_digivolve` cards (BT22-013/026, BT16-027) per design.md D1-REVISED
- [x] 5.3 Updated `qa/qa-reports/validated_cards_dsl.json` (BT24-016, ST22-08 → `IMPLEMENTED`, `gap_kind: null`) and added a Tier-3 follow-up section to `qa/archetype-qa/dsl/medusamon.md` (archetype now 54/54 IMPLEMENTED, 0 PARTIAL)
