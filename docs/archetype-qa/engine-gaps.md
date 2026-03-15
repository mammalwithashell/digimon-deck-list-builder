# Engine Gaps Tracker

This file accumulates engine mechanics that are missing or incomplete, discovered during archetype implementation. Each entry includes the card that exposed the gap and what engine change is needed.

Last updated: 2026-03-14

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

## Remaining Gaps

### DigiXros (Deferred to Phase 7)
- **Card(s):** BT21-021 (OmniShoutmon) + future Xros Heart cards
- **What's missing:** Full DigiXros pipeline (multi-permanent selection, field-to-source, cost reduction).
- **Impact:** Deferred — not in Phase 1-6 scope.

<!-- Entry template:
### {Gap Title}
- **Discovered in:** {archetype name} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{relevant text}"
- **What's missing:** {description of engine capability needed}
- **Suggested change:** {brief proposal}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
-->
