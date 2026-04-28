# Engine Gaps Tracker

This file accumulates engine mechanics that are missing or incomplete, discovered during archetype implementation. Each entry includes the card that exposed the gap and what engine change is needed.

Last updated: 2026-03-17

## Resolved Gaps

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

20. ~~**Ignore Color Requirement**~~ — RESOLVED 2026-03-14. Added `ModifierType.IGNORE_COLOR_REQUIREMENT` for aura-style bypass in `action_mask.py`. 7 Hudiemon Option scripts use `card._match_color_requirement = False` for self-bypass. BT23-094 also updated.

21. ~~**Security Play API**~~ — RESOLVED 2026-03-17. Added `game.effect_play_from_security(player, card)` helper. `security_attack()` now checks `card._security_played` flag before trashing. EX1-066 updated.

22. ~~**Dynamic Alt-Digi Cost**~~ — RESOLVED 2026-03-17. `digivolve_validator.py` now checks `_alt_digi_cost_fn` callable for dynamic cost calculation. BT24-101 Jupitermon uses security card count.

23. ~~**Digivolve from Hand or Trash**~~ — RESOLVED 2026-03-17 (pre-existing). `include_trash` param already exists on `effect_digivolve_from_hand()`.

24. ~~**Dynamic Security Attack Modifier**~~ — RESOLVED 2026-03-17. Wired `CHANGE_SECURITY_ATTACK` modifier registry into `permanent.security_attack_modifier()`. Fixed 6 scripts with wrong `value_fn` arity.

25. ~~**Optional Attack ("may attack")**~~ — RESOLVED 2026-03-17. Added `ModifierType.MAY_ATTACK` — enables but doesn't force attack (pass remains available). 4 scripts updated.

26. ~~**Digimon-Only Attack Target Restriction**~~ — RESOLVED 2026-03-17. Added `ModifierType.CANNOT_ATTACK_PLAYER` checked in `can_attack_player()`.

27. ~~**is_own_effect in WhenRemoveField Context**~~ — RESOLVED 2026-03-17. Added `is_own_effect`/`is_opponent_effect` to WhenRemoveField, OnRemovedField, WhenPermanentWouldBeDeleted contexts.

28. ~~**Conditional Color Requirement Bypass**~~ — RESOLVED 2026-03-17. Added `_match_color_requirement_fn` callable support to `CardSource.match_color_requirement` property. 4 scripts updated.

29. ~~**Deletion Observer Recursion Guard**~~ — RESOLVED 2026-03-17. Added depth limit (8) to `execute_deletion_effects()` to prevent RecursionError from token chain loops (Puppets vs TS Olympos).

## Remaining Gaps

### ~~Inherited Aura Keyword Grants~~ — RESOLVED 2026-04-12
- **Discovered in:** BT11-042 Angewomon fix-card review (2026-04-12)
- **Card(s):** BT11-042 Angewomon (Blocker aura), BT20-019 Jesmon (X Antibody) (Piercing aura while Jesmon GX), plus other cards using `is_inherited_effect + _applies_to_all_own_digimon + keyword`.
- **What was broken:** `permanent.has_keyword()` aura scan only checked `_applies_to_all_own_digimon` effects on other perms' **non-inherited top-card** effects. Inherited aura keyword effects (below-the-line on a card that is either the current top card or a digivolution source below another Digimon) were silently ignored — while the equivalent DP aura path (`_get_aura_dp_modifier`) already scanned inherited effects in other perms' `card_sources[:-1]`.
- **Resolution:** Extended `has_keyword()` in `permanent.py` to (a) scan inherited aura effects from other perms' `card_sources[:-1]` (mirroring `_get_aura_dp_modifier`); (b) scan ALL aura effects (inherited and non-inherited) on other perms' top cards; (c) scan the self permanent's top card for inherited aura effects targeting self (so BT11-042 Angewomon's aura applies to herself via the `_keyword_permanent_condition` filter). No new script APIs required — existing `_applies_to_all_own_digimon` + `_keyword_permanent_condition` pattern now works for inherited auras as documented.

### Digivolve from Hand or Trash — RESOLVED 2026-03-17 (pre-existing)
- **Resolution:** `effect_digivolve_from_hand()` already has `include_trash` parameter (effects.py:454). No engine change needed.

