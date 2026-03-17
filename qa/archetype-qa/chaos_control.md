# Archetype QA: Chaos Control
Date: 2026-03-17 (faithfulness campaign)
Total cards: 24

## Summary
- FAITHFUL: 14
- FIXED: 10 (this campaign)
- DEFERRED: 0
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT20-069 | Punkmon | FIXED | alt_digi_level correction |
| BT20-096 | Black Sabbath | FAITHFUL | Trash trigger, main, security all correct |
| BT21-100 | The Digimon I Designed | FIXED | Color ignore bypass implemented |
| BT24-066 | Guilmon | FAITHFUL | 2-pass reveal, inherited delete filter correct |
| BT24-070 | Growlmon | FAITHFUL | Hand count guard, trash play, inherited delete correct |
| BT24-076 | WarGrowlmon | FAITHFUL | Trash Main, cost -2, inherited On Deletion correct |
| BT24-080 | Megidramon | FAITHFUL | also_treated_as, trash EOT digivolve, blocker, delete all lowest level |
| BT7-107 | Calling From the Darkness | FAITHFUL | Proper selection for trash-to-hand |
| EX1-066 | Analog Youth | FIXED | Reveal trashes remaining when no Digimon found; deletion condition checks event_permanent |
| EX10-040 | DemiDevimon | FAITHFUL | Mills top 2 both decks, conditional memory gain correct |
| EX11-005 | Yaamon | FIXED | Digivolves from trash, then trash 2 from hand only if succeeded |
| EX11-047 | Impmon | FAITHFUL | Trash first then gain memory (order corrected) |
| EX11-050 | Loudmon | FIXED | Trash 2, select reference Digimon, delete opp with DP <=; added inherited SA+1 |
| EX11-069 | Yuuki | FIXED | Optional trash-for-memory, digivolve from trash with selection, EOT suspend+select |
| EX3-072 | Megiddo Flame | FAITHFUL | Delete lv4 or lower OR upgrade to lv6, security plays Guilmon from trash |
| EX4-006 | Guilmon | FIXED | Alt-digi correction |
| EX4-011 | ChaosGallantmon | FAITHFUL | Alt digi, trash EOT play from trash, DP-scaled deletion |
| EX7-053 | Eyesmon: Scatter Mode | FAITHFUL | Trash from hand, return trait Digimon from trash; inherited Retaliation |
| EX7-056 | Orochimon | FIXED | p.level correction |
| EX7-060 | Nidhoggmon | FAITHFUL | Trash Main cost -4, Blocker, On Deletion play from trash |
| P-205 | Insane Synthetic Monster | FIXED | 5 issues: process callback, delay delete-as-cost, trash play cost -3, security callback |
| ST10-15 | Darkness Wave | FAITHFUL | Proper selection for trash-to-hand |
| ST16-14 | Matt Ishida | FAITHFUL | Memory set to 3, suspend on hand trash |
| ST6-14 | Matt Ishida | FAITHFUL | Suspend on own Digimon deletion for memory |

## Fixes Applied (2026-03-17 Campaign)
### BT20-069 Punkmon
- Corrected alt_digi_level; Blocker/Retaliation now granted to player-selected Digimon

### BT21-100 The Digimon I Designed
- Delay digivolve now ignores color requirements; selects own Guilmon/Growlmon, then selects from trash to digivolve into

### EX1-066 Analog Youth
- Reveal now trashes remaining when no Digimon found; deletion condition checks event_permanent ownership/level/digi-cards

### EX11-005 Yaamon
- Rewritten: digivolves from trash, then trash 2 from hand only if digivolve succeeded; uses proper digi cost reduction

### EX11-050 Loudmon
- Rewritten: trash 2, select own Dark Dragon/Evil Dragon as DP reference, delete opp with DP <=; added missing inherited SA+1 with trait+hand-count conditions

### EX11-069 Yuuki
- Optional trash-for-memory ("by trashing" = cost); digivolve from trash with proper selection; end-of-turn suspend+select from trash fixed

### EX4-006 Guilmon
- Alt-digi condition corrected

### EX7-056 Orochimon
- p.level reference corrected in deletion effect

### P-205 Insane Synthetic Monster
- Main: added process callback (draw 2, trash 2); delay: delete own Digimon as cost, play from trash with cost -3; security: added callback

### BT24-076/BT24-080/BT24-066/BT24-070
- Already faithful after prior campaign passes
