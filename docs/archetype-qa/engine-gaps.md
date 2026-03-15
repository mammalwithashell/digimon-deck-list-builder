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

## Remaining Gaps

### One-Shot Digivolve Cost Hook
- **Discovered in:** BG Imperial / ExMaquinamon (2026-03-11)
- **Card(s):** BT3-103 (Hidden Potential Discovered!), EX1-071
- **Effect text:** "When one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5."
- **What's missing:** Player-level temporary digivolve cost reduction hook that fires once on the next qualifying digivolve, with suspend-as-cost.
- **Suggested change:** Add `player.register_one_shot_digivolve_hook(condition_fn, cost_fn, effect_fn)`.
- **Workaround:** None — BLOCKED. Security effects are implemented.
- **Impact:** 2 cards BLOCKED

### End-of-Turn DNA Digivolve
- **Discovered in:** BG Imperial (2026-03-14)
- **Card(s):** BT12-022, BT12-050
- **What's missing:** Engine API to perform DNA digivolution from an end-of-turn trigger.
- **Workaround:** None — BLOCKED (inherited effects).
- **Impact:** 2 cards BLOCKED

### Grant Triggered Effect to Opponent's Permanent
- **Discovered in:** Zephaga (2026-03-11)
- **Card(s):** BT14-044 — Palmon
- **Effect text:** "[Start of Your Main Phase] 1 of your opponent's Digimon gains '[All Turns] When this Digimon becomes suspended, lose 2 memory.' until the end of their turn."
- **What's missing:** Ability to grant a temporary triggered effect (OnTappedAnyone → lose memory) to an opponent's permanent with expiry.
- **Suggested change:** `permanent.grant_temp_effect(ICardEffect, expiry)` that attaches an effect to a permanent with time-based removal.
- **Workaround:** Descriptive-tagged stub.

### Effect-Based Play Lock
- **Discovered in:** Zephaga (2026-03-11)
- **Card(s):** BT9-047 — Pomumon
- **Effect text:** "[All Turns] Players can't play Digimon by effects."
- **What's missing:** Play-lock mechanism preventing effect-based Digimon plays while allowing normal main-phase plays.
- **Suggested change:** Add `ModifierType.CANNOT_PLAY_BY_EFFECT` that `effect_play_from_zone` checks.
- **Workaround:** Descriptive-tagged stub.

### Aura-Style CANNOT_UNSUSPEND for New Entries
- **Discovered in:** Zephaga (2026-03-11)
- **Card(s):** BT12-057 — Quartzmon
- **Effect text:** "[All Turns] All other Digimon and Tamers don't unsuspend."
- **What's missing:** Permanents entering the field AFTER Quartzmon don't receive CANNOT_UNSUSPEND. No pre-unsuspend aura hook.
- **Suggested change:** Aura-style modifier registration that auto-applies to new permanents.
- **Workaround:** Applied at digivolve time; new entries not affected.

### OnDigivolutionCardReturnToDeckBottom Not Auto-Fired
- **Discovered in:** Galacticmon (2026-03-13)
- **Card(s):** BT18-065 (Snatchmon), BT18-092 (Zenith) — Vemmon-archetype
- **What's missing:** `EffectTiming.OnDigivolutionCardReturnToDeckBottom` is defined but `permanent.py` never fires it automatically.
- **Suggested change:** Add `_fire_timing(OnDigivolutionCardReturnToDeckBottom, ...)` in engine wherever digi-cards return to deck bottom.
- **Workaround:** Scripts manually call `game.execute_effects()`. Functional but fragile.

### Top/Bottom Deck Choice
- **Discovered in:** Jesmon (2026-03-11)
- **Card(s):** BT23-057 — Gankoomon
- **What's missing:** Cost reduction returns cards to deck but player should choose top or bottom. Currently defaults to bottom.
- **Impact:** Minor UX (doesn't affect game outcome significantly)

### WhenRemoveField Lacks Removal Cause Context
- **Discovered in:** Rocks (2026-03-14)
- **Card(s):** EX7-049 (Metallicdramon)
- **What's missing:** WhenRemoveField timing does not pass why the permanent is being removed (battle, effect, de-digivolve, etc.).
- **Impact:** 1 card (ENGINE-LIMITATION)

### Face-Down Card Tracking
- **Discovered in:** Millenniummon (2026-03-14)
- **What's missing:** Engine does not track which digivolution cards are face-down vs face-up. Approximation counts all non-top sources.
- **Impact:** Minor (EX9-009 approximation acceptable)

### Ignore Color Requirement Aura
- **Discovered in:** Hudiemon (2026-03-14)
- **What's missing:** `card._match_color_requirement = False` works for self but not as an aura to grant color bypass to other cards.
- **Impact:** 8 cards (non-blocking, affects Option plays only)

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