### Activate Another Card's When Digivolving Effect
- **Discovered in:** Jesmon (2026-03-17)
- **Card(s):** BT10-112 Jesmon GX, BT10-110 Seiken Meppa
- **Effect text:** "Activate 1 of that card's [When Digivolving] effects as an effect of this Digimon."
- **What's missing:** No engine helper to enumerate a card's WD effects and execute a player-selected one.
- **Workaround:** Both BT10-112 and BT10-110 manually iterate `card.effect_list()` and present branch selection. Functional but could benefit from a `game.effect_activate_card_effect()` helper.

### ~~When Attacking Selection Phase Override~~ — RESOLVED 2026-04-02
- **Discovered in:** BT24-024 Submarimon fix-card review
- **Card(s):** All cards with [When Attacking] effects that use selection-based APIs (effect_play_from_zone, effect_select_opponent_permanent, etc.)
- **What was broken:** `declare_attack()` in combat.py continued to counter/block/security after `execute_effects(OnUseAttack)` without checking if effects created a pending selection. The selection phase was overwritten by counter timing.
- **Resolution:** Added park-and-resume pattern: `declare_attack` checks for `pending_selection` after WA effects fire and returns early; `_decode_selection` calls `_maybe_resume_combat_after_wa_selection()` to continue the attack flow after all selections resolve.

### ~~Dynamic Security Attack Modifier~~ — RESOLVED 2026-03-17
- **Resolution:** Wired `ModifierType.CHANGE_SECURITY_ATTACK` into `permanent.security_attack_modifier()` via registry query. BT10-112 uses `_DynamicSAEffect` subclass with `@property` for computed count. Fixed 6 scripts with wrong `value_fn` arity (`lambda: -1` → `lambda cur, t, c: cur - 1`): BT10-042, BT15-084, BT23-094, BT24-071, EX6-022.

### ~~Optional Attack ("may attack")~~ — RESOLVED 2026-03-17, EXTENDED 2026-04-04
- **Resolution:** Added `ModifierType.MAY_ATTACK` semantic marker. Unlike `FORCE_ATTACK`, `MAY_ATTACK` does NOT trigger the forced attackers block in `action_mask.py` — pass (action 62) remains available. Scripts grant Rush + unsuspend alongside `MAY_ATTACK`. Updated 4 scripts: BT24-085, BT24-037, BT24-082, BT24-051.
- **Extension (2026-04-04):** MAY_ATTACK now works at end of turn: `_has_end_of_turn_keywords()` checks MAY_ATTACK, `EndOfTurnAction` action mask offers attack actions (Digimon + player), `_decode_end_of_turn_action` handles SECURITY_TARGET. Also added deferred end-phase completion (`_end_phase_deferred`) for OnEndTurn effects that create pending selections.

### ~~Digimon-Only Attack Target Restriction~~ — RESOLVED 2026-03-17
- **Resolution:** Added `ModifierType.CANNOT_ATTACK_PLAYER` checked in `permanent.can_attack_player()` via modifier registry. BT24-051 Merukimon registers it in the "attack your opponent's Digimon" callback.

### ~~is_own_effect in WhenRemoveField Context~~ — RESOLVED 2026-03-17
- **Resolution:** Added `is_own_effect` and `is_opponent_effect` booleans to `WhenPermanentWouldBeDeleted`, `WhenRemoveField`, and `OnRemovedField` timing contexts in `player.py`. Derived from existing `is_opponent_effect` parameter on `delete_permanent()`. BT24-037 Silphymon updated to use clean `is_own_effect` check instead of `removal_cause` heuristic. BT20-059 Gankoomon was already properly implemented (not affected).

### ~~Conditional Color Requirement Bypass~~ — RESOLVED 2026-03-17
- **Resolution:** Added `_match_color_requirement_fn` callable support to `CardSource.match_color_requirement` property. Dynamic fn is checked first, falls through to static `_match_color_requirement`. Updated 4 scripts: BT24-091 (TS trait check), BT22-099 (CS trait check), ST20-15 (face-up IoA check), BT10-110 (Royal Knight check).

### ~~DigiXros~~ — RESOLVED 2026-03-15
- **Card(s):** 60 cards across BT10-BT24, EX3-EX10, P sets
- **Resolution:** Engine natively supports DigiXros/Assembly: `DigiXrosCost` data model, `parse_digixros_req()` parser (all 60 cards), `digixros_validator.py` for material matching, play intercept → `SelectMaterial` loop → `_execute_digixros_play()`, field materials fire `WhenRemoveField` with `removal_cause='digixros'`, `digixros_count` in `OnEnterFieldAnyone` context.

