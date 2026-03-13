# Archetype QA: Dark Masters
Date: 2026-03-13
Total cards: 58

## Summary
- PASS: 6 frozen QA (BT17-077, BT17-097, BT3-006, BT9-103, EX7-049, ST6-15)
- IMPLEMENTED: 12 new scripts
- QA-FAIL → FIXED: 5 (BT15-080, BT15-081, EX2-046, RB1-035, BT13-088)
- QA-FAIL → REWRITTEN: 1 (BT13-108 — grant-triggered-effect workaround)
- BLOCKED: 1 (BT3-103 — one-shot digivolve hook, shared with ExMaquinamon)
- Unreviewed frozen: 34 (not read by QA agent due to scope confusion)

## Implemented Cards (12 new scripts)

### Simple Batch (5 cards)
| Card | Name | Type | Key Effects |
|------|------|------|-------------|
| BT17-001 | Gigimon | Digi-Egg | Inherited: [When Attacking] pay 1, delete opp Digimon <=3000 DP |
| ST6-14 | Matt Ishida | Tamer | [Your Turn] on own deletion, suspend → +1 memory. Security: play free |
| BT4-097 | Kari Kamiya | Tamer | [All Turns] on security removal, suspend → +1 memory. Security: play free |
| EX2-067 | Fire Ball | Option | Delete opp <=3000 DP, else Draw 2. Security: same |
| ST20-15 | Island of Adventure | Option | Security aura +2000 DP Lv3+. Main: swap security. Security: play Tamer |

### Complex Batch (7 cards)
| Card | Name | Type | Key Effects |
|------|------|------|-------------|
| BT17-068 | Mephistomon | Lv.5 Digimon | Cost reduction via Apocalymon return. On Deletion: play Gulfmon/Dark Masters. Inherited: place Dark Masters from trash +2000 DP |
| BT17-070 | Gulfmon | Lv.6 Digimon | On Play/Digi: place Lv5 Dark Masters + delete Lv5-. When Attacking: return 7 opp trash to deck → unsuspend |
| BT9-112 | DeathXmon | Lv.7 Digimon | Cost -3 per opp permanent. On Play/Digi: de-digivolve all + delete Lv4-. End opp turn: delete lowest cost |
| LM-043 | Darkdramon | Lv.6 Digimon | Counter blast digivolve. Scapegoat. On Play/Digi: de-digivolve 1 + delete lowest. ACE overflow -4 |
| EX4-051 | BlitzGreymon | Lv.6 Digimon | When Digi: 3-branch choice (de-digi 3 / evolve Garurumon / DNA digivolve) |
| EX9-068 | Analogman | Tamer | Start turn: set memory 3. On play trigger: draw + memory + place digi-card |
| EX2-007 | Mother D-Reaper | Token | Can't attack, immune. Place ADR-02 under. D-Reaper cost reduction |

## Fixed Cards
| Card | Fix Summary |
|------|-------------|
| BT15-080 | Added level <= 5 filter to all 3 delete effects |
| BT15-081 | SA+2 → SA+1 |
| EX2-046 | Added BeforePayCost leak guard; added _is_cannot_attack flag |
| RB1-035 | Security condition: False → True |
| BT13-088 | Fixed trash count (1→2), added Belphemon placement, CANNOT_BE_AFFECTED, opponent-turn check |
| BT13-108 | Rewrote: main grants temporary abilities (workaround for engine gap), security targets lowest-cost |

## Engine Gaps
| Card | Gap |
|------|-----|
| BT3-103 | One-shot digivolve hook (shared) |
| BT13-108 | Grant triggered effect to permanent (workaround: OnTappedAnyone listener) |

## Smoke Test
- 50/50 mirror games completed
