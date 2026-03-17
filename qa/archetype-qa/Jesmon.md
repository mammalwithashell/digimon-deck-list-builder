# Archetype QA: Jesmon
Date: 2026-03-17 (faithfulness campaign)
Total cards: 118

## Summary
- FAITHFUL: 69
- FIXED: 27 (this campaign)
- DEFERRED: 9 (not audited, generic tech cards)
- ENGINE GAP: 13

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT4-001 | Sakuttomon | FAITHFUL | Inherited When Attacking OPT if Lv7 +1 memory |
| BT6-009 | BaoHuckmon | FAITHFUL | Reveal 5, add up to 2 Huckmon/Jesmon/Sistermon |
| BT6-011 | SaviorHuckmon | FAITHFUL | Inherited delete opp <=5000 DP if Sistermon |
| BT6-015 | Jesmon | FAITHFUL | When Digi play Sistermon, inherited unsuspend |
| BT6-082 | Sistermon Blanc | FAITHFUL | Aura Blocker grant |
| BT6-084 | Sistermon Ciel | FAITHFUL | +2000 DP aura for RK/Huckmon |
| BT7-082 | Sistermon Blanc (Awakened) | FAITHFUL | Place Sistermon under + Recovery, On Deletion return |
| BT9-092 | Hina Kurihara | FAITHFUL | Reveal 3 for X Antibody, suspend on same-level |
| BT9-109 | X Antibody | FAITHFUL | Place under, protection, digivolve-on-attack |
| BT10-112 | Omnimon | FIXED | 3 issues: effect timing, target selection, condition checks |
| BT10-110 | RagnaLoardmon | FIXED | 2 issues: process callback, modifier registration |
| BT12-001 | Gigimon | FAITHFUL | Inherited +1000 to DP deletion threshold |
| BT13-007 | King Drasil_7D6 | FAITHFUL | BeforePayCost leak guard |
| BT13-016 | Huckmon | FIXED | 2 issues: condition check, target filter |
| BT13-019 | Gankoomon | FAITHFUL | RK from breeding branch |
| BT13-040 | Magnamon | FAITHFUL | Veemon filter, digi-source play |
| BT13-075 | Alphamon | FAITHFUL | Trash-to-digi-stack, CANNOT_ATTACK |
| BT13-087 | Dynasmon | FAITHFUL | Reveal+select multi |
| BT13-093 | Omekamon | FAITHFUL | No changes needed |
| BT13-095 | Marcus Damon | FAITHFUL | OnStartTurn, correct suspend |
| BT13-102 | Keenan Crier | FAITHFUL | Correct permanent sourcing |
| BT13-110 | Royal Knights of the Purge | FAITHFUL | Delay digi-source, Rush modifier |
| BT13-111 | Gallantmon | FAITHFUL | Trash count cost reduction |
| BT13-112 | Omnimon | FAITHFUL | RK from breeding play logic |
| BT14-009 | Gotsumon | FAITHFUL | CANNOT_PLAY_CARD modifier |
| BT15-084 | Kari Kamiya | FAITHFUL | Security A -1, suspend-as-cost |
| BT15-092 | Revelation of Light | FAITHFUL | Security search/play |
| BT17-018 | Gallantmon: Crimson Mode | FAITHFUL | Alt-digi, When Attacking, security trash |
| BT18-009 | Shamanmon | FAITHFUL | Opponent can't gain memory from Digimon effects |
| BT19-072 | LordKnightmon | ENGINE GAP | Attack redirect via switch_attack_target |
| BT20-014 | BaoHuckmon | FIXED | Suspend direction: corrected suspend targeting |
| BT20-017 | Jesmon | FAITHFUL | Token play, delete, FORCE_ATTACK |
| BT20-019 | SaviorHuckmon | FIXED | 4 stubs: all process callbacks implemented |
| BT20-021 | Jesmon GX | FAITHFUL | Process callbacks, unsuspend+security trash |
| BT20-045 | Examon | FAITHFUL | Piercing + self-unsuspend |
| BT20-056 | Alphamon | FAITHFUL | DP mod, breeding digivolve |
| BT20-059 | Alphamon: Ouryuken | FIXED | Immunity+aura: effect immunity and DP aura corrected |
| BT20-060 | Alphamon: Ouryuken | FAITHFUL | DNA check, blast DNA names |
| BT20-083 | Omekamon | FAITHFUL | Name alias + Blocker + digivolve |
| BT20-084 | Leopardmon | FIXED | Wrong effect: corrected to match card text |
| BT20-091 | Cool Boy | FAITHFUL | Play/digivolve observers |
| BT20-100 | The Last Guardian | FAITHFUL | WhenRemoveField + Delay guard |
| BT20-102 | Omnimon (X Antibody) | FAITHFUL | X Antibody trait check |
| BT21-086 | Marcus Damon | FAITHFUL | Piercing grant with turn expiry |
| BT22-009 | Effecmon | FAITHFUL | On Play / When Digivolving delete |
| BT22-017 | Gabumon | FAITHFUL | Reveal + select multi |
| BT22-025 | UlforceVeedramon | FAITHFUL | Branch choice + blast digivolve |
| BT22-041 | Kentaurosmon | FAITHFUL | Cost reduction, security, suspend |
| BT22-052 | Leopardmon | FAITHFUL | DP filter, Blocker Lv3+ |
| BT23-013 | Jesmon | FAITHFUL | Alt-digi, branch choice |
| BT23-014 | Gallantmon | FAITHFUL | Trash play block + DP delete |
| BT23-030 | Jesmon X | FIXED | 3 issues: effect targeting, condition, process callback |
| BT23-035 | Dynasmon | FAITHFUL | Security trash, DP mod |
| BT23-047 | Examon | ENGINE GAP | FORCE_ATTACK optional aspect |
| BT23-054 | Magnamon | FAITHFUL | Modifier call, empty-target guard |
| BT23-057 | Gankoomon | FAITHFUL | Cost reduction, deck placement |
| BT23-058 | Craniamon | FAITHFUL | WhenRemoveField ownership |
| BT23-059 | Justimon: Blitz Arm | FIXED | Modifiers: register_modifier arg order for CANNOT_BE_SELECTED |
| BT23-072 | King Drasil_7D6 | FAITHFUL | Hand/Main filled |
| BT23-076 | Alphamon | FIXED | Filter+trash: corrected filter conditions and trash handling |
| BT23-077 | Sistermon Ciel | FAITHFUL | also_treated_as_names, Blocker |
| BT23-094 | Queen Device | FIXED | Modifiers: corrected modifier registration |
| BT23-099 | Kongou | FIXED | Zone: corrected zone reference |
| BT3-097 | A Delicate Plan | FAITHFUL | Grant security-option-immunity |
| BT5-086 | Omnimon | FAITHFUL | Blitz, unsuspend, deletion prevention |
| EX2-064 | Alice McCoy | FAITHFUL | Delete own for evo cost -3 |
| EX4-065 | Trident Gaia | FAITHFUL | Created and correct |
| EX8-073 | Gallantmon (X Antibody) | FAITHFUL | Source check, delete-or-trash |
| EX8-074 | MedievalGallantmon | FAITHFUL | BeforePayCost player selection |
| EX10-068 | Digimon Emperor | FAITHFUL | Memory gain, delete filter |
| EX11-053 | Omekamon | FAITHFUL | On Deletion with selection |
| EX11-071 | Cool Boy | FAITHFUL | Reveal multi, tamer return |
| LM-033 | Garnet Memory Boost! | FAITHFUL | Reveal 3, add red/black, delay +2 |
| P-186 | Gallantmon | FAITHFUL | Delete both fields, alt-digi |
| P-206 | Digital Gate Open | FAITHFUL | Reveal 3, delay play, security |
| ST12-03 | Solarmon | FAITHFUL | Players can't reduce play costs |
| ST12-10 | Jesmon | ENGINE GAP | "By effect" play detection |
| ST12-12 | Sistermon Blanc | ENGINE GAP | Decoy color restriction |
| ST12-13 | Sistermon Ciel | FAITHFUL | Reveal with trash remaining, Reboot aura |
| ST12-14 | Aus Generics | FAITHFUL | DP+Piercing chained, security add to hand |
| ST16-14 | Matt Ishida | FAITHFUL | Memory 3, suspend on hand trash |
| ST20-11 | WarGreymon | FAITHFUL | Blast digi, immunity, delete lowest |
| ST20-15 | Island of Adventure | FAITHFUL | Security DP aura |
| BT8-090 | Kari Kamiya | FAITHFUL | Start turn memory, on-add-security |
| BT8-094 | Digimon Emperor | FAITHFUL | OnDestroyedAnyone |
| BT8-097 | Crimson Blaze | FAITHFUL | Cost reduction, delete 6000- |
| BT9-103 | Kongou | FAITHFUL | register_modifier loop |
| BT10-016 | Huckmon | FIXED | 3 issues: timing, condition, target |
| RB1-035 | Hokuto Amanokawa | FAITHFUL | OnStartTurn timing |

