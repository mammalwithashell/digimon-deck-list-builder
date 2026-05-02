# Engine Gaps Tracker

This file accumulates engine mechanics that are missing or incomplete, discovered during archetype implementation. Each entry includes the card that exposed the gap and what engine change is needed.

Last updated: 2026-04-30

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
- **Discovered in:** Jesmon (2026-03-17); Puppets/Nyabootmon assessment (2026-04-28)
- **Card(s):** BT10-112 Jesmon GX, BT10-110 Seiken Meppa, BT22-042 Nyabootmon
- **Effect text:** BT10-112 / BT10-110: "Activate 1 of that card's [When Digivolving] effects as an effect of this Digimon." BT22-042: "[All Turns] [Once Per Turn] When any of your other Digimon are deleted, you may activate 1 of this Digimon's [When Digivolving] effects."
- **What's missing:** No engine helper to enumerate a card's available [When Digivolving] effects and execute a player-selected one from another trigger. Nyabootmon needs this for its own top card, while Jesmon-style effects need it for another card/source.
- **Suggested change:** Add a helper that enumerates [When Digivolving] effects on a specified source card/permanent, exposes a legal branch choice if multiple effects are available, and executes the selected effect using the correct source permanent/card attribution. The choice must flow through pending selection/action-mask machinery.
- **Workaround:** BT10-112 and BT10-110 manually iterate `card.effect_list()` and present branch selection. BT22-042 has no authored Rust implementation yet.

### Puppet-Scoped Overclock Sacrifice Filter [G-OVERCLOCK-TRAIT-FILTER]
- **Discovered in:** Puppets/Nyabootmon assessment (2026-04-28)
- **Scope:** Rust engine action mask and Overclock activation.
- **Card(s):** BT22-042 Nyabootmon, EX7-027 Chaperomon, EX7-030 Cendrillmon, EX11-024 Cendrillmon, BT22-036 Kazuchimon, plus other cards with `<Overclock ([Puppet] Trait)>`.
- **Effect text:** "<Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)"
- **What's missing:** Current Overclock eligibility treats any other Digimon as a valid sacrifice. Puppets require the sacrifice to be either a token or another [Puppet] trait Digimon, so the action mask can expose illegal non-Puppet sacrifices when mixed allies are present.
- **Suggested change:** Parameterize Overclock with a sacrifice predicate, or let the card effect provide one. Use the same predicate in `has_overclock_sacrifice`, end-of-turn action masking, and `activate_overclock` pending choices.
- **Workaround:** None. The current mask/activation path is too broad for Puppet Overclock.

### Familiar Token On Deletion Effect Missing [G-FAMILIAR-TOKEN-ON-DELETION]
- **Discovered in:** Puppets/Nyabootmon assessment (2026-04-28)
- **Scope:** Rust token card effects.
- **Card(s):** TOKEN_FAMILIAR; generated by P-165 ShoeShoemon, EX7-030 Cendrillmon, EX11-024 Cendrillmon, ST19-12 Cendrillmon, and related Puppet effects.
- **Effect text:** "Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn."
- **What's missing:** The token registry has Familiar token stats and `EffectContext::play_token` can create tokens, but `src/cards/tokens/familiar.rs` currently returns no effects. Deleting a Familiar token therefore does not create the required opponent-target selection or apply the -3000 DP modifier.
- **Suggested change:** Implement Familiar Token's mandatory `OnDeletion` effect. When opponent Digimon are available, expose the target choice through the existing selection path and apply -3000 DP for the turn.
- **Workaround:** None. Token creation works, but the printed deletion text is absent.

### Event-Gated Delay Activation Windows [G-DELAY-EVENT-GATED]
- **Discovered in:** Puppets/Nyabootmon assessment (2026-04-28)
- **Scope:** Rust engine delayed-option state, action mask, and DSL lowering.
- **Card(s):** BT22-098 Unique Emblem: Fable Waltz, P-229 Unique Emblem: Narrative Ronde.
- **Effect text:** BT22-098: "[Your Turn] When any of your [Arisa Kinosaki] suspend, <Delay> ... 1 of your [Puppet] trait Digimon may digivolve into a [Puppet] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3." P-229: "[Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay> ... 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3."
- **What's missing:** Delay lowering only supports end-of-turn style trigger variants, and non-end-of-turn delay triggers fall back to `EndOfYourNextTurn`. There is no faithful engine path for a placed option to become activatable on an arbitrary later event, such as Arisa suspending or Mirai being played, while still enforcing "Delay cannot activate the turn this card was placed."
- **Suggested change:** Extend delayed-option state with an event trigger and predicate, enqueue/expose a Delay activation action when that event fires, and add DSL timing mappings for `on_suspend`/`on_ally_played` event-gated Delay clauses.
- **Workaround:** None without approximating the activation timing.

### Cost and Replacement Framework

