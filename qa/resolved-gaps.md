# Resolved Engine and DSL Gaps

Last updated: 2026-05-04

This file is the archive for reusable engine and DSL gap entries that have been resolved. Active gap trackers should keep only open gaps or partial slices with remaining implementation work:

- [qa/archetype-qa/engine-gaps.md](archetype-qa/engine-gaps.md)
- [qa/dsl-vocab-gaps.md](dsl-vocab-gaps.md)

When a reusable gap closes, move the full entry here and leave any card-specific migration/test cleanup in the active tracker only if there is still real follow-up work.
## Engine Gaps: Legacy Resolved List

1. ~~**Also Treated As (Name Aliasing)**~~ — RESOLVED 2026-03-14. Added `card.also_treated_as_names` list to `CardSource`, `card_names` property returns `[base] + aliases`. 41 scripts batch-updated.

2. ~~**Disable Effect**~~ — RESOLVED 2026-03-14. Added `ModifierType.DISABLE_EFFECT` check in `_collect_triggered_effects()` before effects fire.

3. ~~**Redirect Attack**~~ — RESOLVED 2026-03-14. Added `game.redirect_attack(new_target_perm)` in `combat.py`. Used by EX11-042, P-094, BT22-061.

4. ~~**Force Attack**~~ — RESOLVED (pre-existing). `ModifierType.FORCE_ATTACK` + action mask enforcement in `action_mask.py`.

5. ~~**DP Floor**~~ — RESOLVED 2026-03-14. Added `ModifierType.DP_FLOOR`; `permanent.dp` applies `max(floor, computed)`.

6. ~~**Security Effect Duplication**~~ — RESOLVED 2026-03-14. `is_security_effect` effects now only fire on `SecuritySkill`/`OnSecurityCheck`.

7. ~~**Suppress On Play**~~ — RESOLVED 2026-03-14. `_suppress_on_play` context flag skips `is_on_play` effects. Used by BT13-110.

8. ~~**Aura Keywords**~~ — RESOLVED 2026-03-14. `has_keyword()` now supports aura-style grants from other permanents (Blocker, Rush, Piercing, Alliance, etc.).

9. ~~**Hand-Activated Main Effects**~~ — RESOLVED 2026-03-12. `_is_hand_main` action type (actions 30-59). See engine-api-reference.md Pattern 13.

10. ~~**Trash Main Action Mask**~~ — RESOLVED 2026-03-14. Added `TRASH_MAIN_START = 1150` action range (1150 + trash_idx). Scripts use `_is_trash_main = True` flag. 5 scripts updated: BT20-096, BT24-076, EX10-054, EX10-011, EX7-060.

11. ~~**One-Shot Digivolve Cost Hook**~~ — RESOLVED 2026-03-14. `Player.digivolve()` now checks `CHANGE_DIGIVOLUTION_COST` modifiers from the registry. BT3-103 and EX1-071 use closure-based consumed flags for one-shot behavior.

12. ~~**End-of-Turn DNA Digivolve**~~ — RESOLVED 2026-03-14. No new engine API needed; existing `effect_dna_digivolve_from_hand()` called from inherited `OnEndTurn` effects on BT12-022 and BT12-050.

13. ~~**Grant Triggered Effect to Opponent's Permanent**~~ — RESOLVED 2026-03-14 (pre-existing). `permanent.grant_temp_effect(effect, expiry_turn)` API + `clear_expired_effects()` at turn start. BT14-044 fully implemented.

14. ~~**Effect-Based Play Lock**~~ — RESOLVED 2026-03-14. Added `ModifierType.CANNOT_PLAY_BY_EFFECT`; checked in `effect_play_from_zone()` only (normal hand plays unaffected). BT9-047 updated.

15. ~~**Aura-Style CANNOT_UNSUSPEND for New Entries**~~ — RESOLVED 2026-03-14 (pre-existing). BT12-057 uses aura-style modifier with condition `target is not owner_perm`; `unsuspend()` dynamically checks `has_modifier()` for all permanents including new entries.

16. ~~**OnDigivolutionCardReturnToDeckBottom Not Auto-Fired**~~ — RESOLVED 2026-03-14 (matches DCGO). DCGO also uses manual trigger. Scripts call `game.execute_effects()` explicitly. Functional pattern, not a gap.

17. ~~**Top/Bottom Deck Choice**~~ — RESOLVED 2026-03-14. Added `game.effect_choose_deck_placement(player, card, callback)` helper. BT23-057 updated to use it.

18. ~~**WhenRemoveField Lacks Removal Cause Context**~~ — RESOLVED 2026-03-14. `Player.delete_permanent()` now accepts `removal_cause` parameter ('battle', 'effect', 'rule', 'cost', 'de_digivolve'). Passed in WhenRemoveField/OnRemovedField/WhenPermanentWouldBeDeleted context. EX7-049 updated.

19. ~~**Face-Down Card Tracking**~~ — RESOLVED 2026-03-14 (matches DCGO). DCGO `IsFlipped` is Security-only. Approximation counting all non-top sources is acceptable.

20. ~~**Ignore Color Requirement**~~ — RESOLVED 2026-03-14. Added `ModifierType.IGNORE_COLOR_REQUIREMENT` for aura-style bypass in `action_mask.py`. 7 Hudiemon Option scripts use `card._match_color_requirement = False` for self-bypass. BT23-094 also updated. Rust note 2026-05-02: `ModifierType::IgnoreColorRequirement` is now honored by Option masks and decode/execution in `digimon-engine`, covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture`.

21. ~~**Security Play API**~~ — RESOLVED 2026-03-17. Added `game.effect_play_from_security(player, card)` helper. `security_attack()` now checks `card._security_played` flag before trashing. EX1-066 updated.

22. ~~**Dynamic Alt-Digi Cost**~~ — RESOLVED 2026-03-17. `digivolve_validator.py` now checks `_alt_digi_cost_fn` callable for dynamic cost calculation. BT24-101 Jupitermon uses security card count.

23. ~~**Digivolve from Hand or Trash**~~ — RESOLVED 2026-03-17 (pre-existing). `include_trash` param already exists on `effect_digivolve_from_hand()`.

24. ~~**Dynamic Security Attack Modifier**~~ — RESOLVED 2026-03-17. Wired `CHANGE_SECURITY_ATTACK` modifier registry into `permanent.security_attack_modifier()`. Fixed 6 scripts with wrong `value_fn` arity.

25. ~~**Optional Attack ("may attack")**~~ — RESOLVED 2026-03-17. Added `ModifierType.MAY_ATTACK` — enables but doesn't force attack (pass remains available). 4 scripts updated.

26. ~~**Digimon-Only Attack Target Restriction**~~ — RESOLVED 2026-03-17. Added `ModifierType.CANNOT_ATTACK_PLAYER` checked in `can_attack_player()`.

27. ~~**is_own_effect in WhenRemoveField Context**~~ — RESOLVED 2026-03-17. Added `is_own_effect`/`is_opponent_effect` to WhenRemoveField, OnRemovedField, WhenPermanentWouldBeDeleted contexts.

28. ~~**Conditional Color Requirement Bypass**~~ — RESOLVED 2026-03-17. Added `_match_color_requirement_fn` callable support to `CardSource.match_color_requirement` property. 4 scripts updated.

29. ~~**Deletion Observer Recursion Guard**~~ — RESOLVED 2026-03-17. Added depth limit (8) to `execute_deletion_effects()` to prevent RecursionError from token chain loops (Puppets vs TS Olympos).

---

## Engine Gap: ~~Inherited Aura Keyword Grants~~ — RESOLVED 2026-04-12

- **Discovered in:** BT11-042 Angewomon fix-card review (2026-04-12)
- **Card(s):** BT11-042 Angewomon (Blocker aura), BT20-019 Jesmon (X Antibody) (Piercing aura while Jesmon GX), plus other cards using `is_inherited_effect + _applies_to_all_own_digimon + keyword`.
- **What was broken:** `permanent.has_keyword()` aura scan only checked `_applies_to_all_own_digimon` effects on other perms' **non-inherited top-card** effects. Inherited aura keyword effects (below-the-line on a card that is either the current top card or a digivolution source below another Digimon) were silently ignored — while the equivalent DP aura path (`_get_aura_dp_modifier`) already scanned inherited effects in other perms' `card_sources[:-1]`.
- **Resolution:** Extended `has_keyword()` in `permanent.py` to (a) scan inherited aura effects from other perms' `card_sources[:-1]` (mirroring `_get_aura_dp_modifier`); (b) scan ALL aura effects (inherited and non-inherited) on other perms' top cards; (c) scan the self permanent's top card for inherited aura effects targeting self (so BT11-042 Angewomon's aura applies to herself via the `_keyword_permanent_condition` filter). No new script APIs required — existing `_applies_to_all_own_digimon` + `_keyword_permanent_condition` pattern now works for inherited auras as documented.

---

## Engine Gap: Digivolve from Hand or Trash — RESOLVED 2026-03-17 (pre-existing)

- **Resolution:** `effect_digivolve_from_hand()` already has `include_trash` parameter (effects.py:454). No engine change needed.

---

## Engine Gap: ~~Puppet-Scoped Overclock Sacrifice Filter [G-OVERCLOCK-TRAIT-FILTER]~~ — RESOLVED 2026-05-02

- **Discovered in:** Puppets/Nyabootmon assessment (2026-04-28)
- **Scope:** Rust engine action mask and Overclock activation.
- **Card(s):** BT22-042 Nyabootmon, EX7-027 Chaperomon, EX7-030 Cendrillmon, EX11-024 Cendrillmon, BT22-036 Kazuchimon, plus other cards with `<Overclock ([Puppet] Trait)>`.
- **Effect text:** "<Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)"
- **Resolution:** Overclock cost candidates are now parameterized. The end-of-turn activation bit only appears when at least one legal token or predicate-matching Digimon can pay the cost, and the pending selection stores only those legal action IDs. The generic mask exposes only stored candidates plus `PASS`, and decode rejects non-candidate targets without deleting a permanent or starting an attack.
- **Covered by:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_overclock overclock --nocapture`
- **DSL coverage:** `grant_keyword` accepts `overclock_cost_filter`, compiled as a predicate and lowered onto the same runtime Overclock cost filter path.

---

## Engine Gap: ~~Familiar Token On Deletion Effect Missing [G-FAMILIAR-TOKEN-ON-DELETION]~~ — RESOLVED 2026-05-02

