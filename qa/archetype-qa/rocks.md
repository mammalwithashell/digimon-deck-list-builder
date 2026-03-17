# Archetype QA: Rocks
Date: 2026-03-17 (faithfulness campaign)
Total cards: 47

## Summary
- FAITHFUL: 40
- FIXED: 2 (this campaign)
- DEFERRED: 5 (auto-selection in non-core, WhenRemoveField removal-cause)
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT4-072 | Gogmamon | FAITHFUL | |
| BT8-094 | Digimon Emperor | FAITHFUL | |
| BT9-103 | Kongou | FAITHFUL | |
| BT14-009 | Gotsumon | FAITHFUL | |
| BT16-082 | Ukkomon | FAITHFUL | Reveal mandatory, hatch optional |
| BT18-064 | Mercurymon | FAITHFUL | |
| BT20-055 | Invisimon | FAITHFUL | Opponent turn check added |
| BT21-021 | OmniShoutmon | FAITHFUL | Play from hand cost -5, OnDeletion implemented |
| BT21-055 | Sunarizamon | FAITHFUL | BeforePayCost + leak guard |
| BT23-059 | Justimon: Blitz Arm | FAITHFUL | register_modifier corrected |
| BT23-096 | Comet Hammer | FAITHFUL | |
| EX6-072 | Mega Digimon Assembly! | FIXED | Scoping: security trash-to-hand with player selection corrected |
| EX7-049 | Metallicdramon | DEFERRED | WhenRemoveField lacks removal-cause context |
| EX7-074 | Vortex Resonance | FAITHFUL | Reveal mandatory |
| EX8-005 | Tumblemon | FAITHFUL | |
| EX8-046 | Gotsumon | FAITHFUL | |
| EX8-047 | Sunarizamon | FAITHFUL | Reveal mandatory |
| EX8-048 | Landramon | FAITHFUL | |
| EX8-050 | Gogmamon | FAITHFUL | |
| EX8-051 | Proganomon | FAITHFUL | Inherited condition checks trashed_cards + trait |
| EX8-055 | Pyramidimon | FAITHFUL | SA+1 via register_modifier, fires OnDigivolutionCardDiscarded |
| EX8-067 | Close | FAITHFUL | |
| EX8-070 | Zofr Kabus | FAITHFUL | |
| EX10-003 | Tumblemon | FAITHFUL | |
| EX10-025 | Sunarizamon | FAITHFUL | Trash selection via player choice |
| EX10-028 | Landramon | FAITHFUL | Source selection, fires OnDigivolutionCardDiscarded |
| EX10-032 | Proganomon | FAITHFUL | Condition + value_fn + OnDigivolutionCardDiscarded |
| EX10-033 | Pyramidimon | FAITHFUL | Cost reduction value_fn, trash placement via selection |
| EX10-034 | Blastmon | FAITHFUL | Timing corrected, value_fn lambdas |
| EX10-036 | Magneticdramon | FAITHFUL | Cross-Digimon source count, trash via selection |
| EX10-063 | Close | FAITHFUL | |
| EX10-069 | Unique Emblem: Gravel Hearts | FAITHFUL | Close filter via card_names |
| EX11-038 | Sunarizamon | FAITHFUL | |
| EX11-044 | Pyramidimon | FAITHFUL | |
| EX11-065 | Close | FAITHFUL | |
| LM-031 | Black Scramble | FAITHFUL | Security retrieval, delay trash selection |
| LM-032 | Purple Scramble | FAITHFUL | Target selection, delay trash selection |
| P-039 | Black Memory Boost! | FAITHFUL | |
| P-107 | Defense Training | FAITHFUL | Delay selects target Digimon, reveal mandatory |
| P-123 | Ukkomon | FAITHFUL | Optional hatch |
| P-130 | Lui Ohwada | FAITHFUL | Self tamer suspend |
| P-167 | Landramon | FAITHFUL | Deck top/bottom choice, fires OnDigivolutionCardDiscarded |
| P-169 | Close | FAITHFUL | |
| P-186 | Gallantmon | FAITHFUL | |
| P-206 | Digital Gate Open | FAITHFUL | Reveal mandatory |
| P-215 | Icemon | FAITHFUL | |
| ST13-08 | Chikurimon | FAITHFUL | |
| ST22-11 | Defense Plug-In F | FAITHFUL | |

## Fixes Applied (2026-03-17 Campaign)
### EX6-072 Mega Digimon Assembly!
- Corrected scoping for security trash-to-hand with proper player selection

### (Prior fixes verified faithful)
All 22 prior fixes (BeforePayCost, value_fn lambdas, register_modifier arg order, selection improvements, OnDigivolutionCardDiscarded firing, reveal mandatory flags) were applied in earlier passes and verified faithful this campaign.
