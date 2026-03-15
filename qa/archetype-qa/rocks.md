# Archetype QA: Rocks
Date: 2026-03-14 (updated)
Total cards: 48

## Summary
- PASS: 30
- FIXED: 17 (all previously QA-FAIL cards fixed)
- ENGINE-LIMITATION: 1 (EX7-049)

## Smoke Test
- 50/50 mirror-match games completed successfully (random policy, max 500 steps)

---

## Card Results

### Batch 1: Eggs + Rookies
| Card | Name | Verdict |
|------|------|---------|
| EX8-005 | Tumblemon | PASS |
| EX10-003 | Tumblemon | PASS |
| BT21-055 | Sunarizamon | FIXED (was critical: missing BeforePayCost timing + leak guard) |
| EX10-025 | Sunarizamon | FIXED (was medium: auto-selected trash cards, now player selects) |
| EX8-046 | Gotsumon | PASS |
| EX8-047 | Sunarizamon | FIXED (was low: reveal is_optional now False) |
| EX11-038 | Sunarizamon | PASS |

### Batch 2: Champions
| Card | Name | Verdict |
|------|------|---------|
| EX8-048 | Landramon | PASS |
| EX10-028 | Landramon | FIXED (was medium: player now chooses source + fires OnDigivolutionCardDiscarded + value_fn fixed) |
| P-167 | Landramon | FIXED (was medium: deck top/bottom choice added + fires OnDigivolutionCardDiscarded) |
| P-215 | Icemon | PASS |
| BT14-009 | Gotsumon | PASS |
| BT18-064 | Mercurymon | PASS |

### Batch 3: Close/Tamers
| Card | Name | Verdict |
|------|------|---------|
| EX8-067 | Close | PASS |
| EX10-063 | Close | PASS |
| EX11-065 | Close | PASS |
| P-169 | Close | PASS |
| P-130 | Lui Ohwada | FIXED (was: process suspended opponent instead of self tamer) |
| BT8-094 | Digimon Emperor | PASS |

### Batch 4: Fragment Ultimates
| Card | Name | Verdict |
|------|------|---------|
| EX8-050 | Gogmamon | PASS |
| BT4-072 | Gogmamon | PASS |
| EX8-051 | Proganomon | FIXED (was high: inherited condition now checks trashed_cards + Mineral/Rock trait) |
| EX10-032 | Proganomon | FIXED (was critical: inherited condition now checks trashed_cards + trait; value_fn fixed; fires OnDigivolutionCardDiscarded) |
| EX8-055 | Pyramidimon | FIXED (was high: SA+1 now via register_modifier with end_of_turn expiry; fires OnDigivolutionCardDiscarded) |
| EX10-033 | Pyramidimon | FIXED (was high: cost reduction value_fn fixed with 3-arg lambda + condition scoped to target; trash card placement via player selection) |
| EX11-044 | Pyramidimon | PASS |

### Batch 5: Complex Megas
| Card | Name | Verdict |
|------|------|---------|
| EX10-034 | Blastmon | FIXED (was critical: timing OnDeclaration->OnUseAttack; value_fn lambda: 3000 -> lambda cur,t,c: cur+3000; SA+1 likewise fixed) |
| EX10-036 | Magneticdramon | FIXED (was high: cross-Digimon source count now aggregated; trash placement via player selection) |
| EX7-049 | Metallicdramon | ENGINE-LIMITATION (WhenRemoveField lacks removal-cause context) |

### Batch 6: Archetype Options
| Card | Name | Verdict |
|------|------|---------|
| BT9-103 | Kongou | PASS |
| EX8-070 | Zofr Kabus | PASS |
| EX10-069 | Unique Emblem: Gravel Hearts | FIXED (was critical: Close filter now checks card_names not traits; delay trigger uses contains_card_name) |
| LM-031 | Black Scramble | FIXED (was medium: security card retrieval by identity ref; delay trash selection via player choice) |
| LM-032 | Purple Scramble | FIXED (was: auto-selected first purple Digimon; delay auto-selected trash card; now player selects both) |
| BT23-096 | Comet Hammer | PASS |