- **Discovered in:** Puppets/Nyabootmon assessment (2026-04-28)
- **Scope:** Rust token card effects.
- **Card(s):** TOKEN_FAMILIAR; generated by P-165 ShoeShoemon, EX7-030 Cendrillmon, EX11-024 Cendrillmon, ST19-12 Cendrillmon, and related Puppet effects.
- **Effect text:** "Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn."
- **Resolution:** `src/cards/tokens/familiar.rs` now implements the mandatory `OnDeletion` effect using the opponent-permanent selection path and applies -3000 DP until end of turn. Token card-data construction is also covered by an all-registered-token invariant so future token definitions must synthesize complete `CardData`.
- **Covered by:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- token`, `cargo test --manifest-path code/digimon-engine/Cargo.toml token_registry`, and the full `cargo test --manifest-path code/digimon-engine/Cargo.toml`.
- **Remaining limits:** This closes the Familiar token deletion text only. Event-gated Delay windows remain a separate Puppet blocker.

---

## Engine Gap: Cost and Replacement Framework

Resolved by Group 3:
- BT13-007 King Drasil_7D6 and ST21-13 Matt Ishida & T.K. Takaishi can both reduce AD1-025 Omnimon before memory is paid because AD1-025 has both `[Royal Knight]` and `[ADVENTURE]`.
- Triggered effect costs may install pending selections and resume process only after cost payment.
- Optional cost decline skips process without hidden auto-selection.
- Replacement predicates can inspect cause, source controller, and subject controller.
- Partition source requirements are enforced before prevention.
- Delay options can pay themselves as replacement costs and prevent deletion.
- Effects can end a pending attack after a printed cost resolves.

---

## Engine Gap: ~~When Attacking Selection Phase Override~~ — RESOLVED 2026-04-02

- **Discovered in:** BT24-024 Submarimon fix-card review
- **Card(s):** All cards with [When Attacking] effects that use selection-based APIs (effect_play_from_zone, effect_select_opponent_permanent, etc.)
- **What was broken:** `declare_attack()` in combat.py continued to counter/block/security after `execute_effects(OnUseAttack)` without checking if effects created a pending selection. The selection phase was overwritten by counter timing.
- **Resolution:** Added park-and-resume pattern: `declare_attack` checks for `pending_selection` after WA effects fire and returns early; `_decode_selection` calls `_maybe_resume_combat_after_wa_selection()` to continue the attack flow after all selections resolve.

---

## Engine Gap: ~~Dynamic Security Attack Modifier~~ — RESOLVED 2026-03-17

- **Resolution:** Wired `ModifierType.CHANGE_SECURITY_ATTACK` into `permanent.security_attack_modifier()` via registry query. BT10-112 uses `_DynamicSAEffect` subclass with `@property` for computed count. Fixed 6 scripts with wrong `value_fn` arity (`lambda: -1` → `lambda cur, t, c: cur - 1`): BT10-042, BT15-084, BT23-094, BT24-071, EX6-022.

---

## Engine Gap: ~~Optional Attack ("may attack")~~ — RESOLVED 2026-03-17, EXTENDED 2026-04-04

- **Resolution:** Added `ModifierType.MAY_ATTACK` semantic marker. Unlike `FORCE_ATTACK`, `MAY_ATTACK` does NOT trigger the forced attackers block in `action_mask.py` — pass (action 62) remains available. Scripts grant Rush + unsuspend alongside `MAY_ATTACK`. Updated 4 scripts: BT24-085, BT24-037, BT24-082, BT24-051.
- **Extension (2026-04-04):** MAY_ATTACK now works at end of turn: `_has_end_of_turn_keywords()` checks MAY_ATTACK, `EndOfTurnAction` action mask offers attack actions (Digimon + player), `_decode_end_of_turn_action` handles SECURITY_TARGET. Also added deferred end-phase completion (`_end_phase_deferred`) for OnEndTurn effects that create pending selections.

---

## Engine Gap: ~~Digimon-Only Attack Target Restriction~~ — RESOLVED 2026-03-17

- **Resolution:** Added `ModifierType.CANNOT_ATTACK_PLAYER` checked in `permanent.can_attack_player()` via modifier registry. BT24-051 Merukimon registers it in the "attack your opponent's Digimon" callback.

---

## Engine Gap: ~~is_own_effect in WhenRemoveField Context~~ — RESOLVED 2026-03-17

- **Resolution:** Added `is_own_effect` and `is_opponent_effect` booleans to `WhenPermanentWouldBeDeleted`, `WhenRemoveField`, and `OnRemovedField` timing contexts in `player.py`. Derived from existing `is_opponent_effect` parameter on `delete_permanent()`. BT24-037 Silphymon updated to use clean `is_own_effect` check instead of `removal_cause` heuristic. BT20-059 Gankoomon was already properly implemented (not affected).

---

## Engine Gap: ~~Conditional Color Requirement Bypass~~ — RESOLVED 2026-03-17

- **Resolution:** Added `_match_color_requirement_fn` callable support to `CardSource.match_color_requirement` property. Dynamic fn is checked first, falls through to static `_match_color_requirement`. Updated 4 scripts: BT24-091 (TS trait check), BT22-099 (CS trait check), ST20-15 (face-up IoA check), BT10-110 (Royal Knight check).

---

## Engine Gap: ~~DigiXros~~ — RESOLVED 2026-03-15

- **Card(s):** 60 cards across BT10-BT24, EX3-EX10, P sets
- **Resolution:** Engine natively supports DigiXros/Assembly: `DigiXrosCost` data model, `parse_digixros_req()` parser (all 60 cards), `digixros_validator.py` for material matching, play intercept → `SelectMaterial` loop → `_execute_digixros_play()`, field materials fire `WhenRemoveField` with `removal_cause='digixros'`, `digixros_count` in `OnEnterFieldAnyone` context.

---

## Engine Gap: Digivolution-Stack Inherited Triggered-Effect Dispatch (Rust Engine)  [G-INHERITED-DISPATCH]

- **Discovered in:** Medusamon archetype, BT21-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine (`code/digimon-engine/`) only. Python engine resolves inherited effects via `card_sources[:-1]` scanning in `_collect_triggered_effects`.
- **Card(s):** BT21-008 Elizamon — inherited `[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, gain 1 memory.` Almost all Lv3+ Digimon in this archetype have a similar inherited triggered effect; this gap blocks the inherited half of every one of them.
- **Effect text:** any DSL clause with `scope: inherited` + a triggered timing.
- **Status:** Fixed for battle-area permanent dispatch on 2026-04-29. `enqueue_from_permanent` now preserves top-card / linked / Training dispatch, then scans below-top `card_sources` and queues only matching `effect.inherited == true` effects with `source_permanent` set to the carrier and `source_card` set to the inherited source card.
- **Affected cards:** YAML cards with `scope: inherited` and a triggered timing can now fire from below the top card when the relevant event is already dispatched to the carrier permanent's battle-area observer path.
- **Regression coverage:** `bt21_008_inherited_positive_fires_when_source_under_carrier_your_turn` and `buried_non_inherited_triggered_effect_does_not_fire_from_source_position`.
- **Remaining limits:** This does not add every event fire site. Group 4 added source-trash context for direct `EffectContext::trash_card_source` / `trash_top_source` helpers and effect-driven security-removal fan-out/resume for direct security stack moves. Lower-source trash from some older zone-return paths and breeding-area dispatch remain separate follow-ups. Group 2 closed the shared source, DP-budget, breeding-permanent, and empty-tail selection primitives on 2026-04-29.
- **Updated 2026-05-02 (Group 5 Task 6):** Training sideways inheritance now records `trained: Option<TrainingBinding>` on the Training permanent. Binding is by specific Training permanent handle and validates the carrier's physical top card source before enqueue and queued-effect resolution, closing the over-broad owner-wide Training fan-out slice for bound Training effects without aliasing stale field indices or duplicate Training copies. Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- training_sideways_effect_applies_only_to_its_intended_trained_permanent training_bound_to_removed_permanent_does_not_apply_to_reused_index duplicate_training_copies_bind_to_distinct_carriers_by_permanent_handle queued_training_effect_revalidates_bound_carrier_before_resolution`.

---

## Engine Gap: `max_per_turn` (Once-Per-Turn) Not Enforced for Triggered Effects  [G-OPT-TRIGGERED]

- **Discovered in:** Medusamon archetype, EX11-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-008 Elizamon (inherited OPT clause); applies to every DSL card with `once_per_turn: true` on a triggered clause.
- **Effect text:** any clause that combines `[Once Per Turn]` with a non-Main triggered timing (`OnLoseSecurity`, `WhenAttacking`, `OnPlay`, `OnDigivolving`, etc.).
- **Status:** Fixed for permanent-backed queued triggered effects on 2026-04-29. `run_queued_effect_inner` now checks `Permanent::activation_count(source_card, slot) >= effect.max_per_turn` before processing and records activation before `process`, matching the existing activated field-main timing.
- **Regression coverage:** `bt21_008_inherited_opt_blocks_second_trigger_same_turn`.
- **Remaining limits:** This only enforces the existing queued-effect activation counter. It does not add optional prompt/action-space handling or breeding dispatch. Group 4 separately covered direct source-trash helper context and owner routing.

---

## Engine Gap: `EffectTiming::OnMove` for Breeding-to-Battle Movement  [G-ON-MOVE]

- **Discovered in:** Medusamon archetype, EX11-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine + DSL (hybrid; see `qa/dsl-vocab-gaps.md` for DSL half).
- **Card(s):** EX11-008 Elizamon — `[When Moving] [On Play]` shared body; BT16-082 Ukkomon — entire [Your Turn][OPT] triggered effect (observer in battle area watches any own Digimon move from breeding). Other archetypes will surface this for cards with "[When one of your Digimon moves from the breeding area]" observer triggers.
- **Effect text (EX11-008):** "[When Moving] 1 of your Digimon ... gains <Raid> and +3000 DP for the turn."
- **Effect text (BT16-082):** "[Your Turn][Once Per Turn] When one of your Digimon moves from the breeding area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon card or Tamer card among them to the hand. Return the rest to the bottom of the deck. Then, you may hatch in your breeding area."
- **Status:** Fixed for breeding-to-battle movement on 2026-04-29. `EffectTiming::OnMove`, `Effect::on_move(card)`, DSL `when: on_move`, and `TriggerSource::MovedFromBreeding { player, permanent, card }` now carry the moved battle-area permanent and top/source card after `Game::move_from_breeding` commits. Regression coverage: `on_move_fires_after_breeding_permanent_moves_to_battle`; direct DSL event-context coverage: `on_move_event_target_trait_predicate_matches_moved_permanent` proves `event_target_trait_has` sees the moved permanent/card.
- **Remaining limits:** This does not add unrelated `OnPlay`/`WhenDigivolving` body work for multi-timing cards. BT16-082's reveal/add-to-hand/remainder-bottom/optional-hatch body is implemented as native DSL as of 2026-05-04, backed by a reusable `can_hatch` predicate for the hatch prompt gate.

---

## Engine Gap: `dp_lte` Predicate Compiled but Not Evaluated in `eval_card_fields`  [G-DP-LTE-PREDICATE]

- **Discovered in:** Medusamon archetype, BT21-015 Cyclonemon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-015 Cyclonemon — `[On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 4000 DP or less.`
- **Effect text:** any DSL clause whose `select_*` filter uses `dp_lte: N` to constrain valid targets.
- **What's missing:** `dp_lte` parses and lowers to a `CompiledPredicate` variant, but `eval_card_fields` in `predicate.rs` does not evaluate it — the predicate evaluates as ALWAYS-TRUE for any target's `card_fields`. This means the 4000 DP cap is not enforced at selection time; ineligible targets appear in `valid_action_ids`. Two BT21-015 tests are `#[ignore]`'d pending the fix (`bt21_015_on_play_no_selection_when_no_eligible_target` and `bt21_015_on_play_filters_ineligible_targets_correctly`); boundary-inclusion at exactly 4000 is still asserted via the eligible-target tests that DO pass.
- **Suggested change:** add a `dp_lte` (and presumably `dp_gte`, `dp_eq`) match arm in `eval_card_fields` that reads the target's printed DP from card metadata (or the live effective DP — whichever the predicate semantics intend) and applies the comparison.
- **Workaround:** None — BLOCKED for negative-case tests; positive-case tests still pass because the engine over-permissively accepts eligible targets.

---

## Engine Gap: `event_target_owner` Predicate Missing  [G-EVENT-TARGET-OWNER]

- **Discovered in:** Medusamon archetype, BT24-018 Styracomon (and BT21-029 Medusamon clause-d-deletion-arm) DSL implementations (2026-04-27)
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** BT24-018 Styracomon — replacement clause "When any of your [Reptile] or [Dragonkin] would leave the battle area"; BT21-029 Medusamon — deletion-arm of the All-Turns token-spawn trigger.
- **Status:** RESOLVED for reusable trigger event context and generic replacement context.
- **Trigger coverage:** `event_target_owner` is available in DSL predicates and now reads `OnAnyDeletion` event context carrying the deleted permanent/card. BT21-029's deletion arm uses `event_target_owner: opponent` + `event_target_kind: digimon` and passes behaviorally.
- **Replacement coverage:** Replacement clauses can opt into cross-permanent subjects with `replacement_subject_is_mine` and combine it with `replacement_source_is_opponent` / `replacement_cause`; `lower_replacement.rs` no longer drops the non-source subject during process execution.
- **Regression coverage:** BT21-029 deletion-arm focused tests plus replacement-context focused suites: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement_context --nocapture`.

---

## Engine Gap: `dp_lte` / `dp_gte` Permanent Predicates  [G-PRED-DP-LTE / G-PREDICATE-DP-FILTER / G-SELECT-OPP-FILTER — same root cause]

- **Discovered in:** Medusamon archetype, BT21-015 Cyclonemon (Batch 2) + BT24-017 Medusamon + BT21-029 Medusamon (Batch 3) (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-015 (delete ≤4000 DP), BT24-017 (lowest DP), BT21-029 (lowest DP), and most cards with DP-bounded delete/select.
- **Status:** RESOLVED for reusable permanent predicate evaluation by Group 7 (2026-05-02). `dp_lte` / `dp_gte` now evaluate against live/effective permanent DP in the shared predicate path; BT8-097's high-DP and exact-6000 boundary tests are active and passing.
- **Remaining migration work:** older card-level tests and QA notes that were ignored under this gap may still need to be unignored or retagged after each card is revisited. Aggregate extrema shapes such as "lowest DP" / "highest DP" remain separate aggregate-selection/filtering work when they require more than a direct comparator.
- **Note:** Three card-discovery names (G-PRED-DP-LTE, G-PREDICATE-DP-FILTER, G-SELECT-OPP-FILTER, G-DP-LTE-PREDICATE) all refer to this same root cause; consolidating under G-PRED-DP-LTE.

---

## Engine Gap: Resolved during Medusamon run

- **`PlayFromSecurity` dispatch in security-skill timing** — RESOLVED 2026-04-27 in BT21-015 implementation. `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` now dispatches to `play_pending_security()` when `ctx.game.pending_security.is_some()` (security-skill replay path) and `play_from_security(player)` otherwise. Affects every DSL card with a `[Security]` clause that uses `play_from_security: {}` (BT21-015, BT5-093, BT9-092, BT22-084, ...).
- **Declarative inherited `grant_keyword` not visible to `has_keyword`** — RESOLVED 2026-04-27 in BT24-011 implementation. `code/digimon-engine/src/game.rs::has_keyword` now scans `perm.card_sources` for `declarative && inherited` effects whose name matches the `Grant <Keyword>` convention set by `lower_grant_keyword`. Companion change in `code/digimon-engine/src/debug_runner.rs::card_data_from_compiled` populates `CardData.keywords` from FaceUp `GrantKeyword` clauses so own-printed keywords surface without dispatch.
- **`Progress` keyword not in `lookup_keyword`** — RESOLVED 2026-04-27 in BT21-029 / EX11-012 implementations. `src/dsl_cards/modifier_map.rs` now maps `"Progress" => Keyword::Progress`.
- **`SelectOwnPermanent` / `SelectOpponentPermanent` ignored predicate filters (accept-all)** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/step/selections.rs::install_select_*_permanent` now pre-filters candidates with `eval_predicate` and threads the filter closure to the underlying `select_*_permanent` API.
- **Replacement clause subject-guard missing (would-leave fires for any permanent)** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/lower_replacement.rs` now checks `subject_matches` so `WhenWouldLeaveBattleArea` only fires when the carrier itself is the leaving permanent.
- **Generic cross-permanent replacement authoring** — RESOLVED 2026-05-03 for explicit replacement-context predicates. `src/dsl_cards/lower_replacement.rs` keeps self-scoped replacement clauses self-only by default, but clauses with `replacement_subject_is_mine` may now protect a different matching subject from the source permanent. Covered by `cross_permanent::dsl_source_permanent_can_protect_a_different_subject` and `replacement_context::replacement_subject_and_source_predicates_compile_together`.
- **`CompiledCardKind::Token` missing from predicate match** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/predicate.rs` now handles `CompiledCardKind::Token`, enabling `kind: token` filters (e.g. "delete 1 Token" cost).
- **Petrification token name case-sensitivity bug** — RESOLVED 2026-04-27 in BT21-029 implementation. `TokenRegistry` is keyed lowercase; YAML `token_name:` values must use lowercase (`petrification` not `Petrification`). EX11-012's example version had the same bug; production version corrected.
- **Source-scoped return/de-digivolve immunity** — RESOLVED 2026-05-02 for covered Rust consumers. EX8-070, P-215, and BT18-064 can now use narrow `CannotBeReturnedToHand`, `CannotBeReturnedToDeck`, and `CannotBeDeDigivolved` passive replacements scoped to opponent effects through the production `EffectContext` movement helpers and `EffectContext::grant_zone_return_immunity_to_opponent_effects`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture`.

---

## Engine Gap: ~~`count_lte` / `count_gte` Aggregate Predicate (Non-Security) Not Evaluated  [G-COUNT-LTE-EVAL / G-COUNT-GTE-EVAL]~~ — RESOLVED 2026-05-03

- **Discovered in:** Medusamon archetype, BT21-017 Dimetromon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-017 Dimetromon — "[When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Owen Dreadnought] from your hand without paying the cost." Also BT22-084 Nokia Shiramine (Start of Your Main Phase condition) and any card whose clause-level `condition:` uses `count_lte` with a non-security zone filter.
- **Effect text:** "If you have 1 or fewer Tamers" — a `count_lte` aggregate gate on the controller's battle area.
- **Status:** RESOLVED for `count_lte` / `count_gte`. `eval_predicate_with_bindings` now counts matching subjects across authored zones and owner scopes before comparing to `n`.
- **Affected cards:** BT21-017 (tamer ≤ 1 gate on WhenDigivolving), BT22-084 Nokia Shiramine (count_lte gate on StartOfMainPhase), and any other card with a non-security zone count condition.
- **Passing command(s):** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_017 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_006 --nocapture`.

---

## Engine Gap: ~~`EffectTiming::Declarative` Never Fired — Filtered Aura / Grant-Keyword Runtime Gap  [G-DECLARATIVE-KEYWORD]~~ — RESOLVED 2026-05-02

- **Discovered in:** Medusamon archetype — BT21-029, EX11-012, EX11-054 (grant_keyword), BT5-008 (filtered aura), 2026-04-27
- **Scope:** Rust engine.
- **Card(s):** Any card using `kind: aura` with a non-empty target predicate (filtered aura), or `kind: grant_keyword` with a declarative scope. Specific cards: BT21-029 Medusamon, EX11-012 Medusamon (Progress keyword), BT5-008 Gaossmon (filtered aura +3000 DP to other Gaossmon).
- **Effect text:** "[Your Turn] Your other [Gaossmon] all get +3000 DP." (BT5-008); "[When Digivolving / On Field] <Progress>" (EX11-012 inherited); "SecurityAttack+1" (BT21-029 clause a).
- **Status:** RESOLVED by Group 6 for battle-area filtered aura/player-modifier runtime dispatch. `Game::tick_declarative_effects` runs process-backed declarative effects from face-up field sources, filtered `kind: aura` clauses materialize matching permanent modifiers, `other: true` excludes the source permanent, player-scoped aura clauses can install player modifiers, and tick refresh removes stale materialized modifiers without duplicating entries.
- **Passing command(s):** `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture`; broad confirmation `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- --nocapture`.
- **Remaining related blocker:** security-zone aura sources and the future query-time aura model remain separate work; no remaining blocker for battle-area tick-dispatched filtered auras covered by Group 6.
- **What was missing:** `EffectTiming::Declarative` is defined in `enums.rs` (line 204) and used by `Effect::declarative(card)` in `effect.rs`, but it was not enqueued or fired by the runtime. As a result:
  - Filtered aura `process` closures (which call `ctx.add_dp_modifier`, `ctx.grant_keyword`) are never invoked.
  - Declarative `grant_keyword` modifiers are never installed in `ModifierRegistry` at runtime.
  - The `active_when` condition closure on declarative effects is also never evaluated.
  - The only working declarative path is the **self-aura** (empty predicate), which uses `Effect.dp_modifier` static field read by `source_dp_contribution` — a completely different execution path that doesn't require the process closure.
- **Scope of impact:** All DSL `kind: aura` clauses with a non-empty `target:` predicate, all `kind: grant_keyword` declarative clauses, and all `kind: flood_gate` clauses that target permanents (flood_gate uses the same declarative process path).
- **Suggested change:** Implement a declarative-effects tick loop. Two approaches:
  1. **On-placement tick**: when `play_from_hand` / `fire_on_play` installs a new permanent, iterate all existing permanents and call each declarative process closure via `enqueue_triggered(EffectTiming::Declarative, TriggerSource::PlayerBattleArea(p))` or a dedicated `fire_all_declarative_effects()` scan. The `Expiry::Permanent` flag on declarative modifiers + `ModifierRegistry` deduplication ensure re-firing is safe.
  2. **Per-query computation**: evaluate the declarative effect condition+process inline at query time (e.g., inside `game.modifiers.sum(h, ChangeDp)`) rather than pre-installing modifiers. This matches the "continuous re-evaluation" model used by physical TCGs where static effects are recomputed on every state change.
  The simplest v1 fix is to call a new `Game::run_declarative_effects()` method at the end of `fire_on_play`, `end_turn`, and `begin_turn` — these are the natural state-change boundaries where auras need to be recomputed.
- **Workaround:** Structural tests pass (compile-time verification only). All behavioral tests for filtered aura and grant_keyword modifier installation are `#[ignore]`'d with this gap tag.

---

## Engine Gap: Multi-Select of Opponent Battle-Area Permanents with Running DP-Sum Cap  [G-MULTI-SELECT-OPP-DP-SUM]

- **Discovered in:** Medusamon Batch 10, LM-021 Agumon - Bond of Bravery DSL implementation (2026-04-28)
- **Scope:** Rust engine + DSL.
- **Card(s):** LM-021 Agumon - Bond of Bravery — "[On Play][When Digivolving] Delete any number of your opponent's Digimon whose total DP adds up to equal or less than this Digimon's DP." Also BT17-018 Gallantmon Crimson Mode — "[On Play][When Digivolving] Delete any number of your opponent's Digimon with total DP equal to or less than this Digimon's DP." Both cards share the same selection mechanic.
- **Effect text (LM-021):** "Delete any number of your opponent's Digimon whose total DP adds up to equal or less than this Digimon's DP."
- **What's missing:** `EffectContext` exposes only single-target selection (`select_opponent_permanent`) and count-capped multi-target selection (`select_count_capped_multi`, which caps by pick count, not by DP sum). There is no primitive for iterative multi-select where each pick reduces a remaining DP budget and the player may stop at any point once they have at least one selection (DCGO: `canEndNotMax: true`, `canTargetConditionByPreSelectedList` with dynamic remainder). The running DP-sum cap requires: (a) tracking cumulative DP of already-selected targets, (b) re-filtering valid candidates after each pick to exclude those whose DP would exceed the remaining budget, and (c) allowing early termination once at least one target is picked. None of these are available in the current selection state machine.
- **Suggested change:** Add a `select_opponent_permanent_dp_sum(description, self_dp, callback)` method to `EffectContext` that: (1) initializes a `remaining_budget = self_dp`; (2) presents a filtered pick from `opponent.battle_area` where `perm.dp <= remaining_budget`; (3) after each pick, subtracts the picked card's DP from `remaining_budget` and repeats if budget > 0 and valid candidates remain; (4) allows the player to stop picking at any point; (5) calls `callback` once on all selected handles. Alternatively, extend `PendingSelection` with a `DpBudget(u32)` variant that the selection engine drains per-pick.
- **Workaround:** `raw_rust: { fn: lm_021_delete_dp_sum }` and `raw_rust: { fn: bt17_018_delete_opp_digimon_dp_budget }` — both fall back to single-pick with a DP <= budget filter. Full multi-pick semantics are deferred until this gap closes.
- **Updated 2026-04-29:** Resolved for opponent battle-area DP-budget selection. `EffectContext::select_opponent_permanents_by_dp_budget` installs `SelectionKind::DpBudget`, filters remaining affordable targets after each pick, and exposes PASS after `min_picks`. DSL `select_opponent_dp_budget` binds the chosen permanents and `delete_bound_permanents` consumes them. Covered by `dp_budget_selection_tracks_remaining_dp_and_allows_pass_after_min`, `dp_budget_selection_finishes_when_no_targets_fit`, `dp_budget_selection_mask_exposes_only_remaining_affordable_targets`, and `dsl_select_dp_budget_deletes_bound_permanents`.

---

## Engine Gap: Cross-Permanent Rocks Source Selection Resolved  [G-ROCKS-SOURCE-SELECTION-DSL]

- **Discovered in:** Rocks / RockClose archetype assessment (2026-04-29 follow-up).
- **Scope:** Rust engine + DSL.
- **Card(s):** EX10-032 Proganomon, EX10-028 Landramon, EX8-070 Zofr Kabus, EX10-036 Magneticdramon, EX10-033 / EX11-044 / EX8-055 Pyramidimon source-trash bodies.
- **Status:** Resolved for the shared selection primitive. `EffectContext::select_own_sources` supports exact-N and up-to-N source choices across own battle-area stacks, binds stable `SourceSelectionRef` values, and DSL `trash_selected_sources` consumes those refs without a fake permanent prompt.
- **Regression coverage:** `source_multi::exact_two_sources_can_be_selected_across_own_battle_area`, `source_multi::up_to_sources_enables_pass_only_after_minimum_is_met`, `source_multi_mask_only_exposes_selecting_players_pending_actions`, `select_own_sources_binds_source_refs_for_trashing`, and `empty_select_own_sources_runs_outer_tail_synchronously`.
- **Remaining limits:** Triggered-body cost ordering, Fragment / Digi-Burst / replacement integration, and card-specific Rocks bodies remain separate gaps.

---

## Engine Gap: Effect Re-Firing / Cross-Timing Self-Trigger

- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Dark Masters (2026-04-18); Puppets/Nyabootmon assessment (2026-04-28).
- **Scope:** Rust engine effect context and DSL lowering.
- **Card(s):** EX8-074 MedievalGallantmon; BT22-042 Nyabootmon for the permanent-sourced `[When Digivolving]` self-refire slice.
- **Status:** Resolved for constrained permanent-sourced `WhenDigivolving` re-firing. `EffectContext::refire_effect_from_permanent(source, "when_digivolving")` enumerates refireable effects, queues the selected effect slot through the normal `QueuedEffect` path, preserves `source_card` / `source_permanent` identity, and reuses existing once-per-turn accounting. DSL authors can use `refire_effect: { source: <binding>, timing: when_digivolving, optional: true|false }`.
- **Regression coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_refiring --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- effect_refiring --nocapture`.
- **Remaining limits:** Foreign-card `[On Play]` re-firing and Puppet deleted-object gating for "your other Digimon are deleted" remain separate open/partial gaps; this entry only archives the reusable permanent-sourced WhenDigivolving refire primitive.

---

## Engine Gap: Exact Trashed-Source Inherited Dispatch  [G-ROCKS-TRASHED-SOURCE-INHERITED-DISPATCH]

- **Discovered in:** Rocks pool implementation pass (2026-05-04).
- **Scope:** Rust engine effect queue.
- **Card(s):** EX8-051 Proganomon, EX8-047 Sunarizamon, EX8-005 Tumblemon, EX10-025 Sunarizamon, EX10-028 Landramon, EX10-032 Proganomon, BT21-055 Sunarizamon, EX11-038 Sunarizamon, and other Rocks inherited effects that trigger when the source card itself is trashed from a [Mineral]/[Rock] stack.
- **Status:** Resolved for source-trash events that already emit `TriggerSource::SourceTrashedFromStack`. The queue now enqueues inherited effects from the exact source card that was just trashed, even though that card is no longer live under its former host, and relies on trigger context predicates such as `host_permanent_trait_has` / `trashed_source_trait_has` for the former host and source-card facts.
- **Regression coverage:** Rocks behavioral tests including `ex8_051` exercise an inherited effect firing from the exact trashed source card. The source-trash event-context regression remains covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context --nocapture`.
- **Remaining limits:** This does not make every source-trash producer emit the event. Fragment, Digi-Burst, replacement costs, de-digivolve, and older source movement paths still need their own producer coverage before cards depending on those paths can be marked complete.

---

## Engine Gap: ~~`IgnoreColorRequirement` Modifier Not Enforced in Rust Option Action Mask~~ — RESOLVED 2026-05-02 [G-IGNORE-COLOR-MASK]

- **Discovered in:** Medusamon Batch 11, ST22-08 Offensive Plug-In V DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** ST22-08 Offensive Plug-In V — "While you have a Tamer, you can ignore this card's color requirements." Also any card that would use the `IgnoreColorRequirement` modifier via a flood_gate clause.
- **Effect text:** "While you have a Tamer, you can ignore this card's color requirements."
- **Resolution:** `code/digimon-engine/src/action/mask.rs` now treats player-scoped `ModifierType::IgnoreColorRequirement` as satisfying Option use legality before board color checks. The same helper is used by `play_option_from_hand`, so decode/execution rejects or accepts the same actions as the mask.
- **Regression coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture`.
- **Remaining limits:** This closes the Rust engine mask/decode hook only. DSL-specific conditional aura expression, live condition refresh, and card-specific flood_gate authoring remain separate DSL/card implementation work where applicable.

---

## Engine Gap: `DelayTrigger::StartOfYourNextTurn` Missing — Delay Fires at Start of Turn  [G-DELAY-START-OF-TURN]

- **Discovered in:** Medusamon Batch 12, LM-027 Red Scramble DSL implementation (2026-04-28)
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** LM-027 Red Scramble; LM-030 Green Scramble in BG Imperial — "[Start of Your Turn] If your opponent has a Digimon, ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)" Likely affects other Delay option cards whose activation timing is the controller's next turn START rather than END.
- **Effect text:** "[Start of Your Turn] … ＜Delay＞ …"
- **Status:** Resolved 2026-05-02. `DelayTrigger::StartOfYourNextTurn` is stored on delayed Option permanents, drains from `begin_turn` after `StartOfYourTurn` observers and before per-turn reset/draw/main progression, and DSL `CompiledTiming::StartOfYourTurn` lowers to the start-delay trigger.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- start_of_your_next_turn_delay_fires_at_turn_start_not_end`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay_start_of_your_turn_maps_to_start_of_your_next_turn`.
- **Group 5 handoff verification:** The full Delay/Link regression set passes: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed`.
- **Remaining related work:** Card-specific Scramble bodies may still be blocked by other non-timing gaps such as pending-security self-to-hand, hand-or-trash/free-play selection, color requirement bypass, and card-specific effect bodies. Existing ignored LM-027 card-level tests that still name `G-DELAY-START-OF-TURN` are stale migration placeholders, not evidence that the generic start-of-turn Delay primitive is open.

---

## Engine Gap: `EffectContext::add_pending_security_to_hand`  [G-ADD-OPTION-SELF-TO-HAND]

- **Discovered in:** Medusamon Batch 12, LM-027 Red Scramble DSL implementation (2026-04-28). Also previously surfaced by ST22-08 Offensive Plug-In V (Batch 11) and EX6-072 pattern.
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** LM-027 Red Scramble — "[Security] … Then, add this card to the hand." Also ST22-08 Offensive Plug-In V and any option card whose Security clause ends with returning itself to hand.
- **Effect text:** "Then, add this card to the hand." — the currently-resolving security option card moves to the controller's hand.
- **Status:** Resolved for the narrow pending-security disposition slice on 2026-05-01. `EffectContext::add_pending_security_to_hand()` consumes `Game.pending_security` and pushes the revealed card to the defender/controller hand so the security dispose phase cannot trash it. DSL `add_this_option_to_hand: {}` lowers to the method. Legacy raw-rust shims now delegate to the method; new scripts should use the native step.
- **Coverage:** `debug_runner_dsl::security_dsl_adds_currently_resolving_option_to_hand`; `lm_027_security_adds_card_to_hand_after_play`.
- **Remaining related work:** ST22-08, P-206, EX7-074, and sibling Options may still be blocked by other gaps such as DP/play-cost predicates, Plug-In/Link, Delay timing, or broader Option play-flow disposition.

<!-- Entry template:

---

## DSL Gap: Reveal-zone multi-bucket selection

- Status: RESOLVED on 2026-05-03 for the reusable DSL/runtime primitive. `select_reveal_buckets` now parses and compiles named reveal buckets, validates empty/duplicate/malformed bucket shapes, evaluates bucket predicates against the reveal overlay, installs one `SelectionKind::RevealBucket` pending selection per bucket, and supports `no_duplicate_cards` across buckets without changing action-space or tensor contracts.
- Bucket results bind as `CardList`; singleton bucket lists are compatible with reveal single-card consumers such as `add_to_hand_from_reveal`.
- Lowers to engine API: `EffectContext::select_reveal_buckets(Vec<RevealBucketSelection>, prompt, no_duplicate_cards, callback)`.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- reveal_buckets --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal_buckets --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2e_select_reveal phase2e_select_ordered_permutation phase2b_zone_moves_extra --nocapture`.
- Remaining related blockers: card-specific TS/Olympos YAML migration and any top-security inherited variants are tracked separately; the reusable multi-bucket reveal selection primitive is closed.

---

## DSL Gap: Source-stack DP sum formula

- Status: RESOLVED on 2026-05-03 for the narrow reusable formula leaf. `source_stack_dp_sum` now parses, compiles, validates its optional predicate filter, and evaluates by summing printed DP of live source-stack cards below the target permanent's top card. The optional filter reuses existing card predicate evaluation against each source card handle.
- Implemented DSL formula:
  ```yaml
  source_stack_dp_sum:
    target: self
    filter: { trait_has: Iliad }
  ```
- Lowers to engine formula evaluator: `CompiledFormula::SourceStackDpSum { target, filter }`.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- residual_formula_predicate_vocab group7_formula_batch group7_predicate_batch group6_dynamic_formulas --nocapture`.
- Remaining related blockers: none for summing matching live source-stack card DP; card-specific YAML authoring remains separate.

---

## DSL Gap: Source-stack trash-all-sources step

- Status: RESOLVED on 2026-05-03 for the reusable `trash_all_sources` DSL step and runtime path. The step preserves the target permanent's top card, trashes every below-top source, and is now used by production `BT24-040` YAML.
- Lowers to engine API: `EffectContext::trash_all_sources(target)`.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- source_stack_aggregates --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 --nocapture`.
- Remaining related blockers: source-count aggregate predicates and dynamic De-Digivolve amount formulas for other TS/Olympos cards remain separate.

---

## DSL Gap: Permanent-scoped effect suppression modifier

- Status: RESOLVED on 2026-05-03 for targeted `CannotActivateEffectsByTiming` DSL modifier use. Production `BT24-040` YAML uses it with `CannotSuspend` to lock two selected opponent Digimon/Tamers until the printed expiry.
- Lowers to engine modifier registry: `ModifierType::CannotActivateEffectsByTiming(EffectTiming::WhenDigivolving)` plus the existing `CannotSuspend` modifier.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 --nocapture`.
- Remaining related blockers: aura-wide timing suppression, other printed timings, and unvalidated Venusmon/Queen Device cards remain separate.

---

## DSL Gap: Security Option self-disposition to hand

- Status: COVERED on 2026-05-03 for the narrow currently-resolving security Option moving itself to hand. The existing DSL step `add_this_option_to_hand: {}` already parses/compiles as `AddThisOptionToHand`, lowers through `zone_moves.rs` to `EffectContext::add_pending_security_to_hand()`, and consumes `Game.pending_security` so the security dispose phase cannot also trash the card.
- No new disposition marker/API was added. Broader security disposition primitives such as adding an opponent's top security card to hand remain separate tracker entries.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl -- security_dsl_adds_currently_resolving_option_to_hand --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- option_security_disposition --nocapture`.

---

## DSL Gap: Security stack steps: top-security-to-hand and Recovery

- Status: RESOLVED on 2026-05-03 for the reusable `add_top_security_to_hand` and `recover_from_deck` DSL/runtime steps. Production `BT24-031` and `BT24-101` YAML now exercise top-security-to-hand, trash-top-security, and Recovery branches through behavioral tests.
- Lowers to engine API: `EffectContext::add_top_security_to_hand(player)` and `EffectContext::recover_from_deck(player, count)`.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- security_stack_steps --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- security_stack_operations --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_031 bt24_101 --nocapture`.
- Remaining related blockers: security-stack search/extraction, face-up security handling, placing the resolving permanent itself into security, and card-specific YAML for `BT24-034`, `BT24-090`, and similar cards remain separate.

---

## DSL Gap: Chaos Control — effect-initiated digivolve from trash

- Effect text: EX11-005 Yaamon / EX11-069 Yuuki / BT21-100 The Digimon I Designed / BT24-080 Megidramon all digivolve a battle-area Digimon into a card in trash, sometimes for free and sometimes with a reduced cost.
- Status: resolved for selected live card bindings in Group 4 (2026-05-02). `effect_initiated_digivolve` now accepts `source:` as a source-zone-parametric binding while preserving legacy `from_hand:`.
- Lowers to engine API: `EffectContext::effect_initiated_digivolve_from_source` / `Game::effect_initiated_digivolve_from_source`, with card-handle bindings resolved from hand, trash, security, material stack, or reveal pool.
- Suggested DSL syntax:
  ```yaml
  - effect_initiated_digivolve:
      target: base
      from:
        zone: trash
        of: you
        card: evo
      cost: { reduce: 1 }
      ignore_requirements: false
  ```
- First reported: 2026-04-28
- Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_from_zones`, plus `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group4_zone_movement`.

---

## DSL Gap: EX4-011 — DP deletion threshold from shared trash count

- Status: RESOLVED for the reusable shared-trash formula primitive on 2026-05-02. `FormulaSpec` now accepts `per: { shared_trash_count: {} }` with a `bucket` on the surrounding base/per/delta formula, compiles to `CompiledPerSelector::SharedTrashCount { bucket }`, and runtime evaluation sums both players' trashes before applying bucket floor division.
- Effect text: "For every 10 total cards in both player's trashes, add 2000 to the maximum DP you can choose with DP-based deletion effects."
- Missing DSL verb / step kind / predicate: formula support for cross-player trash count buckets inside a `dp_lte` selection predicate. Existing formula vocabulary covers some modifier values, but not a target-filter threshold derived from `floor((your_trash + opponent_trash) / 10) * 2000`.
- Lowers to engine API: read both players' trash lengths, compute the threshold, then install the normal opponent-permanent selection and `delete_permanent` callback.
- Suggested DSL syntax:
  ```yaml
  dp_lte:
    formula:
      base: 7000
      per: shared_trash_count
      bucket: 10
      delta: 2000
  ```
- First reported: 2026-04-28
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch`.

---

---

## DSL Gap: BT22-008 / BT22-017 — inherited end-of-turn DNA digivolve registration

- Effect text: "[End of Your Turn] This Digimon and another of your Digimon may DNA digivolve into a Digimon card in your hand."
- Missing DSL verb / step kind / predicate: RESOLVED 2026-05-02 for literal-cost, two battle-area material `alt_path_registration` declarative clauses with `kind: dna_digivolve`, `scope: inherited`, and `trigger: end_of_your_turn`.
- Lowers to engine API: the same alternate-path registration and action-mask channel used by normal DNA digivolve costs, producing a player-visible pending/action path rather than an automatic end-of-turn digivolve.
- Suggested DSL syntax: keep the existing `alt_path_registration` shape and require lowering for inherited clauses, including `timing: end_of_your_turn`, `kind: dna_digivolve`, material filters, target hand-card filter, and cost override.
- Also blocks: `BT12-021` Veemon and `BT12-047` Wormmon in BG Imperial. Their inherited text is "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand."
- Covered by: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_alt_path_registration`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- dna`.
- Remaining limits: formula costs, extra costs, `from` gates, burst end steps, `stacks_unsuspended`, non-battle-area materials, repeat/unbounded materials, `ignore_requirements`, `source_treated_as` for DNA routes, marker routes, and non-DNA alt-path kinds beyond normal digivolve are intentionally not consumed by this v1 action hook. Normal digivolve `source_treated_as` routes are covered separately by the 2026-05-03 Hybrid/Tamer closure.
- First reported: 2026-04-28

---

## DSL Gap: BG Imperial DNA cards — YAML `dna_costs` authoring / production data population

- Effect text: "[DNA Digivolve] Blue Lv.4 + Green Lv.4 : Cost 0" and equivalent BG Imperial DNA requirements.
- Missing DSL verb / step kind / predicate: RESOLVED 2026-05-02 for top-level `alt_paths: [{ kind: dna_digivolve, ... }]` authoring into runtime `CardData.dna_costs`.
- Lowers to engine API: `CardData.dna_costs`, consumed by the DNA digivolve action-mask branch and `Game::initiate_dna_digivolve`.
- Covered by: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_dna_alt_path_enriches_card_data_dna_costs`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- authored_dna_alt_path_makes_dna_action_legal_for_bt20_016`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_016_has_dna_digivolve_alt_path`.
- Full verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml`, `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v`, and `python -m pytest code/tests/rl -v`.
- Remaining limits: this closure covers top-level printed DNA digivolve card data. Inherited/end-of-turn registration is covered separately by the preceding entry and supports the existing DNA action IDs for the v1 literal-cost two-material shape.
- First reported: 2026-04-28 (BG Imperial assess-rust-engine-archetype)

---

## DSL Gap: Group 8 — scoped DigiXros aliases, ACE Overflow metadata, and reveal overlays

- Effect text: "also treated as [X] for DigiXros", `<ACE> Overflow -N`, and reveal effects whose revealed cards should be evaluated with temporary name/kind identity only while in the reveal zone.
- Status: RESOLVED 2026-05-02 for the planned vocabulary/data slices.
- Lowered runtime surfaces: `CardData.digixros_aliases` / compiled `digixros_aliases` for DigiXros-only material matching; `CardData.ace_overflow` for memory loss when ACE cards leave battle-area stacks or are removed from under a stack; and `RevealOverlay` on `CardSource` for reveal-zone predicate evaluation until destination movement clears the overlay.
- Covered by: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor`, and full `cargo test --manifest-path code/digimon-engine/Cargo.toml`.
- Remaining limits: full DigiXros play flow, reveal overlays outside explicit reveal-zone predicates, and unrelated immediate-attack/declarative-keyword gaps remain separate entries.
- First reported: consolidated from Group 8 token/card-data gap planning.

---

## DSL Gap: EX11-008 — [When Moving] timing (DSL half — see engine-gaps.md for engine half)

- Effect text: "[When Moving] [On Play] 1 of your Digimon with the [Reptile] or [Dragonkin] trait gains <Raid> and +3000 DP for the turn."
- Status: fixed for the timing token and direct runtime event context on 2026-04-29. `when: on_move` lowers to `EffectTiming::OnMove`; `event_target_trait_has` can inspect the moved permanent/card from `TriggerSource::MovedFromBreeding`. Verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_move_event_target_trait_predicate_matches_moved_permanent`.
- Remaining DSL work: card-specific bodies still need any additional step verbs/predicates they print, such as EX11-008's target grant body. BT16-082's reveal/add-to-hand/remainder-bottom/hatch tail is implemented as of 2026-05-04 via native reveal steps plus the reusable `can_hatch` predicate.
- Gap kind: hybrid (this entry tracks the DSL half; engine half tracked separately).
- First reported: 2026-04-27 (EX11-008 batch-implement-cards-rust-dsl)

---

---

## DSL Gap: BT24-082 / BT21-081 — Immediate optional attack within effect resolution  [G-MAY-ATTACK-NOW]

- Effect text (BT24-082): "[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack."
- Effect text (BT21-081): "[End of Your Turn] By suspending this Tamer, 1 of your Digimon with the [Reptile] or [Dragonkin] trait gains <Piercing> for the turn. Then, that Digimon attacks."
- Status: RESOLVED for the reusable immediate in-effect attack primitive on 2026-05-03. The DSL now has `may_attack_now`, which lowers to `EffectContext::may_attack_now_optional(...)` and installs an existing-action-ID pending target selection mid-effect-resolution.
- Engine notes: `ModifierType::MayAttack` / `ForceAttack` remain EOT-window modifiers and are still not the right vehicle for this specific immediate prompt. This gap is closed by a distinct effect-context primitive, not by registering those modifiers.
- Lowers to engine API: `EffectContext::may_attack_now_optional(attacker, targets, without_suspending, optional, prompt)`; mandatory "then attacks" is `optional: false`, optional "may attack" is `optional: true`.
- Suggested DSL syntax:
  ```yaml
  - may_attack_now:
      attacker: tgt
      targets: any        # any | player | digimon
      optional: true      # false for mandatory "then attacks"
      without_suspending: false
  ```
- Gap kind: closed for immediate attack prompts. Persistent player-scoped grants such as `MayAttackPlayerOnly` and cross-side granted forced attacks remain separate engine gaps.
- Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- effect_granted_attack --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- effect_granted_attack --nocapture`.
- First reported: 2026-04-27 (BT24-082 batch-implement-cards-rust-dsl)

---

---

## DSL Gap: P-189 — [Security] play cost ≤ 4 filter on select_hand / select_trash  [G-PLAY-COST-LTE]

- Status: CLOSED on 2026-05-01. `play_cost_lte` is now parsed, compiled, evaluated against `CardData::play_cost`, and wired into `select_hand` / `select_trash` valid-action filtering. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch parse_group7_predicate_leaves`.
- Effect text: "[Security] You may play 1 card with the [LIBERATOR] trait and a play cost of 4 or less from your hand or trash without paying the cost."
- Missing DSL verb / step kind / predicate: `play_cost_lte` (or `cost_lte`) — a `PredicateSpec` leaf that checks `CardData::play_cost <= N`. `PredicateSpec` in `digimon-dsl/src/predicate.rs` has no cost-comparison field. The `eval_card_fields` function in `code/digimon-engine/src/dsl_cards/predicate.rs` handles `level_eq`, `level_lte`, `level_gte`, `color_is`, `trait_has`, `name_*`, `card_number_is` — but no `play_cost` / `cost_lte` / `cost_gte` variant.
- Companion issue: `install_select_hand` and `install_select_trash` in `code/digimon-engine/src/dsl_cards/step/selections.rs` currently use `|_game, _idx| true` (accept-all filter, Phase 2b), so even if `play_cost_lte` were added to `PredicateSpec`, it would not be evaluated until Phase 2b filter wiring is completed.
- Lowers to engine API: no new engine method needed. Fix requires: (1) add `play_cost_lte: Option<u32>` (and optionally `play_cost_gte`) to `PredicateSpec`; (2) add a branch in `eval_card_fields` to check `card_data.play_cost <= n`; (3) wire the filter predicate into `install_select_hand` and `install_select_trash`.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - trait_has: LIBERATOR
      - play_cost_lte: 4
  ```
- Gap kind: dsl (engine already stores `play_cost` on `CardData`; the DSL/lowering path just lacks the predicate leaf).
- Workaround: none needed for static play-cost caps after 2026-05-01. Previously ignored tests for incorrect candidate filtering can be unignored when updating affected card suites.
- First reported: 2026-04-27 (P-189 batch-implement-cards-rust-dsl)

---

---

## DSL Gap: BT5-008 — `other: true` predicate not evaluated in `eval_permanent_fields`  [G-OTHER-PREDICATE-UNEVALUATED]

- Effect text: "[Your Turn] Your other [Gaossmon] all get +3000 DP."
- Status: RESOLVED by Group 6. `eval_permanent_fields` now rejects the source `PermanentHandle` when `other: true` is evaluated with `source_permanent` context, so filtered declarative auras can exclude their own source permanent.
- Passing command(s): `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture`.
- Remaining related blocker: none for this predicate primitive.
- Suggested DSL syntax: already in the DSL spec as `other: true`.
- Gap kind: closed DSL/runtime predicate gap.
- First reported: 2026-04-27 (BT5-008 batch-implement-cards-rust-dsl, Medusamon archetype). Closed: 2026-05-02 (Group 6 Task 3).

---

---

## DSL Gap: BT5-008 — Player-level flood-gate modifier not installable from DSL  [G-PLAYER-FLOOD-GATE-DSL]

- Effect text: "[Opponent's Turn] Your opponent can't reduce digivolution costs."
- Status: RESOLVED by Group 6 for aura-delivered player modifiers, and previously closed for DSL vocabulary/direct runtime primitives. The DSL supports `target_player` on `kind: flood_gate` and an explicit `add_player_modifier` step, and the engine has a `CannotReduceDigivolveCost` modifier with digivolve-only cost-reduction enforcement.
- Engine note 2026-05-02: player-scoped `IgnoreColorRequirement` is now consumed by Rust Option masks and decode/execution. No new DSL syntax landed in that engine-only pass; passive/static field dispatch remains governed by the related blocker below.
- Passing command(s): `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture`.
- Remaining related blocker: flood-gate-specific tick coverage is still worth adding separately, but aura-delivered player modifiers no longer require a raw-rust placeholder.
- Engine note 2026-05-02: `Game::tick_declarative_effects` now dispatches process-backed declarative effects from face-up field sources, and Group 6 aura coverage proves a `kind: aura` with `target_player: opponent` installs `CannotReduceDigivolveCost` on the referenced player. The same dispatcher executes flood-gate process closures, but this entry does not claim dedicated flood-gate installation coverage until a flood-gate-specific tick test lands.
- Implemented DSL syntax:
  ```yaml
  - kind: flood_gate
    active_when: { opponents_turn: true }
    target_player: opponent
    modifier: CannotReduceDigivolveCost
  ```
- Gap kind: closed vocabulary/runtime primitive gap; aura-based passive dispatch is covered by Group 6 Task 3, with flood-gate-specific dispatcher coverage still worth adding separately.
- Former workaround removed for BT5-008: raw_rust no-op placeholder (`bt5_008_opp_cannot_reduce_digivolve_cost`).
- First reported: 2026-04-27 (BT5-008 batch-implement-cards-rust-dsl, Medusamon archetype). Closed: 2026-05-01 (floodgate DSL flexibility pass).

---

---

## DSL Gap: P-137 — Opponent adds top security card to hand  [G-ADD-TOP-SECURITY-TO-HAND]

- Effect text: "[Your Turn][Once Per Turn] When this Digimon's attack target is switched, your opponent adds the top card of their security stack to the hand."
- Status: RESOLVED for the reusable DSL/runtime primitive on 2026-05-03. `add_top_security_to_hand` now moves the top security card to the owner's hand and preserves the security-loss observer chain. P-137 production YAML still needs a card-level cleanup pass if it retains an old raw-rust workaround.
- Lowers to engine API: `EffectContext::add_top_security_to_hand(player: PlayerId) -> bool` — pops `security.last()`, pushes to `hand`, fires `EffectTiming::OnLoseSecurity` via `SecurityRevealed` and `EffectTiming::OnOpponentSecurityRemoved` via `PlayerBattleArea(controller)`.
- Suggested DSL syntax:
  ```yaml
  - add_top_security_to_hand: { of: opponent }
  ```
- Gap kind: resolved reusable DSL/engine primitive; card-local raw_rust removal remains a separate authoring cleanup if present.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- security_stack_steps --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test effect_context -- security_stack_operations --nocapture`.
- First reported: 2026-04-27 (P-137 batch-implement-cards-rust-dsl, Medusamon Batch 8)

---

---

## DSL Gap: P-035 / P-103 / BT24-089 — Option-as-permanent placement (inherited security)  [G-PLACE-SELF-AS-OPTION-PERMANENT]

- Effect text (P-035): "[Main] … Then, place this card in your battle area." and "[Security] Place this card in the battle area." (inherited)
- Status: IMPLEMENTED for inherited-security source Options as of 2026-05-02. The DSL verb / step kind is `place_self_as_delay_option: {}` — a step that places the currently-resolving Option card into the battle area as an `OptionState::Delayed` permanent from within an inherited security context. Two contexts exist:
  1. **Main clause** ("Then, place this card in your battle area."): DCGO calls `PlaceDelayOptionCards(card, activateClass)`. In the Rust engine, `dispose_option` + `classify_option_subtype` detect the `kind: delay` clause and auto-place the card at the `MainEffectDrain` phase — no explicit DSL step is needed. The engine handles placement implicitly.
  2. **Inherited security clause** ("[Security] Place this card in the battle area."): DCGO calls `CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)`. Rust now exposes `EffectContext::place_self_as_delay_option_permanent`, which removes the matching non-top source Option from its host stack, places it in the owner's battle area as `OptionState::Delayed`, and dispatches `OnOptionPlaced`.
- Lowers to engine API: `EffectContext::place_self_as_delay_option_permanent(&mut self)`. In the inherited-security context, this method: (1) identifies the source Option card (the digivolution source that triggered this effect), (2) removes it from its current location (digivolution stack), and (3) places it in `self.game.players[owner].battle_area` as a `Permanent` with `OptionState::Delayed`.
- Suggested DSL syntax:
  ```yaml
  - place_self_as_delay_option: {}
  ```
  Used in the `process:` of the inherited security clause. Not needed in the Main clause (engine auto-placement via `dispose_option` suffices).
- Gap kind: resolved dsl vocabulary/engine API gap for inherited security context. Engine already handled the Main clause auto-placement; inherited-security-context placement now uses the explicit DSL step because `dispose_option` is not called in that path.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- inherited_security_places_source_option_as_delay_permanent`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- place_self_as_delay_option`.
- Group 5 handoff verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed`.
- Workaround: no longer needed for new YAML/tests. Existing `process: []` placeholders for P-035, P-103, and BT24-089 can migrate to `place_self_as_delay_option: {}` when those card YAMLs/tests are revisited. Existing ignored card-level tests that still name `G-PLACE-SELF-AS-OPTION-PERMANENT` remain migration/process-body follow-ups, not an open reusable placement primitive; keep distinct card-level blockers such as remaining P-206 Delay/action-flow work separate.
- First reported: 2026-04-28 (P-035 Red Memory Boost! batch-implement-cards-rust-dsl, Medusamon Batch 12). Same gap pre-existed in P-103.yaml and BT24-089.yaml without a tracker entry.

