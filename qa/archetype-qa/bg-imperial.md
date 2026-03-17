# Archetype QA: BG Imperial
Date: 2026-03-17 (faithfulness campaign)
Total cards: 25

## Summary
- FAITHFUL: 21
- FIXED: 0 (this campaign -- all fixes applied in prior passes)
- DEFERRED: 4 (end-of-turn DNA engine gap, one-shot digivolve hook, per-permanent digi compatibility)
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT3-002 | DemiVeemon | FAITHFUL | Inherited draw if Jamming |
| BT3-093 | Davis Motomiya | FAITHFUL | Memory 3, reveal select |
| BT3-103 | Hidden Potential Discovered! | DEFERRED | One-shot digivolve hook engine gap (main effect) |
| BT12-002 | DemiVeemon | FAITHFUL | Inherited draw if green Digimon |
| BT12-021 | Veemon | FAITHFUL | Reveal 3, add Imperialdramon/Free + Davis |
| BT12-022 | ExVeemon | FAITHFUL | DNA into green +1 memory, inherited Jamming |
| BT12-028 | Paildramon | FAITHFUL | register_modifier corrected, inherited Imperialdramon check |
| BT12-031 | Imperialdramon: Fighter Mode | FAITHFUL | register_modifier + value_fn corrected |
| BT12-047 | Wormmon | FAITHFUL | Reveal 3, add Imperialdramon/Free + Ken |
| BT12-050 | Stingmon | FAITHFUL | DNA into blue +1 memory, inherited Piercing |
| BT16-025 | Paildramon | FAITHFUL | CANNOT_UNSUSPEND modifier corrected |
| BT16-027 | Imperialdramon: Fighter Mode | FAITHFUL | Blast digi, bottom deck, end of attack unsuspend |
| BT16-028 | Imperialdramon: Dragon Mode | FAITHFUL | Alt-digi, CANNOT_UNSUSPEND, suspend/unsuspend trade |
| BT16-040 | Wormmon | DEFERRED | Per-permanent digi compatibility (engine gap) |
| BT16-085 | Davis Motomiya & Ken Ichijoji | FAITHFUL | Security play, play Veemon/Wormmon, DNA trash |
| BT17-077 | Imperialdramon: Paladin Mode | FAITHFUL | When Attacking unsuspend checks bounce success |
| BT17-097 | Return to the Primogenitor | FAITHFUL | Digi from hand, delay digi+protection, security play |
| BT20-020 | Imperialdramon: Fighter Mode | FAITHFUL | Raid, Piercing, play restriction, delete |
| BT21-037 | Lighdramon | FAITHFUL | DP register_modifier, suspend filter, effect order |
| EX1-014 | ExVeemon | FAITHFUL | Jamming, inherited conditional Jamming |
| LM-030 | Green Scramble | FAITHFUL | Delay selection corrected, opponent gate |
| P-117 | Veemon | FAITHFUL | Digi cost -1 for Free, inherited draw |
| ST9-05 | Paildramon | FAITHFUL | DNA bounce, unsuspend self |
| ST9-06 | Imperialdramon Dragon Mode | FAITHFUL | Proper selection for blue and green |
| ST9-09 | Stingmon | FAITHFUL | Play cost -1 with leak guard |

## Deferred Items
| Card ID | Issue | Priority |
|---------|-------|----------|
| BT3-103 | One-shot digivolve cost reduction hook (main effect only) | Low |
| BT12-021 | Inherited end-of-turn DNA digivolve engine gap | Low |
| BT12-047 | Inherited end-of-turn DNA digivolve engine gap | Low |
| BT16-040 | Per-permanent digi compatibility check (engine lacks CanPlayCardTargetFrame) | Low |

## Fixes Applied (2026-03-17 Campaign)
No new fixes required this campaign. All prior fixes (register_modifier arg order, value_fn signatures, timing corrections, selection improvements) were applied in earlier passes and verified faithful.
