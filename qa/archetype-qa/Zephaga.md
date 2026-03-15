# Archetype QA: Zephaga
Date: 2026-03-11
Total cards: 40

## Summary
- PASS: 3
- IMPLEMENTED: 19
- QA-FAIL (found and fixed): 18
- BLOCKED: 0 (BT3-103 main effect pre-existing BLOCKED, not new)

## Implemented Cards (19 new scripts)
| Card ID | Name | Category |
|---------|------|----------|
| BT7-004 | Koromon | Digi-Egg |
| EX4-002 | Kokomon | Digi-Egg |
| EX7-004 | Fluffymon | Digi-Egg |
| ST18-01 | Fluffymon | Digi-Egg |
| EX7-031 | Pteromon | Lv.3 |
| ST18-04 | Pteromon | Lv.3 |
| ST18-05 | Muchomon | Lv.3 |
| BT9-047 | Pomumon | Lv.3 |
| EX7-032 | Galemon | Lv.4 |
| ST18-08 | Galemon | Lv.4 |
| ST17-07 | Rapidmon | Lv.5 |
| EX7-034 | GrandGalemon | Lv.5 |
| ST18-10 | GrandGalemon | Lv.5 |
| ST22-13 | GrandGalemon | Lv.5 |
| EX7-036 | Zephagamon | Lv.6 |
| EX7-064 | Shoto Kazama | Tamer |
| ST18-14 | Shoto Kazama | Tamer |
| ST18-12 | Zephagamon | Lv.6 |
| BT12-057 | Quartzmon | Lv.7 |

## QA Failures Found and Fixed (18 cards)

### EX11-026 Pteromon
- Suspend target was opponent-only instead of any Digimon
- DP buff applied to self instead of selected Bird/Avian/Vortex Warriors Digimon
- DP used change_dp instead of register_modifier with proper expiry
- Spurious CANNOT_BE_SELECTED_BY_EFFECT modifier removed

### EX11-028 Galemon
- On Play/When Digivolving suspend was opponent-only and mandatory
- OnTappedAnyone missing checks: own Digimon, tamer count, Shoto Kazama in hand
- Play filter accepted any card instead of Shoto Kazama only

### EX11-032 GrandGalemon
- Completely wrong effects rewritten; complex hand-activation mechanic partially implemented

### P-132 Galemon
- Suspend target opponent-only → any
- DP via change_dp → register_modifier
- Missing Piercing conditional on Shoto Kazama presence

### P-166 Galemon
- Suspend target opponent-only → any
- Wrong effect order (digivolve before suspend)
- Missing turn guard, optional flag, cost reduction from suspended count

### BT14-044 Palmon
- Main effect incomplete (temp-effect granting)
- Inherited missing green Tamer check and WhenWouldDigivolve timing

### BT24-044 Muchomon
- Wrong suspend filter (name check instead of level check)
- Spurious trash_cards.pop(), unconditional reveal, wrong Avian filter

### BT24-047 Kokatorimon
- Wrong suspend target, unconditional unsuspend
- Missing trait filter and "may attack" grant

### P-038 Green Memory Boost
- Spurious trash_cards.pop() before reveal

### P-106 Agility Training
- Spurious trash_cards.pop(), missing green filters, missing digivolve target selection

### BT20-101 Zephagamon
- Missing Piercing factory, missing Ace Overflow inherited
- Suspend target opponent-only → any, wrong bounce method (hand → deck bottom)
- Missing suspended count loop, unsuspend was selecting any perm instead of self

### BT20-037 Chaosmon: Valdur Arm
- Missing level-6 source loop and per-iteration memory gain
- CANNOT_UNSUSPEND applied to 1 selected instead of all opponent permanents

### BT20-085 Shoto Kazama
- Missing self-return-to-deck-bottom cost
- Missing Shoto Kazama name filter, missing no-Digimon conditional play
- End of turn: wrong target for DP buff, missing tamer suspend cost

### LM-030 Green Scramble
- Minor: spurious opponent-has-Digimon gate on delay

### EX11-035 Zephagamon
- Missing Piercing factory
- Unsuspend/suspend targeting not offering both fields
- Dynamic DP cap not implemented
- OnTappedAnyone triggering on any Digimon instead of own only

### EX11-062 Shoto Kazama
- Suspend trigger completely wrong (should be tamer-as-cost, effect-only trigger, draw + DP buff)
- Missing continuous Vortex-can-attack-players effect
- register_modifier argument order fix

### EX11-072 Guardian Vortex
- Wrong delay condition, wrong suspend trigger check
- Digivolve filter targeting wrong cards

### EX11-074 Vortexdramon
- Missing Piercing factory
- Wrong modifier type (CANNOT_BE_SELECTED → CANNOT_BE_AFFECTED)
- Wrong expiry, immunity not gated on own Digimon
- register_modifier argument order fix

### EX8-074 MedievalGallantmon
- "Other suspended" count only checked own field → fixed to both fields
- register_modifier argument order fix

## PASS (no changes needed)
- P-131 Pteromon
- BT3-103 Hidden Potential Discovered! (main effect BLOCKED pre-existing)
- P-038 Green Memory Boost (after fix)

## Smoke Test
- 50/50 games completed successfully (random actions, mirror match)

## Engine Gaps
- BT3-103: No mechanism for player-level one-shot digivolve cost hook
- BT14-044: No mechanism for granting triggered effects to opponent's permanents
- BT9-047: No play-lock mechanism for effect-based plays (descriptive-tagged)
- BT12-057: CANNOT_UNSUSPEND only applies to permanents present at time of digivolve (no aura for new entries)
- EX11-032: Hand-activated [Main] effects on Digimon cards partially approximated