---

---

## DSL Gap: P-206 — Board-color cross-reference predicate in Delay clause  [G-COLOR-MATCH-AGAINST-BOARD]

- Effect text: "[Main] ＜Delay＞ … You may play 1 Tamer card with the same color as any of your Digimon on the field from your hand with the play cost reduced by 4."
- Status: RESOLVED on 2026-05-02 for dynamic board-color card predicates. `color_matches_any_field_digimon` now parses, compiles to `CompiledPredicate.color_matches_any_field_digimon`, and evaluates against live battle-area Digimon top-card colors during selection filtering. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- color_matches_any_field_digimon_compiles`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_hand_color_matches_any_field_digimon_filters_by_live_board_colors`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch`.
- Implemented DSL predicate: `color_matches_any_field_digimon` — a `PredicateSpec` leaf that checks whether a candidate card's colors share at least one color with any Digimon currently in the requested player's battle area.
- Lowers to engine API: `CardData::colors` read from the candidate card and from the requested player's battle-area top cards. No new engine method was needed; empty-board behavior is false.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - kind: tamer
      - color_matches_any_field_digimon: { of: you }
  ```
- Gap kind: resolved dsl vocabulary/evaluator gap for dynamic card-color filtering.
- Workaround: no longer needed for the reusable predicate. P-206 card YAML may still need separate Delay, Option, or action-flow follow-up work before the full card can be unblocked.
- First reported: 2026-04-28 (P-206 Digital Gate Open batch-implement-cards-rust-dsl, Medusamon Batch 14)