Resolved by Group 3:
- BT13-007 King Drasil_7D6 and ST21-13 Matt Ishida & T.K. Takaishi can both reduce AD1-025 Omnimon before memory is paid because AD1-025 has both `[Royal Knight]` and `[ADVENTURE]`.
- Triggered effect costs may install pending selections and resume process only after cost payment.
- Optional cost decline skips process without hidden auto-selection.
- Replacement predicates can inspect cause, source controller, and subject controller.
- Partition source requirements are enforced before prevention.
- Delay options can pay themselves as replacement costs and prevent deletion.
- Effects can end a pending attack after a printed cost resolves.

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
- **Status:** Fixed for battle-area permanent dispatch on 2026-04-29. `enqueue_from_permanent` now preserves top-card / linked / Training dispatch, then scans below-top `card_sources` and queues only matching `effect.inherited == true` effects with `source_permanent` set to the carrier and `source_card` set to the inherited source card.
- **Affected cards:** YAML cards with `scope: inherited` and a triggered timing can now fire from below the top card when the relevant event is already dispatched to the carrier permanent's battle-area observer path.
- **Regression coverage:** `bt21_008_inherited_positive_fires_when_source_under_carrier_your_turn` and `buried_non_inherited_triggered_effect_does_not_fire_from_source_position`.
- **Remaining limits:** This does not add every event fire site. Group 4 added source-trash context for direct `EffectContext::trash_card_source` / `trash_top_source` helpers and effect-driven security-removal fan-out/resume for direct security stack moves. Lower-source trash from some older zone-return paths and breeding-area dispatch remain separate follow-ups. Group 2 closed the shared source, DP-budget, breeding-permanent, and empty-tail selection primitives on 2026-04-29.

### `max_per_turn` (Once-Per-Turn) Not Enforced for Triggered Effects  [G-OPT-TRIGGERED]
- **Discovered in:** Medusamon archetype, EX11-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-008 Elizamon (inherited OPT clause); applies to every DSL card with `once_per_turn: true` on a triggered clause.
- **Effect text:** any clause that combines `[Once Per Turn]` with a non-Main triggered timing (`OnLoseSecurity`, `WhenAttacking`, `OnPlay`, `OnDigivolving`, etc.).
- **Status:** Fixed for permanent-backed queued triggered effects on 2026-04-29. `run_queued_effect_inner` now checks `Permanent::activation_count(source_card, slot) >= effect.max_per_turn` before processing and records activation before `process`, matching the existing activated field-main timing.
- **Regression coverage:** `bt21_008_inherited_opt_blocks_second_trigger_same_turn`.
- **Remaining limits:** This only enforces the existing queued-effect activation counter. It does not add optional prompt/action-space handling or breeding dispatch. Group 4 separately covered direct source-trash helper context and owner routing.

### `EffectTiming::OnMove` for Breeding-to-Battle Movement  [G-ON-MOVE]
- **Discovered in:** Medusamon archetype, EX11-008 Elizamon DSL implementation (2026-04-27)
- **Scope:** Rust engine + DSL (hybrid; see `qa/dsl-vocab-gaps.md` for DSL half).
- **Card(s):** EX11-008 Elizamon — `[When Moving] [On Play]` shared body; BT16-082 Ukkomon — entire [Your Turn][OPT] triggered effect (observer in battle area watches any own Digimon move from breeding). Other archetypes will surface this for cards with "[When one of your Digimon moves from the breeding area]" observer triggers.
- **Effect text (EX11-008):** "[When Moving] 1 of your Digimon ... gains <Raid> and +3000 DP for the turn."
- **Effect text (BT16-082):** "[Your Turn][Once Per Turn] When one of your Digimon moves from the breeding area to the battle area, reveal the top 3 cards of your deck. Add 1 Digimon card or Tamer card among them to the hand. Return the rest to the bottom of the deck. Then, you may hatch in your breeding area."
- **Status:** Fixed for breeding-to-battle movement on 2026-04-29. `EffectTiming::OnMove`, `Effect::on_move(card)`, DSL `when: on_move`, and `TriggerSource::MovedFromBreeding { player, permanent, card }` now carry the moved battle-area permanent and top/source card after `Game::move_from_breeding` commits. Regression coverage: `on_move_fires_after_breeding_permanent_moves_to_battle`; direct DSL event-context coverage: `on_move_event_target_trait_predicate_matches_moved_permanent` proves `event_target_trait_has` sees the moved permanent/card.
- **Remaining limits:** This does not add generic breeding-area trigger fan-out, extra pending selections, reveal/add-to-hand handling for BT16-082, or the unrelated `OnPlay`/`WhenDigivolving` body work for multi-timing cards.

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
- **Updated 2026-04-29:** Normal battle-area `Game::digivolve_from_hand` now dispatches `OnDigivolve` via `TriggerSource::Digivolved { player, permanent, card }`, and `TriggerContext.event_permanent` / `event_card` identify the just-digivolved permanent and new top card. `event_card_trait_has` is proven against the new top card by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card`, and `target: event_target` binding is proven to affect the just-digivolved permanent by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`. Keep effect-initiated digivolve, DNA digivolve, and breeding-area digivolve as open follow-ups unless separately tested.

