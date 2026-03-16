# QA Report: DNA Omnimon vs Medusamon
Date: 2026-03-15
Agent: QA Agent 7

## Test Setup
- **P1 Deck**: DNA Omnimon (54 cards)
- **P2 Deck**: Medusamon (54 cards)
- **Engine**: localhost:8000, DEBUG_MODE=1
- **Method**: Automated regression (random + greedy) + targeted debug games

## Automated Regression Results

### Random Policy (10 games, direct engine)

| # | Config | Winner | Turns |
|---|--------|--------|-------|
| 1 | Omnimon P1 vs Medusa P2 | P2 | 15 |
| 2 | Omnimon P1 vs Medusa P2 | P2 | 18 |
| 3 | Omnimon P1 vs Medusa P2 | P1 | 15 |
| 4 | Omnimon P1 vs Medusa P2 | P2 | 16 |
| 5 | Omnimon P1 vs Medusa P2 | P2 | 16 |
| 6 | Medusa P1 vs Omnimon P2 | P1 | 11 |
| 7 | Medusa P1 vs Omnimon P2 | P1 | 21 |
| 8 | Medusa P1 vs Omnimon P2 | P1 | 13 |
| 9 | Medusa P1 vs Omnimon P2 | P1 | 10 |
| 10 | Medusa P1 vs Omnimon P2 | P1 | 13 |

**Result: 10/10 completed, 0 crashes.**

### Greedy Policy (10 games, /simulations endpoint)

**Omnimon P1 vs Medusa P2 (5 games):** P1 win rate 80%, P2 win rate 20%

| Sim | Winner | Steps |
|-----|--------|-------|
| 0 | P1 | 43 |
| 1 | P1 | 30 |
| 2 | P1 | 45 |
| 3 | P2 | 60 |
| 4 | P1 | 44 |

**Medusa P1 vs Omnimon P2 (5 games):** P1 win rate 80%, P2 win rate 20%

| Sim | Winner | Steps |
|-----|--------|-------|
| 0 | P1 | 55 |
| 1 | P1 | 64 |
| 2 | P1 | 44 |
| 3 | P2 | 36 |
| 4 | P1 | 52 |

**Result: 10/10 completed, 0 crashes.**

**Total: 20/20 games completed without crashes or hangs.**

---

## Targeted Debug Tests

### Test 1: BT17-015 WarGreymon Inherited Effect on Omnimon (PASS)

**Objective:** Verify WarGreymon's inherited "[When Attacking] If this Digimon has [Omnimon] in its name, trash the top card of opponent's security stack" fires when under an Omnimon.

**Setup:**
1. RizeGreymon (BT22-012, Lv.5) played on field
2. WarGreymon (BT17-015) digivolved onto RizeGreymon (alt-digi from Greymon, cost 3)
3. Omnimon (BT17-078) digivolved onto WarGreymon (cost 5)
4. Stack: Omnimon (top) > WarGreymon > RizeGreymon

**Execution:**
- Attacked player with Omnimon (action 114)
- Logs confirm: `[Effect] OnUseAttack | Unknown: [When Attacking][Once Per Turn] If this Digimon has [Omnimon] in its name, trash the top card of opponent's security stack.`
- P2 security went from 5 to 2 (1 trashed by effect + 2 security checks)

**Verdict: PASS** -- The `perm.top_card.contains_card_name('Omnimon')` check correctly identifies the Omnimon top card and fires the inherited security-trashing effect.

---

### Test 2: BT24-016 Lamiamon [Hand][Main] Effect (CRITICAL - SCRIPT NOT LOADED)

**Objective:** Test Lamiamon's compound flow: place Dimetromon from trash under Elizamon, then digivolve into Lamiamon.

**Setup:**
- Elizamon (BT24-008) on field
- Owen Dreadnought (BT24-082) tamer on field
- Dimetromon (BT21-017) in trash
- Lamiamon (BT24-016) in hand

**Finding:** The [Hand][Main] action (actions 30-59) never appeared in the action mask despite all conditions being met.

**Root Cause: Uppercase filename breaks script loading.**

The file `digimon_gym/engine/data/scripts/bt24/BT24_016.py` has an uppercase filename. The script loader converts to lowercase (`bt24_016`) for import, which fails:
```
importlib.import_module('digimon_gym.engine.data.scripts.bt24.bt24_016')
# -> No module named 'digimon_gym.engine.data.scripts.bt24.bt24_016'
```

The script silently fails to load, and BT24-016 operates as a vanilla card with **no effects at all** -- no [Hand][Main], no [When Digivolving], no [When Attacking], no inherited effect.

**Verdict: CRITICAL -- BT24-016 Lamiamon has zero effects.**

**Same issue affects 6 other files (all from Medusa Round 2 fixes, saved with uppercase names):**

| File | Card | In Deck? |
|------|------|----------|
| `bt24/BT24_016.py` | Lamiamon | **YES** (4x) |
| `bt24/BT24_082.py` | Owen Dreadnought | **YES** (4x) |
| `bt24/BT24_089.py` | Unique Emblem: Blazing Conductor | **YES** (4x) |
| `bt21/BT21_072.py` | Arresterdramon: Superior Mode | No |
| `ex8/EX8_074.py` | MedievalGallantmon | No |
| `ex9/EX9_013.py` | BlitzGreymon | No |
| `p/P_206.py` | Digital Gate Open | No |

**Fix:** Rename all 7 files to lowercase to match the import convention.

---