---

---

## DSL Gap: BT8-097 / Royal Knights — formula filters for counted battle-area cards  [G-FORMULA-KIND-FILTER]

- Status: RESOLVED for reusable formula-zone count filters on 2026-05-02. `card_count_in_zone` payloads now accept `filter: { ... }`; the compiler carries the predicate into filtered count IR, and runtime evaluation counts only representable subjects that satisfy the predicate instead of falling back to an unfiltered count.
- Effect text: `BT8-097` Crimson Blaze: "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play."
- Implemented DSL form: `card_count_in_zone` formulas can now apply a `kind: digimon` filter. `BT8-097.yaml` uses this filtered form so Tamers and Option permanents no longer reduce Crimson Blaze's play cost.
- Lowers to engine API: the engine can inspect each battle-area permanent and test `Permanent::is_digimon(&card_data)`; the formula DSL needs a filtered-count form that passes a compiled predicate into formula evaluation.
- Suggested DSL syntax:
  ```yaml
  amount_fn:
    base: 0
    per:
      card_count_in_zone:
        of: opponent
        zone: battle_area
        filter: { kind: digimon }
    delta: 1
  ```
- Gap kind: resolved dsl vocabulary/evaluator gap for filtered zone-count formulas.
- Workaround: no longer needed for BT8-097 or other `card_count_in_zone` formulas with simple predicate filters.
- First reported: 2026-04-28 (Royal Knights archetype assessment; surfaced by BT8-097 in Royal Knights lists)
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch phase3d_formula_zone_count`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_097`.