### `OnEnterFieldAnyone` Observer Context Missing Entering-Permanent Reference  [G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER]
- **Discovered in:** Medusamon archetype, EX11-054 Owen Dreadnought DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** EX11-054 Owen Dreadnought — "[All Turns] When your Digimon are played or digivolve, if any of them have the [Reptile] or [Dragonkin] trait, by suspending this Tamer, <Draw 1>. After, 1 of your Digimon with <Progress> gets +3000 DP."
- **Effect text:** "When your Digimon are played … if any of them have the [Reptile] or [Dragonkin] trait"
- **What's missing:** `OnEnterFieldAnyone` fires via `TriggerSource::PlayerBattleArea(pid)` in `game_actions.rs`. `trigger_context_for_source` for this variant iterates every permanent in `pid`'s battle area and sets `target_permanent = source_permanent` (the OBSERVER). The entering permanent's handle is never threaded into `TriggerContext`. An observer like Owen Dreadnought therefore cannot inspect the traits of the card that just entered — `event_target_trait_has` evaluates Owen's own traits, not the entrant's.
- **Related gap:** G-ON-DIGIVOLVE-TRAIT-FILTER (same limitation for `on_digivolve`). Both share the same root cause: the trigger source variant doesn't carry the triggering permanent's handle.
- **Suggested change:** Add `entering_permanent: Option<PermanentHandle>` to `TriggerContext` (alongside existing `target_permanent`). Populate it in `game_actions.rs::broadcast_on_enter_field_anyone` (and the digivolve broadcast) with the handle of the card that just entered/digivolved. Add a matching `entering_permanent_trait_has` DSL BoolPredicate leaf in `predicate.rs` that reads `ctx.trigger_context.entering_permanent`.
- **Workaround:** `kind: raw_rust` no-op placeholder (`ex11_054_all_turns_noop`). See `qa/dsl-vocab-gaps.md` entry `G-ENTERING-PERMANENT-TRAIT`.
- **Updated 2026-04-29:** Normal hand-played battle-area permanents now dispatch `OnEnterFieldAnyone` via `TriggerSource::EnteredField { player, permanent, card }`, and `TriggerContext.event_permanent` / `event_card` identify the entering permanent and card. `event_card_trait_has` is proven against the entering card by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_enter_field_anyone_event_card_trait_predicate_matches_entering_card`. Keep effect-created permanents, token play, option placement, play-from-trash context, and breeding-area observer fan-out as open follow-ups unless separately tested.

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
- **Updated 2026-04-29:** `Game::digivolve_from_hand` now emits `GameEvent::Digivolve { player, top_card_id, field_index, from_stack_top }` after stack mutation. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- game_event_digivolve_is_emitted_with_new_top_card_and_field_index`. Effect-initiated digivolve, DNA digivolve, and breeding-area digivolve event-log coverage remain open.

### `EffectTiming::Declarative` Never Fired — Filtered Aura / Grant-Keyword Runtime Gap  [G-DECLARATIVE-KEYWORD]
- **Discovered in:** Medusamon archetype — BT21-029, EX11-012, EX11-054 (grant_keyword), BT5-008 (filtered aura), 2026-04-27
- **Scope:** Rust engine.
- **Card(s):** Any card using `kind: aura` with a non-empty target predicate (filtered aura), or `kind: grant_keyword` with a declarative scope. Specific cards: BT21-029 Medusamon, EX11-012 Medusamon (Progress keyword), BT5-008 Gaossmon (filtered aura +3000 DP to other Gaossmon).
- **Effect text:** "[Your Turn] Your other [Gaossmon] all get +3000 DP." (BT5-008); "[When Digivolving / On Field] <Progress>" (EX11-012 inherited); "SecurityAttack+1" (BT21-029 clause a).
- **What's missing:** `EffectTiming::Declarative` is defined in `enums.rs` (line 204) and used by `Effect::declarative(card)` in `effect.rs`, but it is **never enqueued or fired** anywhere in the engine. No call site in `game_phases.rs`, `game_actions.rs`, `game.rs`, or `effect_queue.rs` calls `enqueue_triggered(EffectTiming::Declarative, ...)`. As a result:
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

### `EffectContext::add_top_security_to_hand` Missing (engine half of G-ADD-TOP-SECURITY-TO-HAND)
- **Discovered in:** Medusamon Batch 8, P-137 Flamedramon DSL implementation (2026-04-27)
- **Card(s):** P-137 Flamedramon — "[Your Turn][Once Per Turn] When this Digimon's attack target is switched, your opponent adds the top card of their security stack to the hand."
- **Effect text:** "opponent adds the top card of their security stack to the hand"
- **What's missing:** `EffectContext` only exposes `trash_top_security(player)` for security removal. There is no `add_top_security_to_hand(player)` method that pops the top security card and places it in the player's hand while firing the standard security-removed event chain (`OnLoseSecurity` via `SecurityRevealed` + `OnOpponentSecurityRemoved` via `PlayerBattleArea`).
- **Suggested change:** Add `pub fn add_top_security_to_hand(&mut self, player: PlayerId) -> bool` to `EffectContext`. Implementation: pop `security.last()`, push to `hand`, fire `EffectTiming::OnLoseSecurity` with `TriggerSource::SecurityRevealed { defender: player, card: card_handle }` and `EffectTiming::OnOpponentSecurityRemoved` with `TriggerSource::PlayerBattleArea(controller)`.
- **Workaround:** `raw_rust: { fn: p_137_opp_adds_top_security_to_hand }` — manually implements the move + event chain in `src/cards/raw_rust/mod.rs`.

