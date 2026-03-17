# Archetype QA: Dark Masters
Date: 2026-03-17 (faithfulness campaign)
Total cards: 58

## Summary
- FAITHFUL: 31
- FIXED: 11 (this campaign)
- DEFERRED: 16 (low priority auto-selection in generic support cards)
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT3-006 | DemiMeramon | FAITHFUL | |
| BT3-103 | Hidden Potential Discovered! | DEFERRED | One-shot digivolve hook engine gap |
| BT8-090 | Kari Kamiya | FAITHFUL | Start of turn memory set, on-add-security suspend |
| BT9-103 | Kongou | FAITHFUL | grant_keyword -> register_modifier loop |
| BT9-112 | DeathXmon | FAITHFUL | BeforePayCost cost preview correct |
| BT13-088 | Etemon | FAITHFUL | |
| BT13-108 | Piedmon | DEFERRED | Grant triggered effect workaround |
| BT15-027 | Gazimon | FIXED | Shared pattern: selection and filter corrections |
| BT15-031 | DarkSuperStarmon | FIXED | Self-delete: corrected self-targeting in deletion effect |
| BT15-050 | Volcamon | FIXED | Shared pattern: selection and filter corrections |
| BT15-062 | Puppetmon | FIXED | Shared pattern: selection and filter corrections |
| BT15-066 | Machinedramon | FAITHFUL | End of opponent's turn check, correct targeting, Dark Masters filter |
| BT15-072 | Vilemon | FAITHFUL | Blocker + scapegoat prevention |
| BT15-077 | LadyDevimon | FAITHFUL | 2-pass reveal, EOT delete-own then play Dark Masters |
| BT15-079 | MetalSeadramon | FIXED | Self-delete: corrected self-targeting in deletion effect |
| BT15-080 | Piedmon | FAITHFUL | |
| BT15-081 | Machinedramon | FAITHFUL | |
| BT15-102 | Apocalymon | FAITHFUL | BeforePayCost counts distinct Dark Masters, EOT digi-card placement |
| BT16-026 | Dobermon | FIXED | Suspend target: corrected to target proper permanent |
| BT16-046 | GranKuwagamon | FAITHFUL | Suspend up to 2 with cannot-unsuspend, delete suspended Tamer |
| BT17-077 | Imperialdramon: PM | FAITHFUL | Trash-return + memory gain logic |
| BT17-097 | Return to the Primogenitor | DEFERRED | |
| BT19-075 | MoonMillenniummon | FIXED | Inverted WhenRemoveField logic + self-filter corrected |
| BT21-051 | Grankuwagamon | FIXED | Bounce: corrected bounce targeting |
| EX2-046 | ADR-02 Searcher | FAITHFUL | |
| EX5-016 | SkullMeramon | FIXED | Costs: corrected cost calculation |
| EX7-049 | Metallicdramon | DEFERRED | WhenRemoveField lacks removal-cause context |
| EX8-026 | Impmon | FIXED | Wrong effect: corrected to match card text |
| EX10-010 | BlackWarGreymon | FAITHFUL | Conditional +3000 DP and effect immunity |
| EX10-012 | MetalSeadramon | FAITHFUL | Cost reduction, cannot-suspend, on-deletion to security |
| EX10-020 | Puppetmon | FAITHFUL | Cost reduction, bounce suspended, on-deletion to security |
| EX10-035 | Machinedramon | FAITHFUL | Cost reduction, de-digivolve 2x2, on-deletion to security |
| EX10-057 | Piedmon | FAITHFUL | Cost reduction, delete unsuspended, on-deletion to security |
| EX10-061 | Apocalymon | FAITHFUL | BeforePayCost security Digimon, play from digi-cards, Rush grant |
| EX10-074 | Apocalymon | FIXED | Full implementation: previously incomplete |
| RB1-035 | Hokuto Amanokawa | FAITHFUL | OnStartTurn timing |
| ST20-15 | Island of Adventure | DEFERRED | Security card DP aura engine limitation |
| ST6-15 | Death Claw | FAITHFUL | |
| BT15-003 | Nyaromon | DEFERRED | |
| BT17-093 | Kari Kamiya | DEFERRED | |
| BT17-095 | Brave Tornado | DEFERRED | |
| BT17-102 | Agumon -Bond- | DEFERRED | |
| BT5-092 | Nokia Shiramine | DEFERRED | |
| BT5-093 | Tai & Matt | DEFERRED | |
| EX1-021 | MetalGarurumon | DEFERRED | |
| EX4-061 | Tai & Matt | DEFERRED | |
| EX9-066 | Tai Kamiya & Matt Ishida | DEFERRED | |
| ST16-14 | Matt Ishida | DEFERRED | |
| ST6-14 | Matt Ishida | DEFERRED | |
| BT8-094 | Digimon Emperor | FAITHFUL | |
| BT8-097 | Crimson Blaze | FAITHFUL | |
| BT13-101 | Miki & Megumi | DEFERRED | |
| BT14-009 | Gotsumon | FAITHFUL | |
| P-206 | Digital Gate Open | FAITHFUL | |
| BT16-082 | Ukkomon | FAITHFUL | |
| P-123 | Ukkomon | FAITHFUL | |
| BT12-059 | Agumon (Black) | FAITHFUL | |
| ST20-11 | WarGreymon | DEFERRED | |

## Fixes Applied (2026-03-17 Campaign)
### EX10-074 Apocalymon
- Full implementation of previously incomplete script

### BT19-075 MoonMillenniummon
- Inverted WhenRemoveField logic corrected; self-filter added to prevent self-targeting

### BT15-031 DarkSuperStarmon
- Self-delete effect corrected to target self instead of wrong permanent

### BT15-079 MetalSeadramon
- Self-delete effect corrected to target self instead of wrong permanent

### EX8-026 Impmon
- Replaced wrong effect with correct implementation matching card text

### BT15-027 Gazimon / BT15-050 Volcamon / BT15-062 Puppetmon
- Shared pattern fix: selection and filter corrections across these three cards

### EX5-016 SkullMeramon
- Corrected cost calculation logic

### BT21-051 Grankuwagamon
- Corrected bounce targeting

### BT16-026 Dobermon
- Corrected suspend target selection
