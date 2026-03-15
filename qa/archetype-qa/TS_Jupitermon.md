# Archetype QA: TS Jupitermon
Date: 2026-03-14 (updated)
Total cards: 30

## Summary
- PASS: 7 (BT24-030, BT24-040, BT24-046, BT24-100, BT24-102, P-194, BT24-003)
- IMPLEMENTED: 1 (BT7-032 — CLEAN)
- QA-REVIEWED + FIXED: 1 (EX4-074)
- QA-FAIL -> FIXED: 20
- BLOCKED: 1 (BT10-042 effect1: disable_effect)

## Implemented Cards
### BT7-032 Pulsemon
- Inherited [When Attacking] [OPT] if 3 security, gain 2 memory

## Fixed Cards

### Batch 1
| Card | Fixes |
|------|-------|
| BT24-003 | digi_filter now checks [Shaman] trait; added cost_reduction=1 |
| BT24-014 | Delete gated on security count <= 3 |
| P-196 | Memory check: player.memory → game.memory |
| P-197 | digi_filter: added [Angel] trait; added memory <= 4 gate; added cost_override=0 |
| P-198 | digi_filter: added [Fallen Angel] trait; added memory <= 4 gate; added process2 callback |
| P-213 | condition3: added security <= 3 check |
| BT15-003 | Added security trash cost via effect_choose_branch (top/bottom) |

### Batch 2
| Card | Fixes |
|------|-------|
| BT24-037 | play_filter: AND→OR logic (Yellow/Red/TS); source zone: hand→digi-stack |
| BT24-051 | effect5: added process callback (suspend to prevent deletion) |
| BT24-041 | effect6: added process callback (trash security to prevent deletion) |
| BT24-084 | condition1: card.permanent_of_this_card() replaces non-existent effect_source_permanent; process1: digivolves own Aegiomon (was suspending opponent's) |
| BT24-031 | Recovery check moved inside async callback chain |
| BT24-034 | "By" cost gated via effect_choose_branch opt-in |

### Batch 3
| Card | Fixes |
|------|-------|
| BT10-042 | SA modifier: _temp_sa_modifier → register_modifier(CHANGE_SECURITY_ATTACK). **BLOCKED: effect1 disable_effect** |
| BT14-033 | Added random.shuffle(player.security_cards) after security search |
| BT24-083 | No fix needed — manual list ops are correct pattern |
| BT24-090 | DP modifier: added _dp_permanent_condition filter; security: raw Permanent → play_card_from_source |
| BT24-101 | effect2/3: added OnEnterFieldAnyone timing; effect4: added OnLoseSecurity timing |

### Direct Fixes
| Card | Fix |
|------|-----|
| BT4-104 | security_cards.pop() → pop(0) (top not bottom) |
| BT24-043 | Suspend target_filter: True → p.is_digimon |
| BT24-088 | Three Musketeers: card_text → card_traits |
| EX4-074 | DP: dp_modifier → register_modifier; deletion: hardcoded → selection; hatch: manual → player.hatch() |

## Blocked Cards (Engine Gaps)
| Card | Gap |
|------|-----|
| BT10-042 effect1 | disable_effect — no engine API to selectively disable effects on a permanent |

### 2026-03-14 Grant Skill + QA Fixes
| Card | Fix |
|------|-----|
| BT24-051 | **Rush/Piercing stubs resolved**: Replaced pass-stub with `_applies_to_all_own_digimon` aura effects for both `_is_rush` and `_is_piercing` targeting [Iliad] Digimon. On Play/WD: fixed auto-suspend to use `effect_select_opponent_permanent` for player selection (up to 2). Removed incorrect Rush/Piercing mention from On Play/WD description; C# confirms +5000 DP and attack opponent's Digimon only. |
| BT24-090 | **CRITICAL**: Card text says Blocker, not +2000 DP. Replaced `dp_modifier = 2000` with `_is_blocker = True` aura effect. Alliance stub replaced with proper `_is_alliance` aura effect. Both use `_applies_to_all_own_digimon` + `_keyword_permanent_condition`. |
| BT24-094 | **Alliance stub resolved**: Replaced `pass` stub with proper `_is_alliance` aura effect using `_applies_to_all_own_digimon` + `_keyword_permanent_condition` for green/yellow [TS] Digimon. DP modifier retained (correct per card text). |
| BT24-085 | Fixed description: "number of Digimon" -> "memory" (matches C# and card text). Added `CanActivateCondition`: only fires when player memory <= 0 (memory on opponent's side). |
| BT24-101 | **Alt-digi cost fixed**: Both effects had cost 3, should be 5 per C#. Second alt-digi now targets [Aegiochusmon] by name. **-13000 DP target selection**: replaced auto-select (min DP) with `effect_select_opponent_permanent` for RL agent choice. |

### Engine Enhancement: Aura Keyword System
Added aura keyword scanning to `permanent.py:has_keyword()` and `permanent.py:_get_aura_dp_modifier()`:
- Effects with `_applies_to_all_own_digimon = True` and a keyword attribute (e.g. `_is_rush`, `_is_blocker`) now grant that keyword to other friendly Digimon via aura scanning
- Supports `_keyword_permanent_condition` callback for trait-based filtering
- Also scans security cards for aura effects (option cards placed face-up in security)

## Smoke Test
- 50/50 mirror games completed
- 25/25 cross-archetype games completed