### Deletion Observer Optionality Not Exposed to Agent
- **Discovered in:** Chaos Control (2026-04-10)
- **Card(s):** EX1-066 — Analog Youth, ST6-14 — Matt Ishida
- **Effect text:** "you may suspend this Tamer" / "you may suspend this Tamer to gain 1 memory"
- **What's missing:** `_fire_deletion_observers` (game/__init__.py:1128) auto-fires effects when conditions pass, ignoring `is_optional`. The DCGO `ActivateClass` offers the player a decline choice (`canNoSelect: true`) before the coroutine runs. In the Python engine, "you may" effects fire automatically with no agent choice to decline.
- **Suggested change:** When `effect.is_optional` is True, create a branch selection (accept/decline) before calling `on_process_callback`. This would expose the choice to the RL action space.
- **Workaround:** Scripts use condition gates (e.g., `perm.is_suspended`) that prevent re-activation, effectively limiting to once per event. The auto-fire behavior is functionally correct but removes the agent's ability to strategically decline (e.g., keeping tamer unsuspended for a later, more valuable deletion).

### Digivolution-Stack Inherited Triggered-Effect Dispatch (Rust Engine)  [G-INHERITED-DISPATCH]
- **Discovered in:** Medusamon archetype, BT21-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine (`code/digimon-engine/`) only. Python engine resolves inherited effects via `card_sources[:-1]` scanning in `_collect_triggered_effects`.
- **Card(s):** BT21-008 Elizamon — inherited `[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, gain 1 memory.` Almost all Lv3+ Digimon in this archetype have a similar inherited triggered effect; this gap blocks the inherited half of every one of them.
- **Effect text:** any DSL clause with `scope: inherited` + a triggered timing.
- **What's missing:** `enqueue_from_permanent` in `code/digimon-engine/src/effect_queue.rs` only collects effects from the **top card** of a permanent. It already handles two sideways-inheritance cases (Phase 8 Task 4: `linked_cards`; Phase 8 Task 5: `OptionState::Training`), but it does NOT iterate `card_sources[0..n-1]` (the digivolution stack below the top card) for `effect.inherited = true` triggered effects. Cards compiled with `scope: inherited` set `effect.inherited = true`, but no dispatch path fires them when this card is in someone else's digivolution stack.
- **Affected cards:** every YAML card with `scope: inherited` and a triggered timing.
- **Suggested change:** in `enqueue_from_permanent` after the top-card scan, iterate `perm.card_sources[0..len-1]`. For each source, call `effects_for_card` and collect effects where `effect.inherited && timing_flag_matches(effect, timing)`. Attribute the queued effect to the hosting permanent's controller and `source_permanent`, with `source_card` pointing at the digivolution source's card handle (same pattern as the linked-cards branch).
- **Workaround:** None — BLOCKED.

### `max_per_turn` (Once-Per-Turn) Not Enforced for Triggered Effects  [G-OPT-TRIGGERED]
- **Discovered in:** Medusamon archetype, EX11-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-008 Elizamon (inherited OPT clause); applies to every DSL card with `once_per_turn: true` on a triggered clause.
- **Effect text:** any clause that combines `[Once Per Turn]` with a non-Main triggered timing (`OnLoseSecurity`, `WhenAttacking`, `OnPlay`, `OnDigivolving`, etc.).
- **What's missing:** `once_per_turn: true` in YAML correctly lowers to `Effect::max_per_turn = 1`, but `run_queued_effect_inner` in `effect_queue.rs` does NOT consult `max_per_turn` when dispatching triggered effects through the queue. OPT is enforced only for activated `Main*` effects in `game_actions.rs`. Triggered effects with OPT can therefore fire more than once per turn.
- **Suggested change:** in the queue-drain path, before invoking each queued effect's process closure, consult `Permanent::activation_count(source_card, slot) >= effect.max_per_turn` (already tracked) and skip if exceeded; call `Permanent::record_activation` after a successful invocation.
- **Workaround:** None — BLOCKED for tests; cards still partially work (the effect just over-fires).

