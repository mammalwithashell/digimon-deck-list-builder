# Archetype QA: Hudiemon
Date: 2026-03-17 (faithfulness campaign)
Total cards: 73

## Summary
- FAITHFUL: 39 (approx)
- FIXED: 12 (this campaign)
- DEFERRED: 3 (ignore color req engine limitation)
- ENGINE GAP: 0
- Not audited: ~20 (generic tech cards shared with other archetypes)

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT10-042 | Venusmon | FAITHFUL | Declarative static effect replacement |
| BT16-077 | Dinobeemon | FAITHFUL | DNA check, FORCE_ATTACK modifier |
| BT22-063 | Alphamon | FAITHFUL | -5000 DP callbacks, unsuspend effect |
| BT22-099 | Kuremi Detective Agency | FAITHFUL | Main, Delay +2 memory, Security |
| BT22-100 | Cyberspace EDEN | FAITHFUL | Main/Security, DP modifier |
| BT23-040 | Wormmon | FIXED | Include trash: alt digi, Erika placement, Hudie DP modifier |
| BT23-041 | Kabuterimon | FAITHFUL | Alliance, OPT suspend trigger, piercing+DP |
| BT23-048 | Gotsumon | FIXED | Fabricated effect: reveal_and_select_multi with Hudie+CS filters corrected |
| BT23-050 | Ankylomon | FAITHFUL | -2000 DP + DNA digivolve |
| BT23-051 | Golemon | FAITHFUL | Alliance, blocker, can't attack, OPT delete |
| BT23-058 | Craniamon | FAITHFUL | WhenRemoveField _will_not_be_removed |
| BT23-059 | Justimon: Blitz Arm | FAITHFUL | Trash option + delete, unsuspend+immunity |
| BT23-081 | Hudiemon | FIXED | Missing effect: added missing On Play/WD effect |
| BT23-084 | Erika Mishima | FAITHFUL | Memory gain, bounce+play, inherited alliance |
| BT23-085 | Ryuji Mishima | FAITHFUL | Memory gain, DP immunity+reboot+blocker |
| BT23-089 | Takumi Aiba | FAITHFUL | WhenRemoveField substitute with trash selection |
| BT23-090 | Nokia Shiramine | FIXED | Filter+bounce: corrected filter conditions and bounce targeting |
| BT23-091 | Wolkenapalm | FAITHFUL | min() crash fixed |
| BT23-092 | Ice Archery | FAITHFUL | CANNOT_SUSPEND modifier |
| BT23-095 | Crescent Leaf | FIXED | Delay copy-paste: CS trait check added to Delay condition |
| BT23-096 | Comet Hammer | FIXED | Delay copy-paste: CS trait check added to Delay condition |
| BT23-100 | Hudie Net Cafe | FAITHFUL | Delay and Security correct |
| BT23-101 | Cyberspace EDEN | FIXED | Missing callback: added missing process callback |
| BT22-054 | Stingmon | FIXED | Missing callback: added missing process callback |
| BT22-089 | Mirei Mikagura | FIXED | Costs: corrected cost handling |
| BT22-093 | Arata Sanada | FIXED | Wrong tamers: corrected tamer targeting |
| BT22-101 | Digital Gate | FIXED | Wrong tamers: corrected tamer targeting |
| BT23-032 | Wormmon | FIXED | Wrong zone: corrected zone reference |
| EX10-068 | Digimon Emperor | FAITHFUL | Memory gain, delete filter, execution order |
| P-225 | DigiLab | FAITHFUL | Delay effect verified |

## Fixes Applied (2026-03-17 Campaign)
### BT23-095 Crescent Leaf / BT23-096 Comet Hammer
- Both had Delay conditions copy-pasted without CS trait check; added CS trait verification

### BT23-101 Cyberspace EDEN
- Added missing callback for effect activation

### BT23-081 Hudiemon
- Added missing On Play/When Digivolving effect

### BT22-054 Stingmon
- Added missing process callback

### BT22-089 Mirei Mikagura
- Corrected cost handling for start-of-main return and play effects

### BT22-093 Arata Sanada / BT22-101 Digital Gate
- Corrected wrong tamer targeting in both scripts

### BT23-048 Gotsumon
- Removed fabricated effect; corrected reveal_and_select_multi with proper Hudie+CS filters

### BT23-090 Nokia Shiramine
- Corrected filter conditions and bounce targeting

### BT23-040 Wormmon
- Corrected to include trash as valid source zone

### BT23-032 Wormmon
- Corrected wrong zone reference