### Outer-Tail Steps Lost When Inner `select_hand` Has No Candidates  [G-SELECT-EMPTY-OUTER-TAIL]
- **Discovered in:** Medusamon Batch 8, BT21-024 Cyberdramon side-fix (2026-04-27)
- **Card(s):** BT21-024 Cyberdramon — opponent places hand card as bottom security, then top security trashed.
- **Effect text:** "they place 1 card from their hand as the bottom security card. Then, trash their top security card."
- **What's missing:** When `select_hand` is called inside an `as_selecting_player` body and there are no valid candidates (empty hand), `install_select_hand` returns early without installing a `PendingSelection`. `try_install` still returns `true` (the variant was matched), so `run_steps` returns `RunOutcome::Parked`. `as_selecting_player` propagates `Parked`, and `park_outer_tail` parks subsequent sibling steps in `dsl_outer_tail`. Since no selection was ever installed, the selection callback never fires, and `drain_dsl_outer_tail` is never called — outer-tail steps are permanently lost.
- **Affected pattern:** Any YAML where `as_selecting_player { body: [select_hand, ...] }` is followed by sibling steps, and the opponent may have an empty hand. The sibling steps after `as_selecting_player` are silently skipped in the empty-hand scenario.
- **Suggested change:** When `install_select_hand` detects `valid_action_ids.is_empty()` and `optional=true`, it should run the callback synchronously with a sentinel `NO_SELECTION` index (or call `drain_dsl_outer_tail` directly) rather than just returning. For `optional=false` with an empty hand, the current silent-skip behavior may be acceptable — but `drain_dsl_outer_tail` should still fire.
- **Workaround:** Move subsequent steps that must fire unconditionally INSIDE the `as_selecting_player` body (at the cost of tying them to the selection resolution). Steps after the body that require unconditional execution in the empty-hand case cannot be expressed in the current DSL. The BT21-024 empty-hand test is `#[ignore]`'d with this gap tag.
- **Updated 2026-04-29:** Empty inner selection handling now preserves the outer tail for `select_material` and the new `select_own_sources` path. Covered by `empty_select_material_runs_outer_tail_synchronously` and `empty_select_own_sources_runs_outer_tail_synchronously`. Other legacy selection installers should use the same "no candidates means no park" pattern when they grow empty-candidate tests.

### Multi-Select of Opponent Battle-Area Permanents with Running DP-Sum Cap  [G-MULTI-SELECT-OPP-DP-SUM]
- **Discovered in:** Medusamon Batch 10, LM-021 Agumon - Bond of Bravery DSL implementation (2026-04-28)
- **Scope:** Rust engine + DSL.
- **Card(s):** LM-021 Agumon - Bond of Bravery — "[On Play][When Digivolving] Delete any number of your opponent's Digimon whose total DP adds up to equal or less than this Digimon's DP." Also BT17-018 Gallantmon Crimson Mode — "[On Play][When Digivolving] Delete any number of your opponent's Digimon with total DP equal to or less than this Digimon's DP." Both cards share the same selection mechanic.
- **Effect text (LM-021):** "Delete any number of your opponent's Digimon whose total DP adds up to equal or less than this Digimon's DP."
- **What's missing:** `EffectContext` exposes only single-target selection (`select_opponent_permanent`) and count-capped multi-target selection (`select_count_capped_multi`, which caps by pick count, not by DP sum). There is no primitive for iterative multi-select where each pick reduces a remaining DP budget and the player may stop at any point once they have at least one selection (DCGO: `canEndNotMax: true`, `canTargetConditionByPreSelectedList` with dynamic remainder). The running DP-sum cap requires: (a) tracking cumulative DP of already-selected targets, (b) re-filtering valid candidates after each pick to exclude those whose DP would exceed the remaining budget, and (c) allowing early termination once at least one target is picked. None of these are available in the current selection state machine.
- **Suggested change:** Add a `select_opponent_permanent_dp_sum(description, self_dp, callback)` method to `EffectContext` that: (1) initializes a `remaining_budget = self_dp`; (2) presents a filtered pick from `opponent.battle_area` where `perm.dp <= remaining_budget`; (3) after each pick, subtracts the picked card's DP from `remaining_budget` and repeats if budget > 0 and valid candidates remain; (4) allows the player to stop picking at any point; (5) calls `callback` once on all selected handles. Alternatively, extend `PendingSelection` with a `DpBudget(u32)` variant that the selection engine drains per-pick.
- **Workaround:** `raw_rust: { fn: lm_021_delete_dp_sum }` and `raw_rust: { fn: bt17_018_delete_opp_digimon_dp_budget }` — both fall back to single-pick with a DP <= budget filter. Full multi-pick semantics are deferred until this gap closes.
- **Updated 2026-04-29:** Resolved for opponent battle-area DP-budget selection. `EffectContext::select_opponent_permanents_by_dp_budget` installs `SelectionKind::DpBudget`, filters remaining affordable targets after each pick, and exposes PASS after `min_picks`. DSL `select_opponent_dp_budget` binds the chosen permanents and `delete_bound_permanents` consumes them. Covered by `dp_budget_selection_tracks_remaining_dp_and_allows_pass_after_min`, `dp_budget_selection_finishes_when_no_targets_fit`, `dp_budget_selection_mask_exposes_only_remaining_affordable_targets`, and `dsl_select_dp_budget_deletes_bound_permanents`.

