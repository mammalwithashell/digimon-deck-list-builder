# Archetype QA: TS Jupitermon
Date: 2026-03-17 (faithfulness campaign)
Total cards: 30

## Summary
- FAITHFUL: 20
- FIXED: 10 (this campaign)
- DEFERRED: 0
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT4-104 | Heavenly Chant | FAITHFUL | security_cards.pop(0) correct |
| BT10-042 | Venusmon | FAITHFUL | Declarative static effect replacement |
| BT14-033 | Renamon | FAITHFUL | Security search with shuffle |
| BT15-003 | Nyaromon | FAITHFUL | Trash top/bottom security branch choice |
| BT24-003 | Puttimon | FAITHFUL | Shaman trait digi_filter with cost_reduction=1 |
| BT24-014 | Aegiochusmon | FAITHFUL | Delete gated on security count <= 3 |
| BT24-030 | Neptunemon | FIXED | Cost reduction, bottom-deck, self-unsuspend, protect TS (fixed: WhenRemoveField→WhenPermanentWouldBeDeleted) |
| BT24-031 | Elecmon | FAITHFUL | On Play reveal 3 multi-select |
| BT24-034 | Aegiomon | FAITHFUL | Barrier, security-to-hand to play TS Tamer |
| BT24-037 | Jupitermon | FIXED | Auto-select removed; AND->OR logic (Yellow/Red/TS); source zone hand->digi-stack |
| BT24-040 | Venusmon | FIXED | 3 bugs: CANNOT_SUSPEND leaked globally, DISABLE_EFFECT hit all effects not just WD, freeze dupe selection |
| BT24-041 | Minervamon | FIXED | Alt-digi encodes all 3 traits, _fire_play_observers + play-block checks added |
| BT24-043 | Tapirmon | FIXED | Reveal selection was optional, now mandatory per card text |
| BT24-046 | Garurumon | FAITHFUL | Alt-digi with Gabumon name |
| BT24-051 | Merukimon | PASS | 4 fixes: alt-digi TS trait, suspend chaining, DP turn-scoped, WA→OnUseAttack |
| BT24-083 | Hiroko Sagisaka | FAITHFUL | Start-of-turn return-to-deck to play |
| BT24-084 | Megumi Hinata | FAITHFUL | Condition uses card.permanent_of_this_card() |
| BT24-085 | Dan Yuki & Kanan Yuki | FIXED | Memory threshold: opponent's memory, not Digimon count |
| BT24-088 | Asuna Shiroki | FAITHFUL | Start-of-turn return-to-deck to play from trash |
| BT24-090 | Abyss Sanctuary | FIXED | Dynamic color bypass, face-down security gate for Blocker/Alliance aura, proper security API |
| BT24-094 | Central Town | FIXED | Color bypass, face-down aura condition, security placement |
| BT24-095 | Sonic Shot | FIXED | Color bypass fn, TS link filter, is_on_attack, chained selections |
| BT24-100 | In-Between Theater | FIXED | Color ignore, delay factory, reveal optionality |
| BT24-101 | Homeros | FIXED | Dynamic cost: alt-digi cost 5 (not 3); target selection for -13000 DP |
| BT24-102 | Homeros | FIXED | Olympos XII trait check top-card-only; added eligibility+condition filter for EOT reactivation |
| P-194 | Jupitermon (promo) | FAITHFUL | |
| P-196 | Gomamon | FAITHFUL | Start-of-Main digivolve into Sea Beast/TS |
| P-197 | Patamon | FIXED | alt_digi_level corrected; Angel trait + memory <= 4 gate + cost_override=0 |
| P-198 | DemiDevimon | FIXED | alt_digi_level corrected; Fallen Angel trait + memory <= 4 gate + process2 callback |
| P-213 | Jupitermon (promo) | FAITHFUL | Security <= 3 check |

## Fixes Applied (2026-03-17 Campaign)
### BT24-041 Minervamon
- Removed fabricated effects not present in card text
- Added proper process callback for security trash deletion prevention

### BT24-051 Merukimon
- Removed fabricated effects not present in card text
- Removed duplicate cost; corrected suspend targets to use player selection

### BT24-085 Dan Yuki & Kanan Yuki
- Changed condition from "opponent's Digimon count" to "opponent's memory" (memory threshold)

### BT24-101 Homeros
- Fixed alt-digi cost from 3 to 5 per C# reference
- Second alt-digi now targets Aegiochusmon by name
- Replaced auto-select for -13000 DP with effect_select_opponent_permanent for RL agent choice

### BT24-037 Jupitermon
- Removed auto-selection; changed AND->OR logic for Yellow/Red/TS conditions
- Changed source zone from hand to digi-stack

### BT24-090 Abyss Sanctuary
- Card text says Blocker, not +2000 DP; replaced dp_modifier with _is_blocker aura
- Alliance stub replaced with _is_alliance aura effect

### P-197 Patamon
- Corrected alt_digi_level; added Angel trait filter; added memory <= 4 gate; added cost_override=0

### P-198 DemiDevimon
- Corrected alt_digi_level; added Fallen Angel trait filter; added memory <= 4 gate; added process2 callback

### BT24-041/BT24-051/BT24-085
- Fabricated effects removed (effects not present in card text were generating incorrect behavior)
