# Engine Gaps Tracker

This file accumulates engine mechanics that are missing or incomplete, discovered during archetype implementation. Each entry includes the card that exposed the gap and what engine change is needed.

Last updated: 2026-03-15

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

## Remaining Gaps

### Digivolve from Hand or Trash
- **Discovered in:** Jesmon, Medusamon, TS Neptunemon (2026-03-17)
- **Card(s):** BT23-076, BT10-112, BT13-016, BT23-099, BT23-040
- **Effect text:** "digivolve into ... in the hand or trash"
- **What's missing:** `effect_digivolve_from_hand()` only searches hand. No `effect_digivolve_from_hand_or_trash()` variant exists.
- **Suggested change:** Add `include_trash=False` parameter to `effect_digivolve_from_hand()`, or add a new method that combines both zones.
- **Workaround:** Scripts manually build trash selection with `SEL_TRASH_START` indices (functional but inconsistent).

### Activate Another Card's When Digivolving Effect
- **Discovered in:** Jesmon (2026-03-17)
- **Card(s):** BT10-112 Jesmon GX, BT10-110 Seiken Meppa
- **Effect text:** "Activate 1 of that card's [When Digivolving] effects as an effect of this Digimon."
- **What's missing:** No engine API to enumerate a card's WD effects and execute a player-selected one.
- **Suggested change:** Add `game.effect_activate_card_effect(player, card_source, timing_filter, on_done)` that collects matching effects, presents selection, and executes.
- **Workaround:** None clean — scripts can iterate `card.effect_list(EffectTiming.WhenDigivolving)` manually but the selection UX is non-standard.

### Dynamic Security Attack Modifier
- **Discovered in:** Jesmon (2026-03-17)
- **Card(s):** BT10-112 Jesmon GX
- **Effect text:** "gains Security A. +1 for each card with the [Royal Knight] trait in this Digimon's digivolution cards"
- **What's missing:** `_security_attack_modifier` is a static int. No support for dynamic/computed SA modifiers.
- **Suggested change:** Support `_security_attack_modifier_fn` callable on ICardEffect, checked in `permanent.security_attack_modifier()`.
- **Workaround:** Use `register_modifier(ModifierType.CHANGE_SECURITY_ATTACK, ...)` with dynamic value_fn (if supported).

### Optional Attack ("may attack")
- **Discovered in:** TS Jupitermon, Jesmon, Medusamon (2026-03-17)
- **Card(s):** BT24-085, BT24-037, BT24-082, BT24-051
- **Effect text:** "1 of your Digimon may attack" / "it may attack"
- **What's missing:** `FORCE_ATTACK` modifier is mandatory. No "optional attack" that lets the player choose whether to attack.
- **Suggested change:** Add `ModifierType.MAY_ATTACK` that enables but doesn't force an attack action.
- **Workaround:** Scripts use `effect_select_own_permanent` + `FORCE_ATTACK` — the selection's `is_optional=True` serves as the "may" gate, but FORCE_ATTACK is then mandatory for the selected Digimon.

### Digimon-Only Attack Target Restriction
- **Discovered in:** TS Jupitermon (2026-03-17)
- **Card(s):** BT24-051 Merukimon
- **Effect text:** "attack your opponent's Digimon"
- **What's missing:** No modifier to restrict attack targets to Digimon only (exclude player).
- **Suggested change:** Add `ModifierType.CANNOT_ATTACK_PLAYER` checked in action mask.
- **Workaround:** None — RL agent can learn not to target player, but the action is still available.

### is_own_effect in WhenRemoveField Context
- **Discovered in:** TS Jupitermon, Jesmon (2026-03-17)
- **Card(s):** BT24-037 Silphymon, BT20-059 Gankoomon (X Antibody)
- **Effect text:** "other than by your effects"
- **What's missing:** WhenRemoveField context doesn't include `is_own_effect` flag. `removal_cause` exists but doesn't distinguish own vs opponent effects.
- **Suggested change:** Add `is_own_effect` bool to removal context.
- **Workaround:** Best-effort: check `removal_cause != 'cost'` (costs are always own). 'effect' cause is ambiguous.

### Conditional Color Requirement Bypass
- **Discovered in:** TS Neptunemon, Hudiemon (2026-03-17)
- **Card(s):** BT24-091 Tidal Stream, BT22-099 Kuremi Detective Agency
- **Effect text:** "While you have [TS] trait Digimon... ignore color requirements"
- **What's missing:** `card._match_color_requirement` is a static bool. No dynamic/conditional bypass.
- **Suggested change:** Support `_match_color_requirement_fn` callable checked in action_mask.
- **Workaround:** Set `_match_color_requirement = False` unconditionally (over-permissive).

### ~~DigiXros~~ — RESOLVED 2026-03-15
- **Card(s):** 60 cards across BT10-BT24, EX3-EX10, P sets
- **Resolution:** Engine natively supports DigiXros/Assembly: `DigiXrosCost` data model, `parse_digixros_req()` parser (all 60 cards), `digixros_validator.py` for material matching, play intercept → `SelectMaterial` loop → `_execute_digixros_play()`, field materials fire `WhenRemoveField` with `removal_cause='digixros'`, `digixros_count` in `OnEnterFieldAnyone` context.

<!-- Entry template:
### {Gap Title}
- **Discovered in:** {archetype name} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{relevant text}"
- **What's missing:** {description of engine capability needed}
- **Suggested change:** {brief proposal}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
-->