### Cross-Permanent Rocks Source Selection Resolved  [G-ROCKS-SOURCE-SELECTION-DSL]
- **Discovered in:** Rocks / RockClose archetype assessment (2026-04-29 follow-up).
- **Scope:** Rust engine + DSL.
- **Card(s):** EX10-032 Proganomon, EX10-028 Landramon, EX8-070 Zofr Kabus, EX10-036 Magneticdramon, EX10-033 / EX11-044 / EX8-055 Pyramidimon source-trash bodies.
- **Status:** Resolved for the shared selection primitive. `EffectContext::select_own_sources` supports exact-N and up-to-N source choices across own battle-area stacks, binds stable `SourceSelectionRef` values, and DSL `trash_selected_sources` consumes those refs without a fake permanent prompt.
- **Regression coverage:** `source_multi::exact_two_sources_can_be_selected_across_own_battle_area`, `source_multi::up_to_sources_enables_pass_only_after_minimum_is_met`, `source_multi_mask_only_exposes_selecting_players_pending_actions`, `select_own_sources_binds_source_refs_for_trashing`, and `empty_select_own_sources_runs_outer_tail_synchronously`.
- **Remaining limits:** Triggered-body cost ordering, Fragment / Digi-Burst / replacement integration, and card-specific Rocks bodies remain separate gaps.

### `IgnoreColorRequirement` Modifier Not Enforced in Rust Option Action Mask  [G-IGNORE-COLOR-MASK]
- **Discovered in:** Medusamon Batch 11, ST22-08 Offensive Plug-In V DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** ST22-08 Offensive Plug-In V — "While you have a Tamer, you can ignore this card's color requirements." Also any card that would use the `IgnoreColorRequirement` modifier via a flood_gate clause.
- **Effect text:** "While you have a Tamer, you can ignore this card's color requirements."
- **What's missing:** `code/digimon-engine/src/action/mask.rs`'s `option_color_match_available` function (line 598) has the following comment: "Script-level `match_color_requirement=False` and the `IGNORE_COLOR_REQUIREMENT` aura modifier are residual §4.2b work; both are absent here." The `ModifierType::IgnoreColorRequirement` variant exists in `enums.rs` (line 488) and the DSL validator accepts it, but no enforcement hook reads this modifier in the Rust action mask. The Python engine resolved this gap on 2026-03-14, but the Rust engine's action mask never received the equivalent fix.
- **Suggested change:** In `option_color_match_available`, before returning `false`, check whether the card itself carries an `IgnoreColorRequirement` modifier (self-modifier on the card source) or whether any ally permanent has a permanent-level `IgnoreColorRequirement` modifier that applies to this card. If so, return `true`. This matches the Python engine's `_match_color_requirement_fn` pattern and the `ModifierType.IGNORE_COLOR_REQUIREMENT` aura check in `action_mask.py`.
- **Workaround:** None — the `flood_gate` YAML clause with `modifier: IgnoreColorRequirement` compiles correctly but has zero runtime effect because the enforcement is absent.

### `DelayTrigger::StartOfYourNextTurn` Missing — Delay Fires at Start of Turn  [G-DELAY-START-OF-TURN]
- **Discovered in:** Medusamon Batch 12, LM-027 Red Scramble DSL implementation (2026-04-28)
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** LM-027 Red Scramble; LM-030 Green Scramble in BG Imperial — "[Start of Your Turn] If your opponent has a Digimon, ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)" Likely affects other Delay option cards whose activation timing is the controller's next turn START rather than END.
- **Effect text:** "[Start of Your Turn] … ＜Delay＞ …"
- **What's missing:** The engine's `DelayTrigger` enum (in `src/enums.rs`) only has two variants: `EndOfThisTurn` and `EndOfYourNextTurn`. The DSL `kind: delay` lowerer in `src/dsl_cards/lower_delay.rs` maps `"end_of_your_turn"` to `EndOfThisTurn` and everything else to `EndOfYourNextTurn`. Both variants fire at END-of-turn. LM-027's Delay activates at the START of the controller's next turn (DCGO: `EffectTiming.OnStartTurn` with `CanDeclareOptionDelayEffect`). There is no `DelayTrigger::StartOfYourNextTurn` variant, and the DSL `kind: delay` path has no lowering route for start-of-turn firing. The entire Delay clause body is therefore unimplementable with native DSL.
- **Suggested change:** (1) Add `StartOfYourNextTurn` variant to `DelayTrigger` in `src/enums.rs`. (2) Add a `"start_of_your_turn"` token (or `"start_of_next_turn"`) in the DSL timing map (`timing_map.rs`) that lowers to `DelayTrigger::StartOfYourNextTurn`. (3) Wire `StartOfYourNextTurn` firing into the game's start-of-turn hook (`game_phases.rs::begin_turn`): after incrementing `turn_number`, scan all permanents for `Delay` state with `trigger == StartOfYourNextTurn` and fire those. This is symmetric to the end-of-turn Delay drain already implemented.
- **Workaround:** `kind: raw_rust` no-op placeholder (`lm_027_delay_start_of_turn_noop`) preserving the clause-index slot. All Delay behavioral tests are `#[ignore]`'d.

