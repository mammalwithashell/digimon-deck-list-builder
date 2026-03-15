# Dark Masters Archetype QA Report

**Date**: 2026-03-13
**Archetype**: Dark Masters
**Cards tested**: 39
**Method**: API-driven gameplay via debug endpoints

## Summary

| Verdict | Count |
|---------|-------|
| PASS | 21 |
| PARTIAL | 9 |
| FAIL | 9 |

## Critical Issues Found

### 1. EX10 Dark Masters [Hand][Main] cost reduction NOT working (4 cards)

**Affected**: EX10-012, EX10-020, EX10-035, EX10-057

All four EX10 Dark Masters (MetalSeadramon, Puppetmon, Machinedramon, Piedmon) have a [Hand][Main] effect: "If you don't have any Digimon other than Digimon with [Dark Masters] in their texts, play with cost reduced by 5."

The scripts set `effect0.cost_reduction = 5` but the effect condition checks `effect.effect_source_permanent.top_card.card_text` for "Dark Masters". Since the card is in hand (no permanent), the condition always returns False. The cost reduction never applies. All four cards pay full cost 11 instead of 6.

**Root cause**: Script condition checks permanent text instead of checking the field condition (no non-DM Digimon on battle area). The effect also has wrong timing (`OnDeclaration` instead of implementing a [Hand][Main] play mechanism).

### 2. EX9-068 Analogman crashes on play (1 card)

**Affected**: EX9-068

Playing Analogman causes a 500 Internal Server Error. The script calls `player.set_memory(3)` in the Start of Turn effect, but the `Player` class has no `set_memory` method. Also uses `player.memory` which references the Player's local field (always 0), not the Game's memory gauge.

**Fix**: Use `game.memory` for reading and direct assignment for setting.

### 3. Option cards not playable from empty board (4 cards)

**Affected**: EX10-072, BT19-093, EX2-067, ST20-15

No play actions appear for option cards. The engine's `action_mask.py` requires option cards to have a matching-color Digimon/Tamer on the field (lines 56-71). Cards with "ignore color requirement" effects (EX10-072 Spiral Mountain, BT19-093 Queen Device) should set `card._match_color_requirement = False`, but their scripts have empty process callbacks.

For EX2-067 Fire Ball (red) and ST20-15 Island of Adventure (white), the color requirement is correct behavior when no matching-color card is on field. These would PASS if tested with proper setup.

### 4. Digitama (Lv2) playable from hand (2 cards)

**Affected**: BT14-001, BT17-001

Koromon and Gigimon (both Lv2 digitama) appear as playable from hand via the inject mechanism. In the actual game, Lv2 cards should only be in the egg deck and hatched, never played from hand. This may be an inject artifact rather than a game rules bug, but should be verified.

### 5. Cost anomalies on specific cards

**BT21-051 Puppetmon**: Play cost 7, but charged only 3 (matching alt-digi cost). Something in the engine may be confusing alt-digi cost with play cost.

**EX8-026 MetalSeadramon**: Same issue - play cost 7 but charged 3 (matching alt-digi cost).

**BT17-070 Gulfmon**: Play cost 12, but charged only 8. Unknown reduction source.

**BT19-064 Justimon: Blitz Arm**: Play cost 6, but charged only 6 from 7 memory (correct) in one test, and 6 from 7 in another (also correct). PASS.

**EX4-051 BlitzGreymon**: Play cost 12, but charged only 8 from 10 memory.

**BT9-112 DeathXmon**: Play cost 20 (reduced by 3 per opponent Digimon/Tamer), but appeared playable at 10 memory with no opponent creatures. Should not be playable.

## Detailed Script Issues (by card)

### EX10-057 Piedmon (EX10)
- [Hand][Main] cost reduction: condition checks permanent text instead of field state; never triggers
- [On Play] delete: target filter doesn't check "unsuspended" (should only delete unsuspended)
- [On Deletion] place as security: condition doesn't check "no purple face-up security"; process does `player.recovery(1)` instead of placing the deleted Digimon itself
- [All Turns] only digivolve into Apocalymon: not enforced

### EX10-012 MetalSeadramon (EX10)
- Same [Hand][Main] cost reduction bug as EX10-057
- [On Play] prevent suspend: process function plays a Lv5 Digimon from hand (wrong) instead of preventing opponent suspend

### EX10-020 Puppetmon (EX10)
- Same [Hand][Main] cost reduction bug
- [On Play] return suspended to bottom: process plays Lv5 from hand (wrong)

### EX10-035 Machinedramon (EX10)
- Same [Hand][Main] cost reduction bug
- [On Play] De-Digivolve 2 on 2 targets: process plays Lv5 from hand (wrong)