### `EffectTiming::OnMove` Missing  [G-ON-MOVE]
- **Discovered in:** Medusamon archetype, EX11-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine + DSL (hybrid; see `qa/dsl-vocab-gaps.md` for DSL half).
- **Card(s):** EX11-008 Elizamon — `[When Moving] [On Play]` shared body; BT16-082 Ukkomon — entire [Your Turn][OPT] triggered effect (observer in battle area watches any own Digimon move from breeding). Other archetypes will surface this for cards with "[When one of your Digimon moves from the breeding area]" observer triggers.
- **Effect text (EX11-008):** "[When Moving] 1 of your Digimon ... gains <Raid> and +3000 DP for the turn."
- **Effect text (BT16-082):** "[Your Turn][Once Per Turn] When one of your Digimon moves from the breeding area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon card or Tamer card among them to the hand. Return the rest to the bottom of the deck. Then, you may hatch in your breeding area."
- **What's missing:** `EffectTiming::OnMove` variant + dispatch hook in `game_actions::move_from_breeding()`. DCGO maps to `EffectTiming.OnMove`; Rust has no equivalent. The closest existing variant `OnHatch` fires when an egg moves digitama→breeding — a different event. BT16-082 is a battle-area observer card (not the moving card itself); the OnMove dispatch needs to fire `enqueue_triggered` over the controller's battle area (analogous to `OnEnterFieldAnyone` fan-out) so observer permanents like Ukkomon see the event.
- **Suggested change:** (1) Add `EffectTiming::OnMove` to `src/enums.rs`. (2) In `game_actions::move_from_breeding`, after the permanent moves to battle_area: fire `enqueue_triggered(EffectTiming::OnMove, TriggerSource::PlayerBattleArea(player_id))` so all observer permanents in the controller's battle area see the event. (3) Add `CompiledTiming::OnMoveFromBreeding` to `digimon-dsl/src/compiled.rs` and map it in `timing_map.rs`.
- **Workaround:** EX11-008 — handled only the `[On Play]` half in YAML. BT16-082 — structural stub with `on_play` timing + raw_rust no-op (`bt16_082_on_move_noop`).

### `dp_lte` Predicate Compiled but Not Evaluated in `eval_card_fields`  [G-DP-LTE-PREDICATE]
- **Discovered in:** Medusamon archetype, BT21-015 Cyclonemon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-015 Cyclonemon — `[On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 4000 DP or less.`
- **Effect text:** any DSL clause whose `select_*` filter uses `dp_lte: N` to constrain valid targets.
- **What's missing:** `dp_lte` parses and lowers to a `CompiledPredicate` variant, but `eval_card_fields` in `predicate.rs` does not evaluate it — the predicate evaluates as ALWAYS-TRUE for any target's `card_fields`. This means the 4000 DP cap is not enforced at selection time; ineligible targets appear in `valid_action_ids`. Two BT21-015 tests are `#[ignore]`'d pending the fix (`bt21_015_on_play_no_selection_when_no_eligible_target` and `bt21_015_on_play_filters_ineligible_targets_correctly`); boundary-inclusion at exactly 4000 is still asserted via the eligible-target tests that DO pass.
- **Suggested change:** add a `dp_lte` (and presumably `dp_gte`, `dp_eq`) match arm in `eval_card_fields` that reads the target's printed DP from card metadata (or the live effective DP — whichever the predicate semantics intend) and applies the comparison.
- **Workaround:** None — BLOCKED for negative-case tests; positive-case tests still pass because the engine over-permissively accepts eligible targets.

### `event_target_owner` Predicate Missing  [G-EVENT-TARGET-OWNER]
- **Discovered in:** Medusamon archetype, BT24-018 Styracomon (and BT21-029 Medusamon clause-d-deletion-arm) DSL implementations (2026-04-27)
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** BT24-018 Styracomon — replacement clause "When any of your [Reptile] or [Dragonkin] would leave the battle area"; BT21-029 Medusamon — deletion-arm of the All-Turns token-spawn trigger.
- **What's missing:** No `event_target_owner` (or equivalent) predicate to gate triggered/replacement clauses by which player controls the event-target permanent. Affects any clause whose printed text says "your X" or "your opponent's X" in the trigger condition.
- **Suggested change:** add `event_target_owner: you | opponent` (or `event_target_is_yours: bool`) to `CompiledPredicate` AND wire `eval_predicate` to read the trigger context's target permanent's controller.
- **Workaround:** None — replacement/trigger fires for both players' permanents until closed.

