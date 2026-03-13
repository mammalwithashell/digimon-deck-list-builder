# Archetype QA: DNA Omnimon
Date: 2026-03-13
Total cards: 47

## Summary
- Frozen: 23 (QA pending)
- Unfrozen (prior done): 1 (ST20-15)
- IMPLEMENTED: 23 new scripts (all with C# reference)
- BLOCKED: 0

## Implemented Cards

### BT17 Batch (9 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT17-007 | Agumon | Alt digi Koromon. Start Main: return Greymon/Garurumon/Omnimon from trash. Inherited: end-turn DNA digi |
| BT17-015 | WarGreymon | Alt digi Greymon. Cost -3 w/ Tai. On Play/Digi: delete 8000- OR digi Gabumon→MetalGarurumon. Inherited: trash security if Omnimon |
| BT17-019 | Gabumon | Alt digi Tsunomon. Start Main: draw if Matt. Inherited: end-turn DNA digi |
| BT17-027 | MetalGarurumon | Alt digi Garurumon. Cost -3 w/ Matt. On Play/Digi: can't-suspend 1 OR digi Agumon→WarGreymon. Inherited: unsuspend if Omnimon |
| BT17-078 | Omnimon | Raid+Blocker+Blast DNA. On Play/Digi: if DNA, bottom-deck all opp Digimon of chosen level + delete 1 |
| BT17-081 | Tai & Matt | Tamer. Suspend on play/digi for memory. End turn: Omnimon attacks. Security: play free |
| BT17-093 | Kari Kamiya | Tamer. Suspend on hatch +1 memory. End turn: return self, draw, play tamer |
| BT17-095 | Brave Tornado | Option. Play Agumon/Gabumon free. Delay: DNA digi protection. Security: play tamer |
| BT17-102 | Agumon -Bond- | Alt digi Agumon. When Digi: +3000 DP if Koromon, delete opp <=DP. On Deletion: play tamer/hatch |

### BT12 (1 card)
| Card | Name | Key Effects |
|------|------|-------------|
| BT12-059 | Agumon (Black) | Alt digi Koromon. On Play: reveal 4, add Greymon/Omnimon + Tai Kamiya. Inherited: +1000 DP if Greymon/Omnimon |

### EX4+EX9 Batch (6 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| EX4-038 | Greymon (X) | On Play: reveal 3, add Greymon + Gabumon/Garurumon/Omnimon. Inherited: +1 memory on other digi |
| EX4-039 | Garurumon (X) | On Play: reveal 3, add Garurumon + Agumon/Greymon/Omnimon. Inherited: +1 memory on other digi |
| EX4-061 | Tai & Matt | Tamer. Suspend on Gabumon/Agumon play. On digi: play partner free. Security: play free |
| EX4-073 | Omnimon Alter-S | Alt digi Omnimon. When Digi: de-digi 3 + budget delete. When Attacking: trash evo cards for deletes |
| EX9-021 | Omnimon Alter-S | DNA Blue+Red Lv6. When Digi: if DNA, immunity + delete highest level. End Attack: play from evo cards |
| EX9-066 | Tai & Matt | Tamer. On Play: return Greymon/Garurumon/Omnimon from trash. Suspend: memory per Greymon+Garurumon |

### BT5+ST+LM+EX1 Batch (7 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT5-092 | Matt Ishida | Tamer. On Play: play Agumon/Gabumon. Main: digi cost -1 for Garurumon/Omnimon/Greymon |
| BT5-093 | Tai & Matt | Tamer. Start turn: +2 memory if opp has Lv6+. SA+1 for Omnimon |
| ST2-13 | Matt Ishida | Option. Main: +1 memory. Security: +2 memory |
| ST20-10 | Agumon | Alt digi Adventure/Hero. Warp digi into WarGreymon. Inherited: Reboot |
| ST20-11 | WarGreymon | Alt digi Adventure/Hero. Blast digi. On Play/Digi: immunity to N Digimon. When Digi/Attack: delete lowest DP |
| LM-034 | Blue Memory Boost! | Reveal 3, add blue/red Digimon. Delay +2 memory. Security: place in BA |
| EX1-021 | Plesiomon | When Digi: +1 memory per 4 hand cards. When Attacking: bottom-deck opp with On Deletion |

## Smoke Test
- 50/50 mirror games completed