---

## Engine Gap: `EffectContext::add_top_security_to_hand` Missing (engine half of G-ADD-TOP-SECURITY-TO-HAND)

- **Discovered in:** Medusamon Batch 8, P-137 Flamedramon DSL implementation (2026-04-27)
- **Card(s):** P-137 Flamedramon — "[Your Turn][Once Per Turn] When this Digimon's attack target is switched, your opponent adds the top card of their security stack to the hand."
- **Effect text:** "opponent adds the top card of their security stack to the hand"
- **What's missing:** `EffectContext` only exposes `trash_top_security(player)` for security removal. There is no `add_top_security_to_hand(player)` method that pops the top security card and places it in the player's hand while firing the standard security-removed event chain (`OnLoseSecurity` via `SecurityRevealed` + `OnOpponentSecurityRemoved` via `PlayerBattleArea`).
- **Suggested change:** Add `pub fn add_top_security_to_hand(&mut self, player: PlayerId) -> bool` to `EffectContext`. Implementation: pop `security.last()`, push to `hand`, fire `EffectTiming::OnLoseSecurity` with `TriggerSource::SecurityRevealed { defender: player, card: card_handle }` and `EffectTiming::OnOpponentSecurityRemoved` with `TriggerSource::PlayerBattleArea(controller)`.
- **Workaround:** `raw_rust: { fn: p_137_opp_adds_top_security_to_hand }` — manually implements the move + event chain in `src/cards/raw_rust/mod.rs`.