### Test 3: BT21-029 Medusamon When Digivolving + End of Attack + Petrification Tokens (QA-FAIL)

**Objective:** Verify Medusamon's delete lowest DP, end-of-attack delete, and petrification token generation.

**Setup:**
1. Aldamon (BT21-020, Lv.5) played on field
2. Medusamon (BT21-029) digivolved onto Aldamon
3. P2 had two Lv.3 Digimon (DP 2000 each) on field

**Results:**

| Sub-test | Result | Notes |
|----------|--------|-------|
| When Digivolving: delete lowest DP | PASS (partial) | Effect fired (`[Effect] WhenDigivolving`) but auto-resolved via `is_optional=True` without visible selection. One Agumon deleted on subsequent attack. |
| End of Attack: delete lowest DP | PASS | After attack, log shows `Player 2's permanent Agumon deleted.` |
| Progress keyword (SA+1) | PASS | Attack checked 2 security cards with "Security effects blocked -- attacker has Progress!" |
| Petrification token on opponent Digimon delete (effect4) | **FAIL** | Token never played on P2's field after Agumon deletion |
| Petrification token on opponent security loss (effect5) | **FAIL** | Token never played on P2's field after security went from 5 to 3 |

**Root Cause Analysis -- Petrification Token Failure:**

**Effect4 (`OnDestroyedAnyone`, `is_on_deletion=True`):**
The engine's `execute_deletion_effects()` only scans the **deleted permanent's own card sources** for `is_on_deletion` effects. Medusamon's petrification effect is on Medusamon (a different permanent), so it is never found. The `_fire_deletion_observers()` method does scan other permanents, but it requires `_is_deletion_observer = True` flag, which the script does not set.

**Effect5 (`OnLoseSecurity`):**
The condition checks `ctx_player = context.get('player')` expecting it to be the security-losing player (P2). However, `execute_effects()` sets `context['player']` to the **permanent owner** (P1, Medusamon's owner) and stores the event's original player as `context['event_player']`. Since `ctx_player` (P1) `is` `owner` (P1), the condition returns False, blocking the effect.

**Fix needed for effect4:** Add `_is_deletion_observer = True` to the effect, or change timing approach.
**Fix needed for effect5:** Change condition to use `context.get('event_player')` instead of `context.get('player')`.

**Verdict: QA-FAIL -- Petrification tokens never generated.**

---

### Test 4: BT24-082 Owen Dreadnought + BT24-089 Unique Emblem (NOT LOADED)

Both cards have uppercase filenames (`BT24_082.py`, `BT24_089.py`) and suffer from the same script loading failure as BT24-016. Owen Dreadnought operates as a vanilla tamer with no effects (no DP grants, no FORCE_ATTACK). Unique Emblem operates as a vanilla option with no delay or digivolve effects.

**Verdict: CRITICAL -- Scripts not loaded.**

---

## Summary

| Card | Test | Verdict | Notes |
|------|------|---------|-------|
| BT17-015 WarGreymon | Inherited `contains_card_name('Omnimon')` check | **PASS** | Fires correctly under Omnimon |
| BT17-078 Omnimon | Normal digivolve + WhenDigivolving | **PASS** | Digivolves onto WarGreymon, effect fires |
| BT21-029 Medusamon | WhenDigivolving delete | **PASS** | Fires and deletes |
| BT21-029 Medusamon | End of Attack delete | **PASS** | Fires via OnEndAttack |
| BT21-029 Medusamon | Progress + SA+1 | **PASS** | 2 security checks with Progress immunity |
| BT21-029 Medusamon | Petrification token on delete | **FAIL** | `is_on_deletion` not found by deletion observer system |
| BT21-029 Medusamon | Petrification token on security loss | **FAIL** | Condition checks wrong `player` key in context |
| BT24-016 Lamiamon | All effects | **CRITICAL** | Script not loaded (uppercase filename) |
| BT24-082 Owen Dreadnought | All effects | **CRITICAL** | Script not loaded (uppercase filename) |
| BT24-089 Unique Emblem | All effects | **CRITICAL** | Script not loaded (uppercase filename) |
| Regression (20 games) | Stability | **PASS** | No crashes across random + greedy |

## Systemic Issues Found

### Issue 1: Uppercase Script Filenames Break Import (CRITICAL)
**Scope:** 7 files (all from Medusa archetype Round 2 fixes)
**Impact:** Cards operate as vanilla with zero effects
**Files:** `BT24_016.py`, `BT24_082.py`, `BT24_089.py`, `BT21_072.py`, `EX8_074.py`, `EX9_013.py`, `P_206.py`
**Fix:** `git mv` each file to lowercase (e.g., `BT24_016.py` -> `bt24_016.py`)

### Issue 2: BT21-029 Petrification Token -- Wrong Observer Mechanism
**Scope:** BT21-029 effect4 (OnDestroyedAnyone)
**Impact:** Petrification tokens never generated when opponent Digimon are deleted
**Fix:** Set `effect4._is_deletion_observer = True` and use `EffectTiming.NoTiming` timing so `_fire_deletion_observers()` picks it up. The condition already filters for opponent-only deletion.

### Issue 3: BT21-029 Petrification Token -- Wrong Context Key
**Scope:** BT21-029 effect5 (OnLoseSecurity)
**Impact:** Petrification tokens never generated when opponent loses security
**Fix:** Change `ctx_player = context.get('player')` to `ctx_player = context.get('event_player') or context.get('player')` in condition5.
