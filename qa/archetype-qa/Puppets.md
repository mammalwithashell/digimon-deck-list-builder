# Archetype QA: Puppets
Date: 2026-03-17 (faithfulness campaign)
Total cards: 57

## Summary
- FAITHFUL: 45
- FIXED: 5 (this campaign)
- DEFERRED: 7 (token deletion recursion, minor auto-selection in non-core)
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT5-033 | PawnChessmon | FAITHFUL | Frozen |
| BT5-106 | Miki & Megumi | FAITHFUL | Frozen |
| BT6-084 | Sistermon Ciel | FAITHFUL | +2000 DP aura for RK/Huckmon |
| BT9-033 | Pillomon | FAITHFUL | Frozen, play-lock descriptive-tagged |
| BT9-112 | Yellow Memory Boost | FAITHFUL | Frozen |
| BT13-101 | Miki & Megumi | FAITHFUL | Play PawnChessmon, suspend on play |
| BT15-003 | Nyaromon | FAITHFUL | Trash top/bottom security branch choice |
| BT16-055 | Namakemon | FAITHFUL | Alt digi Pulsemon, keyword grant |
| BT22-002 | Kyaromon | FIXED | Trait filter: inherited draw on Token/Puppet deletion |
| BT22-029 | Shoemon | FAITHFUL | Puppet Blocker grant, -2000 DP with selection |
| BT22-032 | ShoeShoemon | FAITHFUL | Play Lv3 Puppet filter, -2000 DP with selection |
| BT22-036 | Chaperomon | FAITHFUL | Overclock, hand/main place+digi |
| BT22-040 | Cendrillmon | FIXED | Missing callback: added On other deletion WD activation |
| BT22-042 | Nyabootmon | FAITHFUL | Alt digi, overclock, play+DP, re-activate WD |
| BT22-088 | Arisa Kinosaki | FAITHFUL | Start main, on Token/Puppet play suspend+draw |
| BT22-098 | Unique Emblem | FIXED | Delay condition: Arisa suspend digi Puppet into LIBERATOR cost -3 |
| BT23-077 | Sistermon Ciel | FAITHFUL | also_treated_as, Blocker, delete, De-Digivolve |
| EX4-074 | Sistermon Ciel | FAITHFUL | Frozen |
| EX7-024 | Shoemon | FAITHFUL | Digi cost -1 into Puppet |
| EX7-025 | ShoeShoemon | FAITHFUL | When Digi play Arisa |
| EX7-027 | Chaperomon | FIXED | Prevention flag: Overclock, When Digi play Lv3 Puppet, inherited prevent leaving |
| EX7-030 | Cendrillmon | FAITHFUL | Overclock, start main play token, when attacking DP |
| EX7-063 | Arisa Kinosaki | FAITHFUL | Start main +1 memory, on deletion play Lv3 |
| EX7-074 | Vortex Resonance | FAITHFUL | Reveal 3 add LIBERATOR, digi cost -4 |
| EX8-030 | Tapirmon | FAITHFUL | Frozen |
| EX9-024 | Hanimon | FAITHFUL | Alt digi, trash return Puppet |
| EX9-027 | Kokeshimon | FAITHFUL | Alt digi, trash DP reduction |
| EX9-032 | Karakurumon | FAITHFUL | Alt digi, delete Token/Puppet to digi |
| EX9-033 | Kaguyamon | FAITHFUL | Alt digi, Blocker+Alliance, delete lowest |
| EX9-067 | Mirai Kinosaki | FAITHFUL | Reveal 3 add Puppet/LIBERATOR |
| EX11-019 | Shoemon | FAITHFUL | On Deletion play token, inherited Barrier |
| EX11-020 | Hanimon | FAITHFUL | Alt digi, On Deletion play Shoemon |
| EX11-021 | Kokeshimon | FAITHFUL | Alt digi, When Digi play Mirai |
| EX11-022 | Karakurumon | FAITHFUL | Alt digi, Scapegoat, play Puppet |
| EX11-023 | Kaguyamon | FAITHFUL | Alt digi, Alliance, Scapegoat |
| EX11-024 | Cendrillmon | FAITHFUL | Alliance, Overclock, play Puppet+Tokens |
| EX11-060 | Arisa Kinosaki | FAITHFUL | Memory 3, on Token/Puppet delete suspend+draw |
| EX11-061 | Mirai Kinosaki | FAITHFUL | Memory +1, on Puppet digi suspend+play |
| LM-029 | Yellow Scramble | FAITHFUL | Digi from hand cost -3, delay return yellow Digimon |
| LM-035 | Purple Memory Boost | FAITHFUL | Frozen |
| LM-037 | Black Memory Boost! | FAITHFUL | Reveal 3 add black/yellow |
| P-037 | Yellow Memory Boost | FAITHFUL | Reveal 4 add yellow, delay +2 memory |
| P-105 | Physical Training | FAITHFUL | Reveal 2 add yellow, delay digi -2 |
| P-134 | Shoemon | FAITHFUL | SA -1 to opp, inherited -2000 DP |
| P-156 | Future Potential! | FAITHFUL | Choose Tamer, play matching Digimon |
| P-165 | ShoeShoemon | FIXED | Token EOT deletion: When Digi/On Play token play corrected |
| P-206 | Digital Gate Open | FAITHFUL | Reveal 3, delay play, security |
| P-229 | Narrative Ronde | FAITHFUL | Reveal 3 add Puppet+LIBERATOR |
| ST19-01 | Kyaromon | FAITHFUL | Inherited When Attacking draw |
| ST19-03 | Shoemon | FAITHFUL | Reveal 3 add Puppet+LIBERATOR |
| ST19-04 | PawnChessmon (Y/B) | FAITHFUL | Trash Puppet draw 2, inherited Reboot |
| ST19-05 | PawnChessmon (B/Y) | FAITHFUL | Blocker, On Deletion trash draw 2 |
| ST19-08 | ShoeShoemon | FAITHFUL | Security play LIBERATOR, Overclock |
| ST19-11 | Chaperomon | FAITHFUL | On Play/Digi -3000 DP |
| ST19-12 | Cendrillmon | FAITHFUL | Overclock, Blocker, When Digi play tokens |
| ST19-14 | Arisa Kinosaki | FAITHFUL | Memory 3, on Token/Puppet play suspend+Rush |
| BT22-043 | Terriermon | DEFERRED | CS Tamer filter, card rearrangement cost |

## Fixes Applied (2026-03-17 Campaign)
### BT22-040 Cendrillmon
- Added missing callback for On other deletion WD activation

### EX7-027 Chaperomon
- Corrected prevention flag for leaving-field prevention inherited effect

### BT22-002 Kyaromon
- Corrected trait filter on inherited draw for Token/Puppet deletion

### P-165 ShoeShoemon
- Fixed token end-of-turn deletion handling

### BT22-098 Unique Emblem
- Corrected delay condition for Arisa suspend into LIBERATOR digivolve
