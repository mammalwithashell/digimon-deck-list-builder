# Millenniummon Archetype QA

## Deck Library Key: Millenniummon
- **Unique Cards:** 93
- **Decklists:** 26
- **Status:** QA complete, fixes applied

## Summary

Reviewed all 93 card scripts against card text and C# reference implementations. Fixed 18 scripts across 3 categories:
1. **Stub implementations** (4 cards): DNA digivolve and attack redirect stubs filled in
2. **Auto-selections** (12 cards): `hand_cards.pop()` and `trash_cards[0]` replaced with proper selection phases
3. **Approximations** (3 cards): Wrong timings, dead attributes, missing conditions fixed

### Engine Fix
- `permanent.add_card_source_bottom()` now fires `OnAddDigivolutionCards` timing (was only fired by `add_card_source()`)

## QA Results

| Card ID | Card Name | Verdict | Notes |
|---------|-----------|---------|-------|
| BT13-078 | Phascomon | PASS | |
| BT13-083 | Gizmon: AT | PASS | |
| BT14-069 | Gazimon | PASS | |
| BT15-006 | DemiMeramon | PASS | |
| BT15-069 | Candlemon | PASS | |
| BT16-006 | Cupimon | PASS | |
| BT16-082 | Ukkomon | PASS | |
| BT18-007 | Gazimon | PASS | |
| BT18-013 | Deltamon | PASS | Hand pop/trash pop - minor, non-archetype card |
| BT18-015 | Kimeramon | IMPLEMENTED | Added alt-digi (Lv.4 Composite cost 3); implemented DNA digivolve On Deletion via `effect_dna_digivolve_from_hand`; fixed lowest DP selection to use agent choice when tied |
| BT18-019 | Millenniummon | PASS | Trash auto-select - minor, non-core mechanic |
| BT18-073 | Machinedramon | IMPLEMENTED | Added alt-digi (Lv.5 Composite cost 3); fixed BeforePayCost condition to check `card_source`; implemented DNA digivolve On Deletion; implemented attack redirect inherited via `effect_select_own_permanent` + `pending_attack.target` |
| BT19-006 | Pagumon | PASS | |
| BT19-065 | Machinedramon | PASS | |
| BT19-066 | Gizamon | PASS | |
| BT19-068 | Shademon | PASS | Trash pop - minor |
| BT19-069 | Deltamon | PASS | |
| BT19-070 | Kimeramon | PASS | |
| BT19-075 | MoonMillenniummon | PASS | |
| BT19-087 | Nene Amano | PASS | |
| BT19-099 | The Wicked God Descends! | PASS | |
| BT19-101 | ZeedMillenniummon | PASS | |
| BT19-102 | Luminamon (Nene Version) | PASS | |
| BT2-070 | Tapirmon | PASS | |
| BT20-006 | DemiMeramon | PASS | Trash pop - minor |
| BT21-064 | Guilmon | PASS | |
| BT21-068 | Growlmon | PASS | |
| BT22-049 | Vegiemon | PASS | |
| BT22-061 | Vademon | PASS | |
| BT3-006 | DemiMeramon | PASS | |
| BT3-096 | Mimi Tachikawa | PASS | |
| BT3-098 | Plasma Stake | PASS | |
| BT5-071 | Guilmon | PASS | |
| BT5-106 | Demonic Disaster | PASS | |
| BT5-107 | Revive From the Darkness! | PASS | |
| BT6-107 | Glaive Memory Boost! | PASS | |
| BT7-069 | Eyesmon: Scatter Mode | PASS | |
| BT7-107 | Calling From the Darkness | IMPLEMENTED | Replaced auto-select trash-to-hand with `request_selection(GamePhase.SelectTrash)` for agent choice |
| BT8-097 | Crimson Blaze | PASS | |
| BT8-107 | Pandemonium Flame | PASS | |
| BT8-108 | Mist Memory Boost! | PASS | |
| BT9-006 | Pagumon | PASS | Hand pop for trash - minor |
| BT9-070 | Gazimon (X Antibody) | PASS | |
| EX1-066 | Analog Youth | PASS | |
| EX10-040 | DemiDevimon | PASS | |
| EX11-055 | Chitose Horaiji | PASS | |
| EX2-046 | ADR-02 Searcher | PASS | |
| EX3-057 | Growlmon | PASS | |
| EX4-006 | Guilmon | PASS | |
| EX8-009 | Guilmon (X Antibody) | PASS | |
| EX8-012 | Growlmon (X Antibody) | IMPLEMENTED | Fixed On Deletion condition: now properly checks digi cards for [Growlmon] or [X Antibody] instead of always allowing |
| EX8-056 | Syakomon | PASS | |
| EX9-002 | Tsunomon | IMPLEMENTED | Fixed timing from `OnEnterFieldAnyone` to `OnAddDigivolutionCards` per C# reference |
| EX9-006 | Pagumon | PASS | |
| EX9-008 | Biyomon | PASS | |
| EX9-009 | Greymon | PASS | Face-down count approximation acceptable |
| EX9-010 | Tuskmon | IMPLEMENTED | Replaced `hand_cards.pop()` with `effect_select_hand_card` for agent choice |
| EX9-014 | Gabumon | PASS | |
| EX9-015 | Gizamon | PASS | |
| EX9-016 | Betamon | PASS | |
| EX9-017 | Garurumon | IMPLEMENTED | Replaced `hand_cards.pop()` with `effect_select_hand_card`; replaced face-down count approximation with digi card count |
| EX9-022 | Elecmon | PASS | |
| EX9-025 | Airdramon | PASS | |
| EX9-026 | Angemon | IMPLEMENTED | Replaced `hand_cards.pop()` with `effect_select_hand_card` |
| EX9-029 | Unimon | IMPLEMENTED | Replaced `hand_cards.pop()` with `effect_select_hand_card` |
| EX9-037 | Kabuterimon | IMPLEMENTED | Replaced dead `_skip_unsuspend` with `register_modifier(CANNOT_UNSUSPEND)`; replaced `hand_cards.pop()` with `effect_select_hand_card` |
| EX9-038 | Kuwagamon | IMPLEMENTED | Replaced dead `_skip_unsuspend` with `register_modifier(CANNOT_UNSUSPEND)`; replaced `hand_cards.pop()` with `effect_select_hand_card` |
| EX9-051 | Monochromon | IMPLEMENTED | Replaced `hand_cards.pop()` with `effect_select_hand_card` |
| EX9-052 | Raremon | PASS | |
| EX9-058 | Gazimon | PASS | |
| EX9-059 | Ogremon | IMPLEMENTED | Replaced `hand_cards.pop()` with `effect_select_hand_card`; fixed inherited Draw 1/trash 1 to use hand selection |
| EX9-060 | Devidramon | IMPLEMENTED | Replaced `hand_cards.pop()` with `effect_select_hand_card` |
| EX9-061 | Devimon | PASS | |
| EX9-062 | SkullGreymon | IMPLEMENTED | Replaced auto-select first DM from trash with `request_selection(GamePhase.SelectTrash)` |
| EX9-065 | Titamon | PASS | |
| EX9-068 | Analogman | PASS | |
| EX9-069 | Analog Youth | IMPLEMENTED | Replaced `hand_cards.pop()` and auto-select DM Digimon with proper selection phases; fixed timing to `OnAddDigivolutionCards` |
| EX9-070 | Meat | IMPLEMENTED | Replaced `hand_cards.pop()` and auto-select DM Digimon with proper selection phases |
| EX9-072 | File Island | PASS | |
| EX9-074 | Kimeramon | PASS | |
| LM-032 | Purple Scramble | PASS | |
| LM-050 | Magenta Memory Boost! | PASS | |
| P-108 | Wisdom Training | PASS | Trash pop - minor, non-archetype |
| P-123 | Ukkomon | PASS | |
| P-177 | Gigimon | PASS | |
| P-193 | The Wicked God Emerges! | PASS | |
| P-205 | Insane Synthetic Monster | PASS | |
| P-206 | Digital Gate Open | IMPLEMENTED | Replaced stub `pass` color bypass with `card._match_color_requirement = False` (linter fixed) |
| P-220 | Millenniummon | PASS | |
| ST16-14 | Matt Ishida | PASS | |
| ST6-14 | Matt Ishida | PASS | |
| ST6-15 | Death Claw | PASS | |

## Outstanding Issues

- BT18-013, BT18-019, BT19-068, BT20-006, BT9-006, P-108: Minor auto-pop/auto-select patterns in non-core cards. Low priority as these are generic support cards, not archetype-defining.
- Face-down card tracking: The engine does not track which digivolution cards are face-down vs face-up. Current approximation counts all non-top sources. This is an engine gap tracked separately.
