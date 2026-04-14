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

<!-- Entry template:
### {Gap Title}
- **Discovered in:** {archetype name} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{relevant text}"
- **What's missing:** {description of engine capability needed}
- **Suggested change:** {brief proposal}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
-->