---

## DSL Gap: ST22-08 — Link Registration Clause (Plug-In / Link Card Mechanic)  [G-DSL-LINK-VERB]

- Status: closed 2026-05-02. DSL now supports `kind: link_requirement` lowering to `Effect::link(cost, filter)` and `link_to_own_digimon` process steps that surface the existing Link host-selection `PendingSelection` without adding action IDs.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`.
- Effect text: "Inherited: Link Requirements [Link] Lv.3 or higher: Cost 2 (Plug this card from the hand or battle area sideways into the specified Digimon in the battle area.)"
- Also: "[Main] You may link this card to 1 of your Digimon without paying the cost."
- Missing DSL verb / step kind / predicate: Two missing DSL constructs:
  (a) A declarative clause kind for declaring link requirements — no `kind: link_requirement` or equivalent in `TypedDeclarativeBody`. The closest existing cards (EX11-027 Maquinamon) use `kind: raw_rust fn: ex11_027_link_requirements triggers: [main_from_hand, main_on_field]`.
  (b) An optional link-action step within a `process:` body — no `link_to_digimon:` or similar step verb. DCGO's `SelectPermanentEffect` with `Mode.Custom` + `card.CanLinkToTargetPermanent(permanent, false)` + `canNoSelect: true` drives this. The engine has `OptionSubtype::Link`, `Effect::link(cost, filter)`, and `attach_linked_card()`, but these are reachable only via hand-written `CardEffect` or raw_rust functions.
- Lowers to engine API: `Effect::link(cost, filter_fn)` on the declarative side; `ctx.game.attach_linked_card(host_handle)` on the step side. Both exist in the engine; neither is accessible from the DSL step vocabulary.
- Suggested DSL syntax:
  ```yaml
  # Declaration form (inherited link requirements):
  - kind: link_requirement
    scope: inherited
    cost: 2
    filter: { level_gte: 3 }
  
  # Step form (optional free link in process body):
  - link_to_own_digimon:
      optional: true
      cost_delta: -99   # or free: true
      filter: { kind: digimon }
      bind_as: linked_host
  ```
- Gap kind: dsl (engine has the primitive; DSL lacks both the clause kind and the step verb).
- Remaining card-level blockers: ST22-08 behavioral tests that still name `G-DSL-LINK-VERB` are stale card migration placeholders unless they also depend on separate process-body blockers such as `G-BINDING-DP-FORMULA`, DP/play-cost predicates, or card-specific YAML rewrites.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: ST22-08 — Linked-Card Effect Scope  [G-DSL-LINKED-SCOPE]

- Status: closed 2026-05-02. DSL now accepts `scope: linked`; triggered lowering marks the emitted effect with `.linked()` so the existing linked-card dispatch path can fire it from the host.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`.
- Effect text: DCGO shows EndOfTurnLinkedEffect with `activateClass.SetIsLinkedEffect(true)` — an effect that fires only when the card is linked to a Digimon in the battle area, and the linked Digimon may attack at the controller's end of turn.
- Missing DSL verb / step kind / predicate: `scope: linked` — a clause scope for effects that fire as if they were part of the Digimon the card is linked to. `CompiledScope` in `digimon-dsl/src/compiled.rs` has `FaceUp` and `Inherited` variants; there is no `Linked` variant. The effect-queue (`effect_queue.rs`) already handles linked cards in `enqueue_from_permanent` (the Phase 8 Task 4 linked_cards branch), but the scope is expressed as a raw `linked_cards` list on `Permanent`, not as a DSL-compiled clause with `scope: linked`.
- Lowers to engine API: the engine already fires effects for linked cards via the `linked_cards` loop in `enqueue_from_permanent`. The DSL lowering layer would need to detect `scope: linked` on a clause and install the resulting `Effect` via `Effect::declarative(card)` with a flag indicating it should be enqueued from the linked-card path rather than the top-card path.
- Suggested DSL syntax:
  ```yaml
  - scope: linked
    when: end_of_your_turn
    optional: true
    once_per_turn: true
    process:
      - raw_rust: { fn: st22_08_linked_eot_may_attack }
  ```
