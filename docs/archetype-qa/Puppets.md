# Archetype QA: Puppets
Date: 2026-03-13
Total cards: 57

## Summary
- Frozen: 29 (QA pending — not yet reviewed in this pass)
- Unfrozen (prior reviewed): 6 (BT5-033, BT5-106, BT9-033, BT9-112, EX4-074, LM-035)
- IMPLEMENTED: 22 new scripts (all with C# reference)
- BLOCKED: 0

## Implemented Cards

### ST19 Batch (8 cards — Starter Deck)
| Card | Name | Key Effects |
|------|------|-------------|
| ST19-01 | Kyaromon | Digi-Egg. Inherited: [When Attacking] OPT Draw 1 if another Digimon |
| ST19-03 | Shoemon | On Play: reveal 3, add Puppet + LIBERATOR. Inherited: opp security -3000 DP |
| ST19-04 | PawnChessmon (Y/B) | On Play: trash Puppet → Draw 2. Inherited: Reboot |
| ST19-05 | PawnChessmon (B/Y) | Blocker. On Deletion: trash Puppet → Draw 2 |
| ST19-08 | ShoeShoemon | Security: play LIBERATOR cost<=4 free. Overclock (Puppet). Inherited: opp security -3000 DP |
| ST19-11 | Chaperomon | On Play/Digi: opp -3000 DP (-6000 if 3+ total). Inherited: prevent leaving via Puppet/Token delete |
| ST19-12 | Cendrillmon | Overclock (Puppet). Blocker. When Digi: play 2 Familiar Tokens |
| ST19-14 | Arisa Kinosaki | Tamer. Start turn: memory 3. When Token/Puppet played by effect: suspend → grant Rush |

### EX7 Batch (6 cards — Puppet Engine)
| Card | Name | Key Effects |
|------|------|-------------|
| EX7-024 | Shoemon | Digi cost -1 into Puppet. Inherited: opp security -3000 DP |
| EX7-025 | ShoeShoemon | When Digi: play Arisa if <=1 Tamer. Inherited: opp security -3000 DP |
| EX7-027 | Chaperomon | Overclock (Puppet). When Digi: play Lv3 Puppet free. Inherited: prevent leaving via Token/Puppet delete |
| EX7-030 | Cendrillmon | Overclock. Start Main: play Familiar Token. When Digi: play Familiar Token. When Attacking: opp -6000 DP |
| EX7-063 | Arisa Kinosaki | Tamer. Start Main: +1 memory. On Token/Puppet deletion: play Lv3 Puppet free |
| EX7-074 | Vortex Resonance | Option. Reveal 3, add LIBERATOR, digi cost -4. Security: play LIBERATOR cost<=4 |

### EX9+LM+BT6 Batch (8 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| EX9-024 | Hanimon | Alt digi from Kyaromon. On Play: trash 1 → return Puppet from trash. Inherited: end attack |
| EX9-027 | Kokeshimon | Alt digi Puppet. When Digi/On Deletion: trash 1 → opp -4000 DP. Inherited: end attack |
| EX9-032 | Karakurumon | Alt digi Puppet. On Play/Digi: delete Token/Puppet → digi from hand free. Inherited: prevent leaving |
| EX9-033 | Kaguyamon | Alt digi Puppet. Blocker+Alliance for Tokens/Puppets. On other delete: delete opp lowest level. End turn: play Lv4- Puppet from trash |
| EX9-067 | Mirai Kinosaki | Tamer. On Play: reveal 3, add Puppet/LIBERATOR. On Puppet digi: return self, play with cost -3 |
| LM-029 | Yellow Scramble | Option. Digi from hand cost -3, Delay. Security: play yellow DP<=2000 |
| LM-037 | Black Memory Boost! | Option. Reveal 3, add black/yellow Digimon. Delay +2 memory |
| BT6-084 | Sistermon Ciel | Tamer. Aura +2000 DP for Royal Knight/Huckmon. On Play: +1 memory |

### Token Added
- `familiar` token registered in `token_registry.py` — Yellow Digimon, 3000 DP, On Deletion: opp Digimon -3000 DP

## Smoke Test
- 50/50 mirror games completed