### Batch 7: Generic Support
| Card | Name | Verdict |
|------|------|---------|
| BT16-082 | Ukkomon | FIXED (was low: reveal now mandatory; hatch now optional via choose_branch) |
| BT20-055 | Invisimon | FIXED (was low: added opponent turn check) |
| P-039 | Black Memory Boost! | PASS |
| P-107 | Defense Training | FIXED (was high: delay now selects target Digimon first; reveal now mandatory) |
| P-206 | Digital Gate Open | FIXED (was low: reveal now mandatory) |
| P-123 | Ukkomon | FIXED (was: missing hatch from card text, now includes optional hatch) |
| P-186 | Gallantmon | PASS |
| BT21-021 | OmniShoutmon | FIXED (was: process5 incorrectly deleted opponent Digimon; now correctly plays from hand cost -5; OnDeletion stub implemented) |
| BT23-059 | Justimon: Blitz Arm | FIXED (was: register_modifier arg order wrong for CANNOT_BE_SELECTED_BY_EFFECT) |
| EX7-074 | Vortex Resonance | FIXED (was: reveal is_optional now False) |
| ST13-08 | Chikurimon | PASS |
| ST22-11 | Defense Plug-In F | PASS |

---

## Engine Gaps

### EX7-049 Metallicdramon -- WhenRemoveField lacks removal-cause context
- Card text: "When this Digimon would leave battle area **other than by one of your effects**"
- The `WhenRemoveField` context does not carry a removal-cause flag
- The script cannot distinguish self-removal from opponent-removal
- Consistent with how all other `WhenRemoveField` scripts in the codebase handle this limitation
- **Not fixable at script level** -- requires engine enhancement to pass removal source in context

---

## Fixes Applied (2026-03-14)

### Critical fixes
1. **BT21-055**: Added `EffectTiming.BeforePayCost` timing + leak guard on digivolution cost reduction
2. **EX10-032**: Inherited condition now checks `card not in trashed_cards` and Mineral/Rock trait; value_fn fixed to 3-arg lambda; fires OnDigivolutionCardDiscarded event
3. **EX10-034**: Timing changed from `OnDeclaration` to `OnUseAttack`; value_fn lambdas changed from 0-arg to 3-arg (`lambda cur, t, c: cur + 3000`); FORCE_ATTACK modifier arg order fixed
4. **EX10-069**: Close filter changed from trait check to card_names check; delay trigger uses `contains_card_name('Close')`

### High fixes
5. **EX8-051**: Inherited condition now checks trashed_cards identity + Mineral/Rock trait
6. **EX8-055**: SA+1 changed from `_temp_sa_modifier` to `register_modifier(CHANGE_SECURITY_ATTACK, expiry='end_of_turn')`; fires OnDigivolutionCardDiscarded; end-of-turn trash placement now via player selection
7. **EX10-033**: Cost reduction value_fn fixed to 3-arg; condition added to scope to target; trash placement via player selection
8. **EX10-036**: Cross-Digimon source counting fixed (was per-permanent, now aggregated); trash placement via player selection
9. **P-107**: Delay digivolve now selects target Digimon first via `effect_select_own_permanent`; reveal is_optional set to False

### Medium fixes
10. **EX10-025**: Trash card placement now via `request_selection(GamePhase.SelectTrash)` for player choice
11. **EX10-028**: Source selection via `effect_choose_branch` when multiple options; fires OnDigivolutionCardDiscarded; value_fn for CHANGE_DP fixed
12. **P-167**: Deck top/bottom choice added via `effect_choose_branch`; fires OnDigivolutionCardDiscarded
13. **LM-031**: Security card retrieval by identity reference; delay trash selection via `request_selection`

### Low fixes
14. **EX8-047**: `is_optional=True` changed to `False` on reveal
15. **BT16-082**: Reveal `is_optional` set to False; hatch made optional via `effect_choose_branch`
16. **BT20-055**: Added `owner.is_my_turn` check (must be opponent's turn)
17. **P-206**: Reveal `is_optional` set to False

### New card fixes (not in original QA)
18. **BT21-021**: Process5 rewrote to correctly play from hand cost -5 (was incorrectly deleting opponent Digimon); OnDeletion effect implemented
19. **BT23-059**: `register_modifier` arg order fixed (was `ModifierType, perm`, now `perm, ModifierType`)
20. **EX7-074**: Reveal `is_optional` set to False
21. **LM-032**: Target selection via `effect_select_own_permanent` (was auto-selecting first); delay trash selection via `request_selection`
22. **P-123**: Added missing hatch effect (optional via `effect_choose_branch`)
23. **P-130**: Process fixed to suspend self tamer (was selecting opponent permanent to suspend)