### `EffectContext::add_pending_security_to_hand`  [G-ADD-OPTION-SELF-TO-HAND]
- **Discovered in:** Medusamon Batch 12, LM-027 Red Scramble DSL implementation (2026-04-28). Also previously surfaced by ST22-08 Offensive Plug-In V (Batch 11) and EX6-072 pattern.
- **Scope:** Rust engine + DSL (hybrid).
- **Card(s):** LM-027 Red Scramble — "[Security] … Then, add this card to the hand." Also ST22-08 Offensive Plug-In V and any option card whose Security clause ends with returning itself to hand.
- **Effect text:** "Then, add this card to the hand." — the currently-resolving security option card moves to the controller's hand.
- **Status:** Resolved for the narrow pending-security disposition slice on 2026-05-01. `EffectContext::add_pending_security_to_hand()` consumes `Game.pending_security` and pushes the revealed card to the defender/controller hand so the security dispose phase cannot trash it. DSL `add_this_option_to_hand: {}` lowers to the method. Legacy raw-rust shims now delegate to the method; new scripts should use the native step.
- **Coverage:** `debug_runner_dsl::security_dsl_adds_currently_resolving_option_to_hand`; `lm_027_security_adds_card_to_hand_after_play`.
- **Remaining related work:** ST22-08, P-206, EX7-074, and sibling Options may still be blocked by other gaps such as DP/play-cost predicates, Plug-In/Link, Delay timing, or broader Option play-flow disposition.

<!-- Entry template:
### {Gap Title}
- **Discovered in:** {archetype name} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{relevant text}"
- **What's missing:** {description of engine capability needed}
- **Suggested change:** {brief proposal}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
-->

### Self-Digivolution-Stack Name Check (triggered clause condition)  [G-SELF-DIGIVOLUTION-CONTAINS-NAME]
- **Discovered in:** Medusamon Batch 11, BT20-102 Omnimon (X Antibody) DSL implementation (2026-04-27)
- **Card(s):** BT20-102 Omnimon (X Antibody) — "[On Play][When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, ..."
- **Effect text:** "If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards" — a condition on the triggering permanent's OWN card stack.
- **What's missing:** `lower_triggered.rs` passes `PredicateSubject::None` to the condition closure when evaluating a triggered clause's `condition:` block. This means no subject-requiring predicate (e.g., a hypothetical `self_digivolution_contains_name`) can evaluate the source permanent's own stack. The condition closure must receive a `PredicateSubject::Permanent(source_h)` where `source_h` is the permanent that fired the trigger. The engine method `Permanent::contains_card_name(name, data)` already exists in `permanent.rs` and scans the full card_sources stack — the gap is the predicate threading, not the engine primitive.
- **Suggested change:** In `lower_triggered.rs`, when building the condition closure, capture the `source_permanent` handle (available from `EffectContext` at fire time) and pass it as `PredicateSubject::Permanent(source_h)` instead of `PredicateSubject::None`. Add a `self_digivolution_contains_name: Option<String>` field to `BoolPredicateSpec` in `digimon-dsl` that evaluates `perm.contains_card_name(name, &game.card_data)` when the subject is a permanent. This is a hybrid gap: engine has the method, DSL+lowering need the predicate leaf + subject threading.
- **Workaround:** Entire boardwipe clause (clause d) routed through `raw_rust: { fn: bt20_102_boardwipe_and_return }` which checks `perm.contains_card_name("Omnimon", ...)` and `perm.contains_card_name("X Antibody", ...)` directly. Over-approximation: top card name "Omnimon (X Antibody)" always contains "X Antibody", so condition is always true for standalone BT20-102 rather than only when a genuine "Omnimon" or "X Antibody" base is in the digivolution stack.

### `for_each` + `delete_permanent` Stale Index After First Deletion  [G-FOR-EACH-DELETE-INDEX-SHIFT]
- **Discovered in:** Medusamon Batch 12, BT8-097 Crimson Blaze DSL implementation (2026-04-27)
- **Scope:** Rust engine.
- **Card(s):** BT8-097 Crimson Blaze — "[Main] Delete all of your opponent's Digimon with 6000 DP or less." Also any card whose `for_each` body includes a `delete_permanent` step and has multiple valid targets occupying ascending battle_area indices.
- **Effect text:** "Delete all of your opponent's Digimon with 6000 DP or less." — automated sweep, no player choice.
- **What's missing:** `permanent_scan::scan` in `src/dsl_cards/step/permanent_scan.rs` produces a snapshot of `PermanentHandle` values (each encoding `{player: u8, index: u8}`) before the `for_each` loop begins. `Player::delete_permanent` uses `Vec::remove(index)` which compacts the `battle_area` Vec in place. After the first deletion of a permanent at index `i`, all permanents at indices `> i` shift down by 1. The stale handle for the second target (originally at index `i+1`) now points to the permanent that was at `i+2` (or is out-of-bounds if the first deletion was the last element). The `field_index >= battle_area.len()` guard in `Player::delete_permanent` silently returns without deleting in the out-of-bounds case. Result: when all N targets need to be deleted and they are at contiguous ascending indices, only the first target is deleted.
- **Affected pattern:** `for_each { over: { all_of: [...] }, body: [delete_permanent] }` with 2+ matching targets sharing the same `player`. The bug is latent in BT9-112's Clause B test (`bt9_112_clause_b_deletes_all_lv4_or_lower_spares_lv5`) — that test passes only because the de-digivolve pass shifts survivor indices, masking which permanent was actually deleted.
- **Suggested change:** Either (a) reverse the scan order so highest indices are deleted first (no index shift affects lower indices), or (b) use a stable permanent identifier (e.g., `card_index: u16` on `CardSource`, which is already unique per card) instead of position-based handles, or (c) after each deletion, re-scan to collect the remaining targets. Option (a) is the lowest-effort fix: reverse `scan`'s output before the `for_each` iteration loop when the body contains a deletion verb, or unconditionally reverse (deletion order does not affect observable game state for mass-delete sweeps).
- **Workaround:** Test `bt8_097_main_deletes_multiple_opp_digimon_with_no_player_choice` is `#[ignore]`'d with this gap tag. The single-target delete test (`bt8_097_main_deletes_opp_digimon_with_dp_lte_6000`) still passes because only one target is in the scan snapshot.

