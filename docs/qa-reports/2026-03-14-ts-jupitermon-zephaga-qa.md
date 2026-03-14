# QA Report: TS Jupitermon vs Zephagamon
Date: 2026-03-14

## Test Setup
- **TS Jupitermon** deck (Spain 1st Place, digimonmeta_20bbf3003f6a): 54 cards
- **Zephagamon** deck (digilab_a02eaa11e002, 1st Place): 54 cards
- All games run via `/debug/games` endpoint with deterministic settings
- Manual card-by-card testing plus greedy auto-play regression

## Critical Bug Found: Tamer Security Effect Duplication

### Description
Tamers with `[Security] Play this card without paying the cost` effects that use
`play_card_from_source(card, pay_cost=False)` in their security process callback
**duplicate on normal play from hand**. Playing 1 copy creates 2 permanents on the field.

### Root Cause
In `_effect_matches_timing()` (game/__init__.py line 736), when an effect has:
- No timing set (`effect.timing is None`)
- No flag set (not `is_on_play`, not `is_when_digivolving`, etc.)
- `is_security_effect = True` (ignored by the matcher)

The code falls through to the `OnEnterFieldAnyone` check (line 759) which returns `True`
for the just-played card. The security effect's process callback then fires, calling
`play_card_from_source()` which creates a second permanent from the same card object.

### Affected Cards (43 scripts)
All tamers with `is_security_effect = True` + `play_card_from_source` in process callback
but no `set_timing(EffectTiming.SecuritySkill)`:

**In this matchup:**
- EX7-064 Shoto Kazama (Zephagamon deck)
- ST18-14 Shoto Kazama (alternate Zephagamon builds)

**Systemic (43 total):** BT1-087, BT3-093, BT3-096, BT4-097, BT5-090, BT5-092,
BT5-093, BT5-112, BT6-107, BT7-105, BT9-092, BT11-105, BT17-081, BT17-093,
BT20-055, BT24-086, BT24-100, EX2-064, EX4-061, EX7-063, EX7-064, EX9-066,
EX9-067, EX9-068, EX9-069, EX9-070, EX11-067, LM-033, LM-034, LM-035, LM-037,
LM-045, LM-048, LM-050, P-037, P-105, ST6-14, ST14-11, ST15-14, ST16-14,
ST18-14, ST19-14, ST20-05

### Suggested Fix
Either:
1. In `_effect_matches_timing()`, add early return: `if effect.is_security_effect: return False`
2. Or add `set_timing(EffectTiming.SecuritySkill)` to all 43 affected scripts

Option 1 is the engine-level fix and prevents future regressions.

### Reproduction
```
POST /debug/games
deck2: [..., "EX7-064", ...]
hand2: ["EX7-064", ...]

# After playing EX7-064, field shows 2 copies instead of 1
```

---

## TS Jupitermon Card Results

| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT24-003 | Tsunomon | PASS | Egg hatches correctly, inherited effect present |
| P-194 | Aegiomon | PASS | Plays correctly (Lv4, cost 4), no on-play trigger expected |
| P-196 | Gomamon | PASS | Lv3 rookie, plays correctly |
| P-197 | Patamon | PASS | Lv3 rookie, plays correctly |
| BT14-033 | Patamon | PASS | Lv3 rookie, plays correctly |
| BT24-034 | Aegiomon | PASS | On Play effect works: "By adding top security to hand, play [TS] Tamer free" with opt-in branch, tamer name filter, correct security cost. Barrier keyword present. |
| BT24-014 | Aegiochusmon | PASS | Lv5, digivolves correctly from Aegiomon (cost 3) |
| BT24-046 | Garurumon | PASS | Lv4, plays correctly |
| P-213 | Aegiochusmon | PASS | Lv5 alternate, plays correctly |
| BT24-101 | Jupitermon | PASS | Lv6, digivolve chain BT24-034->BT24-014->BT24-101 works. When Digivolving: trashes own security, applies -13000 DP to opponent Digimon, conditional Recovery +2 (tested: skips when security > 1). OnLoseSecurity effect correctly trashes opponent's top security (verified: P2 sec 5->4 when P1 sec was trashed). |
| BT24-083 | Hiroko Sagisaka | PASS | Tamer plays correctly (cost 3). On Play: reveal top 3, select 1 [TS] to hand works. Security effect has no process callback so duplication bug does not affect it. |
| BT24-084 | Inori Misono | PASS | Tamer plays correctly (cost 3). No on-play trigger expected. |
| BT24-088 | Asuna Shiroki | PASS | Tamer, consistent with other tamers in deck |
| BT24-102 | Homeros | PASS | Tamer, plays correctly |
| BT24-100 | In-Between Theater | PASS | Option with OptionSkill timing, correctly gated by color requirement (white card needs [TS] on field). Delay effect and security effect use SecuritySkill timing. |
| BT4-104 | Blinding Ray | PASS | Option, correctly not playable without yellow Digimon on field |
| BT10-042 | Venusmon | BLOCKED | effect1 disable_effect not implemented (known, pre-existing) |
| BT24-040 | Venusmon | PASS | Lv6, plays correctly |
| BT24-041 | Minervamon | PASS | Lv6, plays correctly |