### `dp_lte` / `dp_gte` Predicates Not Evaluated for Permanents  [G-PRED-DP-LTE / G-PREDICATE-DP-FILTER / G-SELECT-OPP-FILTER — same root cause]
- **Discovered in:** Medusamon archetype, BT21-015 Cyclonemon (Batch 2) + BT24-017 Medusamon + BT21-029 Medusamon (Batch 3) (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-015 (delete ≤4000 DP), BT24-017 (lowest DP), BT21-029 (lowest DP), and most cards with DP-bounded delete/select.
- **What's missing:** `dp_lte` / `dp_gte` parse and lower into `CompiledPredicate` but `eval_permanent_fields` (in `dsl_cards/predicate.rs`) doesn't read the target's DP and apply the comparator. Selection prompts therefore include ineligible high-DP targets.
- **Suggested change:** add `dp_lte` / `dp_gte` arms to `eval_permanent_fields` that read printed DP from card metadata (or `permanent.dp()` for live DP — pin the semantics first).
- **Note:** Three card-discovery names (G-PRED-DP-LTE, G-PREDICATE-DP-FILTER, G-SELECT-OPP-FILTER, G-DP-LTE-PREDICATE) all refer to this same root cause; consolidating under G-PRED-DP-LTE.

### `[All Turns]` (Both-Player) Filter on Triggered Clauses  [G-ALL-TURNS-FILTER]
- **Discovered in:** Medusamon archetype, BT24-018 Styracomon (2026-04-27).
- **Scope:** DSL.
- **Card(s):** BT24-018, BT21-029, BT24-016, BT21-025 — every card with `[All Turns]` triggered clauses.
- **What's missing:** `active_when: { all_turns: true }` parses but the predicate evaluator may not actually allow firing on the opponent's turn (uncertain — needs verification). Tests for opp-turn triggers are #[ignore]'d pending verification.
- **Workaround:** Use `active_when: { all_turns: true }` and confirm via behavioral test on opp's turn.

### `trash_security_card` Verb (Non-Top Security) Missing  [G-TRASH-SELECTED-SECURITY]
- **Discovered in:** Medusamon archetype, BT24-018 Styracomon (2026-04-27).
- **Scope:** DSL + engine (hybrid).
- **Card(s):** BT24-018 — "[When Digivolving] You may trash any 1 of your opponent's security cards."
- **What's missing:** `select_security` can bind a target index but no DSL verb consumes that binding to actually trash the chosen card. Only `trash_top_security` exists. The engine likely has the primitive (security indexing already supported elsewhere); just no DSL bridge.
- **Workaround:** `raw_rust:` escape hatch.

### Trash → Deck-Bottom Move (Without Reveal Phase)  [G-ZONE-TRASH-TO-DECK]
- **Discovered in:** Medusamon archetype, BT24-017 Medusamon (Batch 3, 2026-04-27).
- **Scope:** DSL + engine (hybrid).
- **Card(s):** BT24-017 (return 2 trash to bottom of deck), BT21-029-related, EX11-012 (return 1 trash to bottom).
- **What's missing:** A DSL verb / `EffectContext` API for moving a chosen trash card to the bottom of the owner's main deck. Existing `return_to_deck_from_reveal` works for cards in the reveal zone, not trash.
- **Workaround:** EX11-012 implementer added a `raw_rust: ex11_012_return_trash_to_deck_bottom` (6-line bridge in `src/cards/raw_rust/mod.rs`). Generalizing it as a first-class DSL verb is the proper fix.

### Resolved during Medusamon run

- **`PlayFromSecurity` dispatch in security-skill timing** — RESOLVED 2026-04-27 in BT21-015 implementation. `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs` now dispatches to `play_pending_security()` when `ctx.game.pending_security.is_some()` (security-skill replay path) and `play_from_security(player)` otherwise. Affects every DSL card with a `[Security]` clause that uses `play_from_security: {}` (BT21-015, BT5-093, BT9-092, BT22-084, ...).
- **Declarative inherited `grant_keyword` not visible to `has_keyword`** — RESOLVED 2026-04-27 in BT24-011 implementation. `code/digimon-engine/src/game.rs::has_keyword` now scans `perm.card_sources` for `declarative && inherited` effects whose name matches the `Grant <Keyword>` convention set by `lower_grant_keyword`. Companion change in `code/digimon-engine/src/debug_runner.rs::card_data_from_compiled` populates `CardData.keywords` from FaceUp `GrantKeyword` clauses so own-printed keywords surface without dispatch.
- **`Progress` keyword not in `lookup_keyword`** — RESOLVED 2026-04-27 in BT21-029 / EX11-012 implementations. `src/dsl_cards/modifier_map.rs` now maps `"Progress" => Keyword::Progress`.
- **`SelectOwnPermanent` / `SelectOpponentPermanent` ignored predicate filters (accept-all)** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/step/selections.rs::install_select_*_permanent` now pre-filters candidates with `eval_predicate` and threads the filter closure to the underlying `select_*_permanent` API.
- **Replacement clause subject-guard missing (would-leave fires for any permanent)** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/lower_replacement.rs` now checks `subject_matches` so `WhenWouldLeaveBattleArea` only fires when the carrier itself is the leaving permanent.
- **`CompiledCardKind::Token` missing from predicate match** — RESOLVED 2026-04-27 in EX11-012 implementation. `src/dsl_cards/predicate.rs` now handles `CompiledCardKind::Token`, enabling `kind: token` filters (e.g. "delete 1 Token" cost).
- **Petrification token name case-sensitivity bug** — RESOLVED 2026-04-27 in BT21-029 implementation. `TokenRegistry` is keyed lowercase; YAML `token_name:` values must use lowercase (`petrification` not `Petrification`). EX11-012's example version had the same bug; production version corrected.

### `on_digivolve` Trigger Context Missing Newly-Digivolved Permanent Reference  [G-ON-DIGIVOLVE-TRAIT-FILTER]
- **Discovered in:** Medusamon archetype, BT24-082 Owen Dreadnought DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT24-082 Owen Dreadnought — "[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn."
- **Effect text:** "When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon … that Digimon gets +3000 DP"
- **What's missing:** `on_digivolve` fires via `TriggerSource::PlayerBattleArea(pid)` in `game_actions.rs`, which sets every permanent's effect as an observer. When constructing the `TriggerContext`, `target_permanent` is set to the observer permanent (the tamer itself), NOT the permanent that just digivolved. Therefore: (a) a trait filter on the newly-digivolved card ("digivolve INTO a Reptile/Dragonkin") cannot be expressed in the condition predicate, and (b) the DP-modifier target ("that Digimon") cannot be bound to the newly-digivolved card.
- **Suggested change:** Add a `digivolve_target: Option<PermanentHandle>` field to `TriggerSource::PlayerBattleArea` (or a sibling `DigivolveTarget` variant). Populate it in `fire_on_digivolve` with the permanent that just completed digivolution. Thread it through to `TriggerContext::target_permanent` for each observer's effect dispatch, or add a distinct `digivolve_target` field to `TriggerContext` so observer effects can reference both "the observer" and "the card that digivolved".
- **Workaround:** `any_permanent` condition over own battle area with `trait_has: Reptile/Dragonkin` (over-fires if a matching ally is on board but a non-matching Digimon digivolved). `select_own_permanent` prompt for DP modifier target (player picks instead of auto-targeting). Two tests `#[ignore]`'d.

### `OnEnterFieldAnyone` Observer Context Missing Entering-Permanent Reference  [G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER]
- **Discovered in:** Medusamon archetype, EX11-054 Owen Dreadnought DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-054 Owen Dreadnought — "[All Turns] When your Digimon are played or digivolve, if any of them have the [Reptile] or [Dragonkin] trait, by suspending this Tamer, <Draw 1>. After, 1 of your Digimon with <Progress> gets +3000 DP."
- **Effect text:** "When your Digimon are played … if any of them have the [Reptile] or [Dragonkin] trait"
- **What's missing:** `OnEnterFieldAnyone` fires via `TriggerSource::PlayerBattleArea(pid)` in `game_actions.rs`. `trigger_context_for_source` for this variant iterates every permanent in `pid`'s battle area and sets `target_permanent = source_permanent` (the OBSERVER). The entering permanent's handle is never threaded into `TriggerContext`. An observer like Owen Dreadnought therefore cannot inspect the traits of the card that just entered — `event_target_trait_has` evaluates Owen's own traits, not the entrant's.
- **Related gap:** G-ON-DIGIVOLVE-TRAIT-FILTER (same limitation for `on_digivolve`). Both share the same root cause: the trigger source variant doesn't carry the triggering permanent's handle.
- **Suggested change:** Add `entering_permanent: Option<PermanentHandle>` to `TriggerContext` (alongside existing `target_permanent`). Populate it in `game_actions.rs::broadcast_on_enter_field_anyone` (and the digivolve broadcast) with the handle of the card that just entered/digivolved. Add a matching `entering_permanent_trait_has` DSL BoolPredicate leaf in `predicate.rs` that reads `ctx.trigger_context.entering_permanent`.
- **Workaround:** `kind: raw_rust` no-op placeholder (`ex11_054_all_turns_noop`). See `qa/dsl-vocab-gaps.md` entry `G-ENTERING-PERMANENT-TRAIT`.

### `count_lte` Aggregate Predicate (Non-Security) Not Evaluated  [G-COUNT-LTE-EVAL]
- **Discovered in:** Medusamon archetype, BT21-017 Dimetromon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT21-017 Dimetromon — "[When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Owen Dreadnought] from your hand without paying the cost." Also BT22-084 Nokia Shiramine (Start of Your Main Phase condition) and any card whose clause-level `condition:` uses `count_lte` with a non-security zone filter.
- **Effect text:** "If you have 1 or fewer Tamers" — a `count_lte` aggregate gate on the controller's battle area.
- **What's missing:** `count_lte` (and presumably `count_gte`, `count_eq`) aggregate predicates are parsed and lowered into `CompiledPredicate.count_lte` in `code/digimon-engine/src/dsl_cards/predicate.rs`, but `eval_predicate_with_bindings` has no match arm for `count_lte` / `count_gte`. Only the security-specific variants (`security_count_lte`, `security_count_gte`) are evaluated. As a result, any clause whose `condition:` uses `count_lte { filter: { zone: [battle_area], ... }, n: N }` silently evaluates to TRUE regardless of actual battlefield count, making the gate permanently open.
- **Affected cards:** BT21-017 (tamer ≤ 1 gate on WhenDigivolving), BT22-084 Nokia Shiramine (count_lte gate on StartOfMainPhase), and any other card with a non-security zone count condition.
- **Suggested change:** Add `count_lte` / `count_gte` / `count_eq` match arms to `eval_predicate_with_bindings` in `predicate.rs`. The implementation should: (1) resolve the filter's `zone` + `kind` + optional predicate to a set of permanents (reusing the existing zone-scan logic from `install_select_*_permanent`), (2) count matching permanents, (3) compare against `n`. An `eval_zone_count` helper factored out of the existing `count_lte` case in `eval_predicate_with_bindings` (once added) would keep the three comparison operators DRY.
- **Workaround:** None — BLOCKED for negative-case condition tests. Positive-case tests pass because the gate is always open.

### `GameEvent::Digivolve` Not Emitted  [G-GAME-EVENT-DIGIVOLVE]
- **Discovered in:** Medusamon archetype, EX11-054 Owen Dreadnought DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-054 Owen Dreadnought (digivolve half of [All Turns] trigger); any card that would use the event log to detect digivolves.
- **Effect text:** "When your Digimon … digivolve, if any of them have the [Reptile] or [Dragonkin] trait …"
- **What's missing:** `GameEvent::Digivolve` is defined in `events.rs` as "for future wiring — not emitted yet." Even if an observer could use raw_rust to read `ctx.game.events`, the digivolve-detection path is unavailable. Blocks raw_rust workarounds for G-ON-DIGIVOLVE-TRAIT-FILTER that try to infer "which permanent just digivolved" via the event log.
- **Suggested change:** Emit `GameEvent::Digivolve { player, permanent: PermanentHandle }` inside the digivolve execution path (wherever `fire_on_digivolve` is called). This unblocks event-log-based raw_rust workarounds until the full TriggerContext fix lands.
- **Workaround:** None — raw_rust event-log detection blocked until emission is wired.

<!-- Entry template:
### {Gap Title}
- **Discovered in:** {archetype name} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{relevant text}"
- **What's missing:** {description of engine capability needed}
- **Suggested change:** {brief proposal}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
-->