### Breeding-Area Trigger Dispatch Missing  [G-BREEDING-TRIGGER-DISPATCH]
- **Discovered in:** Royal Knights archetype assessment (2026-04-28)
- **Scope:** Rust engine.
- **Card(s):** BT13-007 King Drasil_7D6 — `[Breeding] [Start of Your Main Phase] Reveal the top card of your Digi-Egg deck, then place that card and all of your [Royal Knight] trait Digimon as this Digimon's bottom digivolution cards.` BT20-083 Omekamon inherited also needs a breeding-area carrier for its opponent-security-removed trigger.
- **Effect text:** any clause whose source permanent is in the breeding area and whose timing fires while it remains there, especially `[Breeding] [Start of Your Main Phase]`, inherited breeding effects, and future effects that explicitly act from breeding.
- **What's missing:** `Game::enter_main_phase` fires `EffectTiming::StartOfYourMainPhase` via `TriggerSource::PlayerBattleArea(tp)`, and `enqueue_triggered` only scans `battle_area` for `PlayerBattleArea`. The engine comments already note that there is no `TriggerSource::BreedingArea`. As a result, a face-up `BT13-007` in breeding is never enqueued for its start-main trigger, even though the YAML is authored.
- **Suggested change:** Add a breeding-area trigger source, e.g. `TriggerSource::BreedingArea(PlayerId)` or extend `PlayerBattleArea` fan-out with an explicit `include_breeding` mode. `enter_main_phase` should enqueue `StartOfYourMainPhase` against both battle-area observers and the turn player's breeding permanent. `enqueue_from_permanent` must support a source permanent that lives in `player.breeding_area`, preserving source-card and controller attribution for effect conditions and activation counts.
- **First test:** place `BT13-007` in player 0 breeding, put one Royal Knight in player 0 battle area, enter main phase, and assert the top digitama plus that Royal Knight are placed under King Drasil while the Royal Knight leaves battle.
- **Workaround:** None — BLOCKED. Moving King Drasil to battle just to reuse `PlayerBattleArea` would change legal zones and action masks.

### Breeding-Area Pending Selection / Permanent Handles  [G-BREEDING-PERMANENT-SELECTION]
- **Discovered in:** Royal Knights archetype assessment (2026-04-28)
- **Scope:** Rust engine + DSL.
- **Card(s):** BT20-083 Omekamon — `[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area.` Also BT13-093 Omekamon, BT13-110 Royal Knights of the Purge, BT13-112 Omnimon, EX11-053 Omekamon, and BT23-072 King Drasil_7D6, all of which target or play cards from a breeding-area King Drasil stack.
- **Effect text:** effects that select "your [King Drasil_7D6] in the breeding area" or select cards from that Digimon's digivolution cards.
- **What's missing:** DSL selection lowering for `select_own_permanent` and `select_opponent_permanent` prefilters by iterating `player.battle_area`; the `zone: [breeding]` predicate cannot produce candidates. Runtime `PendingSelection` kinds and action encodings cover battle-area permanents, hands, trash, reveal, security, sources, and count-capped selections, but not a breeding-area permanent. `PermanentHandle { player, index }` currently encodes battle-area vector indices, so the breeding slot needs either a distinct handle form or a dedicated selection kind.
- **Suggested change:** Introduce a stable way to address breeding permanents in selections, such as `PermanentHandle::Breeding { player }` or a new `PermanentRef` enum with `BattleArea(PermanentHandle)` and `Breeding(PlayerId)`. Add an action-mask/decoder path for selecting the breeding slot, then update `select_own_permanent` / `select_any_permanent` prefilters to include it when the compiled predicate allows `CompiledZone::Breeding`.
- **First test:** trigger `BT20-083` On Deletion with a `BT13-007` in breeding and assert a pending selection offers the breeding King Drasil rather than silently doing nothing.
- **Workaround:** None faithful. Auto-selecting the only breeding permanent hides a gameplay choice and fails when future cards offer multiple legal destinations across battle/breeding zones.
- **Updated 2026-04-29:** Resolved for pending selection and DSL binding without fake battle-area handles. `EffectContext::select_own_breeding_permanent` installs `SelectionKind::BreedingPermanent`, masks only the phase-scoped breeding selection action (`encode_breeding_select(player)`), and DSL `select_own_breeding_permanent` binds a `BreedingPermanentRef`. Covered by `breeding_permanent_selection_targets_breeding_without_fake_battle_handle`, `breeding_selection_mask_exposes_only_breeding_select_action`, and `dsl_select_breeding_permanent_binds_target`.
- **Remaining limits:** Group 4 now covers effect-initiated movement to/from the real breeding slot and bottom-source placement under the `BREEDING_TARGET` selected breeding permanent. This still does not solve breeding-area trigger fan-out for effects whose source remains in breeding; keep that under `G-BREEDING-TRIGGER-DISPATCH`.