- Gap kind: dsl (engine fires linked-card effects; DSL has no `scope: linked` clause kind that lowers into the linked-card effect list).
- Remaining card-level blockers: ST22-08 ignored tests that still name `G-DSL-LINKED-SCOPE` should be migrated or retagged during card implementation; the reusable DSL scope is closed. Leave unrelated blockers such as `G-BINDING-DP-FORMULA` open.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: ST22-08 — Named-Binding DP Reference in Formula  [G-BINDING-DP-FORMULA]

- Status: RESOLVED for the reusable `binding_dp` formula primitive on 2026-05-02. `dp_lte: { formula: { binding_dp: ally } }` now parses, compiles to `CompiledFormula::BindingDp("ally")`, and predicate formula evaluation threads current `Bindings` so the threshold reads the named permanent's effective DP.
- Effect text: "[Main] … delete 1 of your opponent's Digimon with as much or less DP as 1 of your Digimon."
- Missing DSL verb / step kind / predicate: `binding_dp` — a formula primitive that reads the effective DP of a named binding (a `PermanentHandle` stored by `bind_as:` from a prior `select_own_permanent`). The formula system (`formula.rs` + `formula_eval.rs`) can read `source_permanent`'s DP via `{ of: source_permanent, value: dp }` (see DSL spec §3.10), but there is no form to read an arbitrary named binding's DP — which is required for "DP ≤ chosen own Digimon's DP" where the comparator is player-selected mid-effect.
- Lowers to engine API: `ctx.game.effective_dp(handle)` — already exists. The gap is that `CompiledFormula` has no `BindingDp(String)` variant that reads `bindings.get_permanent(name)` and calls `effective_dp`. 
- Suggested DSL syntax:
  ```yaml
  # In dp_lte formula, reference a named binding:
  dp_lte:
    formula:
      binding_dp: ally   # resolves bindings["ally"] as PermanentHandle, calls effective_dp
  ```
  Requires: (1) add `BindingDp(String)` to `FormulaSpec` and `CompiledFormula`; (2) add evaluation branch in `formula_eval.rs` that resolves the binding from `Bindings` and calls `ctx.game.effective_dp(h)`; (3) pass `Bindings` into the formula evaluator call chain.
- Gap kind: dsl (engine has `effective_dp`; DSL formula system has no binding-reference form).
- Workaround: None for `binding_dp` itself. Static and existing formula-backed `dp_lte` / `dp_gte` permanent predicates are now evaluated as of 2026-05-01, but this gap remains open until formulas can read a named binding's effective DP.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch group7_predicate_batch`.

---

---

## DSL Gap: ST22-08 — Return Played Option Card to Hand Post-Security  [G-ADD-OPTION-SELF-TO-HAND]

- Effect text: "[Security] Delete 1 of your opponent's Digimon with the lowest DP. Then, add this card to the hand."
- Status: Resolved for the narrow pending-security disposition slice on 2026-05-01. DSL now supports a deterministic `add_this_option_to_hand: {}` step that lowers to `EffectContext::add_pending_security_to_hand()`, consuming `Game.pending_security` and moving the revealed card to the defender/controller hand instead of letting security dispose trash it.
- DSL syntax:
  ```yaml
  - add_this_option_to_hand: {}
  ```
- Coverage: `debug_runner_dsl::security_dsl_adds_currently_resolving_option_to_hand` and `lm_027_security_adds_card_to_hand_after_play`.
- Remaining related gaps: cards may still be blocked by other predicates or Option mechanics, such as lowest-DP selection, Plug-In/Link, Delay timing, or play-cost filters.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: BT21-072 — [All Turns] +1000 DP per digivolution card (dynamic formula aura)  [G-AURA-DP-FORMULA]

- Effect text: "[All Turns] This Digimon gets +1000 DP for each of its digivolution cards."
- Missing DSL verb / step kind / predicate: `dp_modifier_fn` / `dp_modifier_formula` — a formula-based variant of `AuraBody.dp_modifier`. The DCGO implements this via `ChangeSelfDPStaticEffect(changeValue: 1000 * count(), ...)` at `EffectTiming.None`, where `count()` is a live lambda returning `PermanentOfThisCard().DigivolutionCards.Count()` (= material_count = stack_size - 1). This is a **continuously-recomputed** aura that updates dynamically each tick, including after `de_digivolve` operations that pop digivolution cards from the stack. The DSL `kind: aura` with self-target accepts only `dp_modifier: Option<i32>` — a static literal with no formula variant. The `FormulaSpec` type (with `per: material_count, delta: 1000`) exists for step-level `add_dp_modifier` verbs, but `add_dp_modifier` only snapshots the formula's value at event-fire time, not continuously. Storing a snapshot in `Effect.dp_modifier` cannot model the dynamic behaviour required.
- Lowers to engine API: `source_dp_contribution(perm_handle, source_index)` reads `Effect.dp_modifier` continuously — the engine query mechanism already supports live reads. The gap is that `AuraBody` has no formula field to store a `FormulaSpec` that `lower_aura.rs` could evaluate at read-time rather than compile-time.
- **Status:** RESOLVED by Group 6. `kind: aura` now accepts `dp_modifier_fn` and lowers it into a live runtime formula closure instead of snapshotting into `Effect.dp_modifier`. `Game::effective_dp` and `Game::source_dp_contribution` evaluate that closure at query/tensor time, so `per: material_count` changes when the stack changes. Sibling coverage: `security_attack_fn` is also accepted on aura clauses and is recomputed at security-check resolution, alongside printed Security Attack keywords and `ModifierType::SecurityAttackChange`.
- **Passing command(s):** `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_dynamic_formulas --nocapture`; broader formula regression coverage remains in `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- --nocapture`.
- **Remaining related blocker:** none for formula-backed DP and Security Attack aura clauses.
- Suggested DSL syntax:
  ```yaml
  - kind: aura
    active_when: { all_turns: true }   # or omit for always-on
    target: {}                          # self
    dp_modifier_fn:                     # NEW: formula-based dynamic variant
      base: 0
      per: material_count               # CompiledPerSelector::MaterialCount = stack_size - 1
      delta: 1000
  ```
  Implementation notes: implemented as `AuraBody.dp_modifier_fn` / `CompiledDeclarativeClause::Aura::dp_modifier_fn` plus runtime `Effect.dp_modifier_fn` evaluation in the DP query path.