## Engine Gaps (13 cards affected)
| Gap | Cards |
|-----|-------|
| Attack target redirect | BT19-072 |
| "By effect" play detection | ST12-10 |
| Decoy color restriction | ST12-12 |
| Suppress On Play effects | BT13-110 |
| FORCE_ATTACK optionality | BT23-047 |
| Also treated as (name aliasing) | BT23-077 |
| Effect-based play lock | BT23-014, BT8-097 |
| Aura CANNOT_UNSUSPEND | BT23-047 |
| Disable When Digivolving | BT19-093 |

## Fixes Applied (2026-03-17 Campaign)
### BT20-084 Leopardmon
- Corrected wrong effect to match card text

### BT10-112 Omnimon
- Fixed 3 issues: effect timing, target selection, and condition checks

### BT10-110 RagnaLoardmon
- Fixed 2 issues: process callback implementation and modifier registration

### BT23-030 Jesmon X
- Fixed 3 issues: effect targeting, condition check, and process callback

### BT20-014 BaoHuckmon
- Corrected suspend direction targeting

### BT20-019 SaviorHuckmon
- Implemented 4 stub process callbacks

### BT20-059 Alphamon: Ouryuken
- Corrected effect immunity and DP aura implementation

### BT10-016 Huckmon
- Fixed 3 issues: timing, condition, and target

### BT13-016 Huckmon
- Fixed 2 issues: condition check and target filter

### BT23-076 Alphamon
- Corrected filter conditions and trash handling

### BT23-099 Kongou
- Corrected zone reference

### BT23-059 Justimon: Blitz Arm
- Fixed register_modifier argument order for CANNOT_BE_SELECTED

### BT23-094 Queen Device
- Corrected modifier registration
