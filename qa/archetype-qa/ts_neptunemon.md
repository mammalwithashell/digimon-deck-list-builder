# Archetype QA: TS Neptunemon
Date: 2026-03-17 (faithfulness campaign)
Total cards: 30

## Summary
- FAITHFUL: 19
- FIXED: 7 (this campaign)
- DEFERRED: 4 (force attack at end of turn, multi-Digimon WhenRemoveField protection)
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT3-093 | Davis Motomiya | FAITHFUL | Memory set to 3, On Play reveal |
| BT24-002 | Bukamon | FAITHFUL | ESS end-of-turn unsuspend with cost |
| BT24-014 | Aegiochusmon | FAITHFUL | DP -5000 target with player selection |
| BT24-019 | Kamemon | FAITHFUL | Alt-digi from TS Lv.2, digi cost reduction |
| BT24-020 | Gomamon | FAITHFUL | Reveal 3 multi-select, inherited unsuspend draw |
| BT24-022 | Ikkakumon | FIXED | Trash from top: Jamming, trash 2 digi cards + stun corrected |
| BT24-023 | Calmaramon | FAITHFUL | Bottom-deck Lv.4-, conditional stun |
| BT24-025 | Shellmon | FAITHFUL | Unsuspend trigger, end-of-turn unsuspend other TS |
| BT24-027 | Lanamon | FAITHFUL | Decode, tuck + battle protection |
| BT24-028 | Divermon | FIXED | Pipe alt-digi: ESS play from digi sources with proper selection |
| BT24-029 | Whamon | FAITHFUL | End-of-Attack and ESS with selection |
| BT24-030 | Neptunemon | FAITHFUL | Cost reduction, bottom-deck, self-unsuspend, protect TS |
| BT24-031 | Elecmon | FAITHFUL | Reveal 3 multi-select |
| BT24-034 | Aegiomon | FAITHFUL | Barrier, security-to-hand play TS Tamer |
| BT24-040 | Venusmon | FAITHFUL | Trash all digi cards + stun 2, protect TS |
| BT24-041 | Minervamon | FAITHFUL | Cost reduction, play Iliad, deletion prevention |
| BT24-051 | Merukimon | FIXED | Duplicate cost: suspend targets player-selectable; Piercing+Rush aura |
| BT24-059 | Sharkmon | FIXED | Pipe alt-digi: De-Digivolve, On Deletion reveal+play, ESS absorb |
| BT24-083 | Hiroko Sagisaka | FAITHFUL | Start-of-turn return-to-deck to play |
| BT24-085 | Dan Yuki & Kanan Yuki | FIXED | Memory threshold: changed from "Digimon count" to "opponent's memory" |
| BT24-088 | Asuna Shiroki | FAITHFUL | Start-of-turn return-to-deck, trash-to-draw |
| BT24-090 | Abyss Sanctuary | FAITHFUL | Blocker + Alliance aura effects |
| BT24-091 | Tidal Stream | FAITHFUL | Bounce lowest, unsuspend TS, link |
| BT24-100 | In-Between Theater | FAITHFUL | Delay timing corrected to field_main |
| BT24-102 | Homeros | FAITHFUL | Start-of-Main memory gain, TS DP aura |
| BT19-101 | ZeedMillenniummon | FIXED | 3 issues via Millenniummon review: critical effect corrections |
| LM-028 | Blue Scramble | FAITHFUL | Delay return blue Digimon from trash |
| P-104 | Mental Training | FAITHFUL | Reveal top 2, select blue, delay digi -2 |
| P-196 | Gomamon (promo) | FAITHFUL | Start-of-Main digivolve into Sea Beast/TS |
| P-197 | Patamon | DEFERRED | ESS -2000 DP auto-selection (low priority) |

## Fixes Applied (2026-03-17 Campaign)
### BT24-028 Divermon / BT24-059 Sharkmon
- Separated piped alt-digi into separate effects for proper handling

### BT24-022 Ikkakumon
- Corrected trash-from-top mechanic for digi card trashing

### BT24-085 Dan Yuki & Kanan Yuki
- Changed end-of-turn Option use condition from "opponent's Digimon count" to "opponent's memory" threshold

### BT24-051 Merukimon
- Removed duplicate cost; suspend targets now player-selectable via effect_select_opponent_permanent

### BT19-101 ZeedMillenniummon
- Fixed 3 critical issues found during Millenniummon archetype review