- Gap kind: dsl (closed for formula-backed DP and Security Attack aura clauses).
- Workaround: None for this shape.
- First reported: 2026-04-27 (BT21-072 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: BT20-102 — Omnimon (X Antibody) self-digivolution-stack name check  [G-SELF-DIGIVOLUTION-CONTAINS-NAME]

- Effect text: "[On Play][When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, ..."
- DSL predicate coverage: `self_digivolution_contains_name` is the needed `BoolPredicate` leaf for evaluating this Digimon's digivolution stack from a triggered clause's `condition:` block. The DSL predicate `source_name_contains` applies to the SOURCE PERMANENT (the Digimon this card is stacked under, in inherited contexts) — not to this card's own digivolution stack at runtime.
- Engine support: `Permanent::contains_card_name(name, data)` exists in `code/digimon-engine/src/permanent.rs` and scans the full stack. Triggered `condition:` / `active_when:` evaluation now passes a `PredicateSubject::Permanent(source_h)` when a live source permanent is available.
- Lowers to engine API: `Permanent::contains_card_name(name, &game.card_data)` on `rctx.source_permanent()`.
- Suggested DSL syntax:
  ```yaml
  condition:
    self_digivolution_contains_name: "Omnimon"
    # or: any_of: [{ self_digivolution_contains_name: "Omnimon" }, { self_digivolution_contains_name: "X Antibody" }]
  ```
  Implementation: add `self_digivolution_contains_name: Option<String>` to `BoolPredicateSpec` in `digimon-dsl/src/predicate.rs`, compile to `CompiledPredicate` field, evaluate in `eval_predicate(p, rctx, PredicateSubject::Permanent(source_h))` where `source_h` is the triggering permanent's handle — requires threading the source handle into the triggered-clause condition closure in `lower_triggered.rs`.
- Updated 2026-05-02: `self_digivolution_contains_name` is now a compiled predicate leaf, triggered `condition:` / `active_when:` evaluation passes the live source permanent as subject when available, and runtime evaluation scans the full source stack through `Permanent::contains_card_name`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch`.
- Gap kind: hybrid; the predicate leaf and triggered-condition subject threading are implemented, with broader BT20-102 authored-card coverage tracked separately.
- Workaround: entire boardwipe clause routed through `raw_rust: { fn: bt20_102_boardwipe_and_return }` which calls `perm.contains_card_name(...)` directly. Over-wide: top card name "Omnimon (X Antibody)" contains "X Antibody" so condition is always true for BT20-102 even with no digivolution source.
- First reported: 2026-04-27 (BT20-102 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

---

## DSL Gap: BT20-102 — Exclude-from-binding filter in `for_each`  [G-FOR-EACH-EXCLUDE-BINDING]

- Status: CLOSED on 2026-05-01. `not_in_binding` is now parsed, compiled, evaluated against `Permanent` / `PermanentList` bindings, and `for_each` threads current bindings into its permanent scan. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch parse_group7_predicate_leaves`.
- Effect text: "[On Play][When Digivolving] ... choose 1 of both players' Digimon and delete all other Digimon."
- Missing DSL verb / step kind / predicate: `not_in_binding` — a `CandidatePredicate` leaf in `for_each { over, filter, body }` that excludes permanents whose handle appears in a named binding (a prior selection). Without it, "delete all OTHER Digimon" (all except the two saved by selection) cannot be expressed purely in DSL.
- Engine API: the engine can iterate `battle_area` handles and compare against a collected `Vec<PermanentHandle>`. No new API needed — gap is purely in the DSL filter vocabulary.
- Suggested DSL syntax:
  ```yaml
  - for_each:
      over: { owner: any, kind: digimon, not_in_binding: saved }
      bind_as: candidate
      body:
        - delete_permanent: { target: candidate }
  ```
  Implementation: add `not_in_binding: Option<String>` to `CandidatePredicateSpec` in `digimon-dsl/src/predicate.rs`, compile, and evaluate by looking up the named binding in `Bindings` and comparing handle equality.
- Gap kind: dsl (engine can express this in a raw_rust loop; DSL has no filter for handle-set exclusion).
- Workaround: none needed for handle-set exclusion after 2026-05-01. Card YAML/tests may still need separate migration away from any raw_rust bridge if other blockers remain.
- First reported: 2026-04-27 (BT20-102 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## Rust Engine Gap Group Summaries

- **Group 6 modifiers / auras / keywords closure (2026-05-02):** Status: resolved for the planned Group 6 primitives. Option color bypass uses the existing player-scoped modifier in both masks and decode/execution; source-scoped return/de-digivolve immunity blocks opponent effects while allowing own effects, battle, costs, and rule cleanup; filtered/dynamic auras refresh without stale materialized modifiers; dynamic DP and Security Attack formulas recompute from existing tensor/action surfaces; Collision, Piercing, Reboot, Retaliation, and Progress use existing combat/action paths; Overclock uses the field-effect sub-slot; and DigiXros aliases remain scoped to DigiXros material matching only. Passing targeted coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- source_scoped_immunity --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras group6_dynamic_formulas --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- group6_keywords group6_overclock --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parses_group6_core_combat_keywords parses_digixros_scoped_alias_without_global_name_alias --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- digixros_matching_accepts_scoped_alias_but_generic_name_checks_do_not --nocapture`. Broad Rust gates passed with Cargo's required no-filter separator forms for `--nocapture`. Python gates are not closure evidence as of this review: `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v` failed one tensor-shape parity test (`(1375,)` legacy vs `(8320,)` Rust default), and `python -m pytest code/tests/rl -v` failed three legacy/Rust parity tests plus `test_same_seed_reproduces_first_action`.

- **G-DELAY-START-OF-TURN / start-of-turn Delay options (Group 5, 2026-05-02):** resolved by `DelayTrigger::StartOfYourNextTurn`, trigger-aware `OptionState::Delayed`, a start-of-turn delay drain in `Game::begin_turn`, and DSL lowering from `CompiledTiming::StartOfYourTurn`. Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- start_of_your_next_turn_delay_fires_at_turn_start_not_end`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay_start_of_your_turn_maps_to_start_of_your_next_turn`. Remaining Scramble/card blockers are non-timing gaps tracked separately.
- **Group 8 token/card-data gap closure (2026-05-02):** Status: implemented for the planned slices. Familiar Tokens now have their printed mandatory `OnDeletion` target selection and -3000 DP modifier; all registered tokens synthesize complete `CardData`; top-level authored DNA `alt_paths` populate runtime `CardData.dna_costs`; `digixros_aliases` are scoped to DigiXros material matching and do not leak into generic name predicates; `ace_overflow` card data is enforced when ACE cards leave battle-area stacks or leave from under a stack; and reveal-zone overlays let reveal predicates see temporary name/kind identities until the card moves to its destination. Regression coverage includes `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- token`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor`, full `cargo test --manifest-path code/digimon-engine/Cargo.toml`, `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v`, and `python -m pytest code/tests/rl -v`. Remaining limits: inherited/end-of-turn DNA alt-path registration, full DigiXros play flow, reveal overlays outside explicit reveal-zone predicates, and unrelated token/keyword mechanics remain tracked by their own entries.
- **Group 4 zone/source/security movement (2026-05-02):** implemented source-parametric effect digivolve, exact material-source movement/trash with source-trash event context, selected security-to-hand and security shuffle DSL steps, effect-driven security removal cleanup that resumes after `OnLoseSecurity` selections without duplicating cards, full-stack return-to-deck with owner routing, and real breeding-slot effect movement/placement helpers.
- **Latest source-stack residual closure (2026-05-03 Task 4):** direct source-stack residual operations are narrowed by `EffectContext::trash_all_sources`, DSL `trash_all_sources`, and DSL `play_selected_sources_free` over stable `SourceSelectionRef` bindings. Same-level source aggregate formula coverage remains in the existing Group 7 formula path. Focused coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- source_stack_operations --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- source_stack_aggregates group7_formula_batch group7_predicate_batch --nocapture`.
- **Latest Hybrid/Tamer alt-path closure (2026-05-03 Task 5):** normal hand-to-battle-area digivolve masks and execution now consume DSL `alt_paths.kind: digivolve` with literal costs and `source_treated_as`, allowing Tamer-as-level/color Hybrid bases without mutating printed Tamer kind. Union-zone selections are covered end-to-end into effect-initiated digivolve from a selected `CardHandle`. Focused coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- hybrid_tamer_digivolve phase2e_select_union_zone --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_union_zones effect_digivolve_from_zones --nocapture`. Remaining limits: dynamic-cost effect digivolve from Tamer/Digimon base unions, delayed cleanup riders, and card-local Red Hybrid YAML authoring.
- **Cost and Replacement Framework (Group 3, 2026-04-30):** Status: implemented. Regression coverage: `code/digimon-engine/tests/cost_hooks/stacked_would_play_reducers.rs`; `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`; `code/digimon-engine/tests/replacements/context_predicates.rs`; `code/digimon-engine/tests/replacements/partition.rs`; `code/digimon-engine/tests/option_flow/replacement_integration.rs::bt17_097_delay_prevents_deletion_and_digivolves_from_hand`; `code/digimon-engine/tests/replacements/attack_cancel.rs`. The engine supports stacked optional would-play cost reducers, triggered pay costs that park pending selections before process execution, optional pay-cost decline, replacement cause/controller predicates, Partition source selection, Delay-as-replacement prevention, and effect-driven pending attack cancellation.
- **G-ROCKS-SOURCE-SELECTION-DSL / cross-permanent count-capped source selection (2026-04-29):** resolved by `EffectContext::select_own_sources`, `SelectionKind::SourceMulti`, stable `SourceSelectionRef` bindings, and DSL `select_own_sources` / `trash_selected_sources`.
- **G-MULTI-SELECT-OPP-DP-SUM DP-budget slice (2026-04-29):** resolved by `EffectContext::select_opponent_permanents_by_dp_budget`, `SelectionKind::DpBudget`, and DSL `select_opponent_dp_budget` / `delete_bound_permanents`. Count-capped non-DP sibling shapes remain open where called out above.
- **G-BREEDING-PERMANENT-SELECTION (2026-04-29):** resolved by `EffectContext::select_own_breeding_permanent`, `SelectionKind::BreedingPermanent`, phase-scoped `encode_breeding_select`, and DSL `select_own_breeding_permanent`.
- **G-SELECT-EMPTY-OUTER-TAIL selection regression (2026-04-29):** covered by `empty_select_material_runs_outer_tail_synchronously` and `empty_select_own_sources_runs_outer_tail_synchronously`.