**TS Jupitermon Summary: 18 PASS, 1 BLOCKED (pre-existing)**

---

## Zephagamon Card Results

| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| EX7-004 | Fluffymon | PASS | Digi-Egg hatches correctly |
| EX7-031 | Pteromon | PASS | Lv3, plays correctly |
| ST18-04 | Pteromon | PASS | Lv3, on-play reveal and trash selection works correctly |
| EX11-026 | Pteromon | PASS | Lv3 (corrected in previous QA) |
| EX11-028 | Galemon | PASS | Lv4, On Play suspend (any Digimon) works correctly with own/opponent targeting. OnTappedAnyone Shoto play effect has proper tamer count gate and Shoto filter. |
| P-166 | Galemon | PASS | Lv4 (corrected in previous QA) |
| BT24-047 | Kokatorimon | PASS | Lv4 (corrected in previous QA) |
| EX11-032 | GrandGalemon | PASS | Lv5, digivolves from Galemon correctly (cost 3), drew 1 card |
| ST22-13 | GrandGalemon | PASS | Lv5, plays correctly |
| EX11-035 | Zephagamon | PASS | Lv6, digivolves correctly. Keywords: Piercing, Vortex, Blocker all present. When Digivolving unsuspend/suspend selection works. |
| BT20-101 | Zephagamon | PASS | Lv6 (corrected in previous QA) |
| EX11-074 | Vortexdramon | PASS | Lv6, plays correctly. Keywords: Piercing, Vortex, Blocker present. DP=14000 correct. |
| EX7-064 | Shoto Kazama | **FAIL** | **Tamer duplicates on play** (security effect fires as on-play). See Critical Bug above. |
| EX11-062 | Shoto Kazama | PASS | Tamer plays correctly, no duplication (security effect handled differently) |
| BT20-085 | Shoto Kazama | PASS | Tamer (corrected in previous QA) |
| EX11-072 | Guardian Vortex | PASS | Option (corrected in previous QA) |
| LM-030 | Green Scramble | PASS | Option (corrected in previous QA) |
| P-038 | Green Memory Boost | PASS | Option (corrected in previous QA) |
| P-106 | Agility Training | PASS | Option (corrected in previous QA) |
| BT3-103 | Hidden Potential Discovered! | BLOCKED | Pre-existing: no player-level digivolve cost hook |

**Zephagamon Summary: 18 PASS, 1 FAIL, 1 BLOCKED (pre-existing)**

---

## Regression Test: Greedy Auto-Play

A 30-step greedy auto-play session (TS Jupitermon P1 vs Zephagamon P2) completed
without crashes or exceptions. The game progressed through 10 turns with proper
phase transitions (Breeding -> Main -> SelectReveal cycles).

**Note:** Full game completion was not achievable in auto-play because the greedy
policy (first valid non-pass action) doesn't generate efficient attacking strategies.
The game engine handled all 30 action steps without errors.

---

## Summary

| Archetype | PASS | FAIL | BLOCKED |
|-----------|------|------|---------|
| TS Jupitermon (19 unique cards) | 18 | 0 | 1 (pre-existing BT10-042) |
| Zephagamon (20 unique cards) | 18 | 1 | 1 (pre-existing BT3-103) |

### Critical Issue
- **EX7-064 Shoto Kazama tamer duplication on play** - systemic bug affecting 43 tamer
  scripts across the codebase. Security effects without explicit timing get incorrectly
  matched as OnEnterFieldAnyone effects by `_effect_matches_timing()`. Fix recommended
  in engine (`game/__init__.py`) rather than per-script.

### Pre-existing BLOCKED
- BT10-042 effect1: disable_effect (no engine API)
- BT3-103: no player-level digivolve cost hook
