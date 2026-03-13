# Archetype QA: Jesmon
Date: 2026-03-13
Total cards: 118

## Summary
- Frozen: 87 (QA pending)
- Unfrozen (prior reviewed): 16
- IMPLEMENTED: 15 new scripts (13 with C#, 2 from API)
- BLOCKED: 0

## Implemented Cards

### Batch 1 (8 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT6-009 | BaoHuckmon | On Play: reveal 5, add up to 2 Huckmon/Jesmon/Sistermon |
| BT6-011 | SaviorHuckmon | Inherited: [When Attacking] OPT delete opp <=5000 DP if Sistermon in play |
| BT6-015 | Jesmon | When Digi: play Sistermon free. Inherited: unsuspend if Sistermon in play |
| BT7-082 | Sistermon Blanc (Awakened) | On Play: place Sistermon Blanc under + Recovery +1. On Deletion: return Jesmon/Huckmon/Sistermon from trash |
| BT9-092 | Hina Kurihara | Tamer. On Play: reveal 3 for X Antibody. Suspend on same-level X digi for +1 memory + Draw 1 |
| BT9-109 | X Antibody | Option. Place under Digimon as digi-card. Inherited: protect X digi-cards + digi into X Antibody on attack |
| BT4-001 | Sakuttomon | Digi-Egg. Inherited: [When Attacking] OPT if Lv7, +1 memory |
| ST12-03 | Solarmon | [All Turns] Players can't reduce play costs |

### Batch 2 (7 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT12-001 | Gigimon | Digi-Egg. Inherited: +1000 to DP deletion threshold |
| BT18-009 | Shamanmon | [All Turns] Opponent can't gain memory from Digimon effects |
| BT3-097 | A Delicate Plan | Option. Grant security-option-immunity. Security: add to hand |
| BT5-086 | Omnimon | Blitz. When Digi: unsuspend. Prevent deletion by trashing Lv6 from digi-stack |
| EX2-064 | Alice McCoy | Tamer. BeforePayCost: delete own Digimon for evo cost -3 (Lv5→Lv6). Security: play free |
| LM-033 | Garnet Memory Boost! | Reveal 3, add red/black Digimon. Delay +2 memory. Security: place in BA |
| ST16-14 | Matt Ishida | Tamer. Start turn: memory 3. On hand trash: suspend for +1 memory. Security: play free |

## Smoke Test
- 50/50 mirror games completed