### Option-Placed Observer Timing Missing  [G-OPTION-PLACED-TIMING]
- **Discovered in:** Royal Knights archetype assessment (2026-04-28)
- **Scope:** Rust engine + DSL.
- **Card(s):** BT13-007 King Drasil_7D6 inherited — `[Breeding] [Your Turn] [Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory.` Royal Knights of the Purge (BT13-110) and The Last Guardian (BT20-100) are common Royal Knights options that need to surface this trigger when placed.
- **Effect text:** "When an Option card with the [Royal Knight] trait is placed in the battle area..."
- **What's missing:** The DSL has `CompiledTiming::OnOptionPlaced`, but `compiled_timing_to_engine` returns `None` for it, and the engine has no `EffectTiming::OnOptionPlaced` variant or dispatch site after Option cards are placed as battle-area permanents. Without a trigger context carrying the placed Option card, predicates such as `event_card_trait_has: "Royal Knight"` cannot be evaluated.
- **Suggested change:** Add `EffectTiming::OnOptionPlaced` and fire it after `dispose_option` / option placement helpers create the delayed/training/field Option permanent. Dispatch should scan relevant observers, including breeding-area sources once `G-BREEDING-TRIGGER-DISPATCH` is fixed, and should set trigger context fields for the placed card, owner, and permanent if one exists.
- **First test:** place `BT13-110` Royal Knights of the Purge into battle while `BT13-007` is in breeding with its inherited effect active, then assert the King Drasil controller gains 1 memory exactly once per turn.
- **Workaround:** None — BLOCKED for the inherited memory trigger. Piggybacking on `OnEnterFieldAnyone` would over-fire for Digimon/Tamers and lacks the Option-specific trait context.
- **Updated 2026-04-29:** Delay-style Option placement through `Game::play_option_from_hand` now dispatches `OnOptionPlaced` via `TriggerSource::OptionPlaced { player, permanent, card }`, and the placed Option is exposed through `TriggerContext.event_permanent`, `event_card`, and `source_player`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed_fires_after_delay_option_enters_battle_area` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed_event_card_trait_predicate_matches_placed_option`. Keep transient Standard options, security-effect placement, Link, Training, breeding-area observer fan-out, and once-per-turn Royal Knights inherited behavior as open follow-ups unless separately tested.

### `OnAllyAttack` / `OnOpponentAttack` Declared-Attack Observer Timing
- **Discovered in:** Dark Masters / Rocks archetype assessments (2026-04-29 follow-up)
- **Scope:** Rust engine runtime context.
- **Card(s):** BT15-008 Muchomon (`OnAllyAttack`-style "when one of your Digimon attacks a player"); EX10-003 Tumblemon and EX8-050 Gogmamon (`OnOpponentAttack`-style defender-side inherited observers, still blocked on follow-up cost/cancel primitives).
- **Effect text:** "When one of your red Digimon attacks a player..." / "When one of your opponent's Digimon attacks..."
- **Updated 2026-04-29:** Battle-area declared-attack observers now dispatch from the real combat state machine. `OnAllyAttack` scans the attacker's controller battle area and excludes the attacking permanent; `OnOpponentAttack` scans the defending player's battle area before Alliance/Counter/Block windows. `EffectReadContext` / `EffectContext` expose `attack_attacker()` and `attack_target()` over the live pending attack, with `attack_target()` reporting the effective target after substitution, including accepted optional target substitutions. `PendingAttack::declaration_committed` keeps optional pre-declaration replacement resumes legal while accepted pre-declaration cancel/substitute outcomes mutate the pending attack before declaration commits; `resolve_generic_selection` resumes parked attacks after replacement accept/decline resolution so normal `decode_action` callers cannot strand a pending attack. Post-declaration resumes require the original handle to still be a live attacking permanent. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- declared_attack_fires_ally_and_opponent_observers_with_attack_context on_ally_attack_does_not_fire_on_the_attacker_itself attack_target_context_reports_effective_declared_target_after_substitution accepted_predeclare_cancel_replacement_cancels_before_observers declined_predeclare_replacement_resumes_attack_declaration accepted_predeclare_target_substitution_updates_attack_context attack_resume_after_trigger_order_does_not_alias_removed_attacker on_ally_attack_still_fires_if_attacker_stack_changes_during_on_attack on_ally_attack_does_not_fire_if_attacker_left_during_on_attack on_opponent_attack_does_not_fire_if_ally_observer_removes_attacker`, plus `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- on_ally_attack` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- on_opponent_attack`.
- **Remaining limits:** First-class DSL predicates such as attack-target kind / attacker trait are still follow-ups. Breeding-area observer fan-out is not proven by this slice.