### EX10-072 Spiral Mountain
- Color requirement bypass not implemented (empty process)
- [Main] effect only draws 2, does not place card in battle area
- Delay effect plays from "hand" instead of "security stack"

### EX9-068 Analogman
- Crashes on play due to `player.set_memory(3)` (method doesn't exist)
- Should use `game.memory` for gauge operations

### BT19-093 Queen Device
- Color requirement bypass not implemented
- Not playable without matching-color field card

## Per-Card Results

| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT14-001 | Koromon | FAIL | Lv2 digitama playable from hand (should only be in egg deck) |
| BT15-008 | Muchomon | PASS | Played for cost 3 correctly |
| BT15-027 | Scorpiomon | PASS | Played for cost 6, reveal effect resolved |
| BT15-031 | MetalSeadramon | PASS | Played for cost 11 correctly |
| BT15-050 | Cherrymon | PASS | Played for cost 6, reveal effect resolved |
| BT15-062 | Gigadramon | PASS | Played for cost 6, reveal effect resolved |
| BT15-066 | Machinedramon | PASS | Played for cost 11 correctly |
| BT15-072 | Vilemon | PASS | Played for cost 4 correctly, Blocker present |
| BT15-077 | LadyDevimon | PASS | Played for cost 6, reveal effect resolved |
| BT15-079 | Piedmon | PASS | Played for cost 11 correctly |
| BT15-102 | Apocalymon | PASS | Playable with DM cards in trash for cost reduction |
| BT16-026 | Vikemon | PASS | Played for cost 7 correctly |
| BT16-046 | GranKuwagamon | PASS | Played for cost 7 correctly |
| BT17-001 | Gigimon | FAIL | Lv2 digitama playable from hand |
| BT17-068 | Mephistomon | PASS | Played for cost 8 correctly |
| BT17-070 | Gulfmon | PARTIAL | Cost anomaly: charged 8 instead of 12 |
| BT19-064 | Justimon: Blitz Arm | PASS | Played for cost 6 correctly |
| BT19-093 | Queen Device | FAIL | Option not playable (color req bypass missing) |
| BT21-051 | Puppetmon (BT21) | PARTIAL | Cost anomaly: charged 3 instead of 7 (alt-digi leak) |
| BT23-007 | Musclemon | PASS | Played for cost 3 correctly |
| BT4-097 | Kari Kamiya | PASS | Played for cost 3 correctly |
| BT9-112 | DeathXmon | PARTIAL | Playable at 10 memory with no cost reduction targets (cost 20) |
| EX10-012 | MetalSeadramon (EX10) | PARTIAL | Cost reduction not applied (pays 11 instead of 6) |
| EX10-020 | Puppetmon (EX10) | PARTIAL | Cost reduction not applied (pays 11 instead of 6) |
| EX10-035 | Machinedramon (EX10) | PARTIAL | Cost reduction not applied (pays 11 instead of 6) |
| EX10-057 | Piedmon (EX10) | PARTIAL | Cost reduction not applied (pays 11 instead of 6) |
| EX10-061 | Apocalymon (EX10) | PASS | Play action available (correct cost check) |
| EX10-072 | Spiral Mountain | FAIL | Option not playable (color req bypass + placement missing) |
| EX10-074 | Beelzemon | PASS | Played for cost 7 correctly |
| EX2-007 | Mother D-Reaper | PASS | Played for cost 0 correctly |
| EX2-067 | Fire Ball | FAIL | Option not playable (needs red card on field) |
| EX4-051 | BlitzGreymon | PARTIAL | Cost anomaly: charged 8 instead of 12 |
| EX5-016 | Lunamon | PASS | Played for cost 3 correctly |
| EX8-026 | MetalSeadramon (EX8) | PARTIAL | Cost anomaly: charged 3 instead of 7 |
| EX9-068 | Analogman | FAIL | 500 error (player.set_memory doesn't exist) |
| LM-043 | Darkdramon | PASS | Played for cost 7 correctly |
| P-216 | WaruMonzaemon | PASS | Played for cost 6, On Play selection triggered |
| ST20-15 | Island of Adventure | FAIL | Option not playable (needs white card on field) |
| ST6-14 | Matt Ishida | PASS | Played for cost 2 correctly |

## Test Environment

- Server: http://localhost:8000 with DEBUG_MODE=1
- Deck: Dark Masters archetype decklist from deck_library.json
- Non-deck cards tested via /debug/games/{id}/inject-card
- Memory gauge interpretation: positive = P1's turn, crossing 0 = turn passes to P2 (P2 gets absolute value)
