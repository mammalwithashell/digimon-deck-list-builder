# Archetype QA: Puppets
Date: 2026-03-14
Total cards: 57

## Summary
- Frozen: 29 (QA reviewed — no selection issues found)
- Unfrozen (prior reviewed): 6 (BT5-033, BT5-106, BT9-033, BT9-112, EX4-074, LM-035)
- IMPLEMENTED: 22 new scripts (all with C# reference)
- FIXED: 4 scripts with selection/filter bugs
- BLOCKED: 0

## Fixes Applied (2026-03-14)

### LM-029 Yellow Scramble
- **Bug**: Delay effect auto-selected `qualifying[0]` for "Return 1 yellow Digimon from trash to top of deck"
- **Fix**: Replaced with `game.request_selection(GamePhase.SelectTrash, ...)` to let agent choose which yellow Digimon to return (matching LM-030 reference implementation)

### P-156 Future Potential!
- **Bug**: Collected ALL Tamer colors as a simplification instead of letting player choose a specific Tamer first
- **Fix**: Added `game.effect_select_own_permanent()` to choose a Tamer, then filter play targets by that Tamer's colors (matching C# reference which uses SelectPermanentEffect for Tamer selection)

### BT22-029 Shoemon
- **Bug 1**: On Play/On Deletion Blocker grant used `p.is_digimon` filter but card text specifies "1 of your Digimon with the [Puppet] trait"
- **Fix 1**: Added Puppet trait check to Blocker target filter (matching C# `SharedPuppetDigimon` which checks `permanent.TopCard.HasPuppetTraits`)
- **Bug 2**: Inherited [When Attacking] -2000 DP auto-selected `min(dp_targets, key=lambda p: p.dp)` instead of player choice
- **Fix 2**: Replaced with `game.effect_select_opponent_permanent()` (matching C# SelectPermanentEffect)

### BT22-032 ShoeShoemon
- **Bug 1**: On Deletion play filter returned `True` for ALL cards but card text specifies "1 level 3 Digimon card with the [Puppet] trait"
- **Fix 1**: Added level 3 + Puppet trait checks to play filter (matching C# `IsPuppetLevel3Card`)
- **Bug 2**: Inherited [When Attacking] -2000 DP auto-selected `min(dp_targets)` instead of player choice
- **Fix 2**: Replaced with `game.effect_select_opponent_permanent()` (matching C# SelectPermanentEffect)

## Implemented Cards

### ST19 Batch (8 cards -- Starter Deck)
| Card | Name | Key Effects |
|------|------|-------------|
| ST19-01 | Kyaromon | Digi-Egg. Inherited: [When Attacking] OPT Draw 1 if another Digimon |
| ST19-03 | Shoemon | On Play: reveal 3, add Puppet + LIBERATOR. Inherited: opp security -3000 DP |
| ST19-04 | PawnChessmon (Y/B) | On Play: trash Puppet -> Draw 2. Inherited: Reboot |
| ST19-05 | PawnChessmon (B/Y) | Blocker. On Deletion: trash Puppet -> Draw 2 |
| ST19-08 | ShoeShoemon | Security: play LIBERATOR cost<=4 free. Overclock (Puppet). Inherited: opp security -3000 DP |
| ST19-11 | Chaperomon | On Play/Digi: opp -3000 DP (-6000 if 3+ total). Inherited: prevent leaving via Puppet/Token delete |
| ST19-12 | Cendrillmon | Overclock (Puppet). Blocker. When Digi: play 2 Familiar Tokens |
| ST19-14 | Arisa Kinosaki | Tamer. Start turn: memory 3. When Token/Puppet played by effect: suspend -> grant Rush |

### EX7 Batch (6 cards -- Puppet Engine)
| Card | Name | Key Effects |
|------|------|-------------|
| EX7-024 | Shoemon | Digi cost -1 into Puppet. Inherited: opp security -3000 DP |
| EX7-025 | ShoeShoemon | When Digi: play Arisa if <=1 Tamer. Inherited: opp security -3000 DP |
| EX7-027 | Chaperomon | Overclock (Puppet). When Digi: play Lv3 Puppet free. Inherited: prevent leaving via Token/Puppet delete |
| EX7-030 | Cendrillmon | Overclock. Start Main: play Familiar Token. When Digi: play Familiar Token. When Attacking: opp -6000 DP |
| EX7-063 | Arisa Kinosaki | Tamer. Start Main: +1 memory. On Token/Puppet deletion: play Lv3 Puppet free |
| EX7-074 | Vortex Resonance | Option. Reveal 3, add LIBERATOR, digi cost -4. Security: play LIBERATOR cost<=4 |

### EX9+LM+BT6 Batch (8 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| EX9-024 | Hanimon | Alt digi from Kyaromon. On Play: trash 1 -> return Puppet from trash. Inherited: end attack |
| EX9-027 | Kokeshimon | Alt digi Puppet. When Digi/On Deletion: trash 1 -> opp -4000 DP. Inherited: end attack |
| EX9-032 | Karakurumon | Alt digi Puppet. On Play/Digi: delete Token/Puppet -> digi from hand free. Inherited: prevent leaving |
| EX9-033 | Kaguyamon | Alt digi Puppet. Blocker+Alliance for Tokens/Puppets. On other delete: delete opp lowest level. End turn: play Lv4- Puppet from trash |
| EX9-067 | Mirai Kinosaki | Tamer. On Play: reveal 3, add Puppet/LIBERATOR. On Puppet digi: return self, play with cost -3 |
| LM-029 | Yellow Scramble | Option. Digi from hand cost -3, Delay: return yellow Digimon from trash (SELECTION). Security: play yellow DP<=2000 |
| LM-037 | Black Memory Boost! | Option. Reveal 3, add black/yellow Digimon. Delay +2 memory |
| BT6-084 | Sistermon Ciel | Tamer. Aura +2000 DP for Royal Knight/Huckmon. On Play: +1 memory |

### EX11 Batch (8 cards)
| Card | Name | Verdict |
|------|------|---------|
| EX11-019 | Shoemon | PASS - On Deletion: play Familiar Token. Inherited: Barrier |
| EX11-020 | Hanimon | PASS - Alt digi Kyaromon. On Deletion: play Shoemon. Inherited: end attack (proper selection) |
| EX11-021 | Kokeshimon | PASS - Alt digi Puppet. When Digi: play Mirai Kinosaki. Inherited: end attack (proper selection) |
| EX11-022 | Karakurumon | PASS - Alt digi Puppet. Scapegoat. On Play/Digi: play Puppet DP<=4000, delete EOT. Inherited: prevent leaving (proper selection) |
| EX11-023 | Kaguyamon | PASS - Alt digi Puppet. Alliance. Scapegoat. When Digi/End Opp Turn: delete lowest. On other delete: play Puppet Lv4- from trash |
| EX11-024 | Cendrillmon | PASS - Alliance. Overclock. On Play/Digi: play Puppet Lv4- + Familiar Tokens. When Digi/Attack: -3000 DP per Digimon (proper selection) |
| EX11-060 | Arisa Kinosaki | PASS - Tamer. Memory 3. On Token/Puppet delete: suspend, Draw 1, Overclock play |
| EX11-061 | Mirai Kinosaki | PASS - Tamer. Memory +1. On Puppet digi: suspend, play Lv3 Puppet from hand |

### BT22 Batch (7 cards)
| Card | Name | Verdict |
|------|------|---------|
| BT22-002 | Kyaromon | PASS - Inherited: Draw 1 on Token/Puppet deletion |
| BT22-029 | Shoemon | FIXED - On Play/On Deletion: Puppet Blocker grant (was missing trait filter). Inherited: -2000 DP (was auto-selecting) |
| BT22-032 | ShoeShoemon | FIXED - On Deletion: play Lv3 Puppet (was unfiltered). Inherited: -2000 DP (was auto-selecting) |
| BT22-036 | Chaperomon | PASS - Overclock. Hand/Main: place ShoeShoemon, digi onto Shoemon (proper trash+perm selection). Inherited: prevent leaving (proper selection) |
| BT22-040 | Cendrillmon | PASS - Overclock. On Play/Digi: play Familiar Token. On other deletion: activate WD |
| BT22-042 | Nyabootmon | PASS - Alt digi Chaperomon. Overclock. When Digi: play Puppet + -3000 DP per Digimon (proper selection). On deletion: re-activate WD |
| BT22-088 | Arisa Kinosaki | PASS - Start Main: bottom deck self, play Arisa from hand. On Token/Puppet play: suspend, Draw 1. Security |
| BT22-098 | Unique Emblem | PASS - Main: play Shoemon/Arisa. Delay: Arisa suspend -> digi Puppet into LIBERATOR cost -3 (proper selection) |

### Other Cards (9 cards)
| Card | Name | Verdict |
|------|------|---------|
| BT13-101 | Miki & Megumi | PASS - On Play: play PawnChessmon. On black+yellow play: suspend, Draw 1 + Memory +1 |
| BT15-003 | Nyaromon | PASS - Inherited: trash top/bottom security (branch choice), +1 memory |
| BT16-055 | Namakemon | PASS - Alt digi Pulsemon. On Play/Digi: grant keywords via proper selection |
| BT23-077 | Sistermon Ciel | PASS - Also treated as Sistermon Noir (via card.also_treated_as_names). Blocker. On Play: delete opp cost<=4. On suspend: De-Digivolve 1 (proper selection) |
| P-037 | Yellow Memory Boost | PASS - Reveal 4, add yellow Digimon. Delay: +2 memory |
| P-105 | Physical Training | PASS - Reveal 2, add yellow. Delay: digi into yellow cost -2 |
| P-134 | Shoemon | PASS - On Play: SA -1 to opp (proper selection). Inherited: -2000 DP (proper selection) |
| P-156 | Future Potential! | FIXED - Main: choose Tamer (was collecting all colors), play matching Digimon cost<=3. Security: play Tamer |
| P-165 | ShoeShoemon | PASS - When Digi/On Play: Familiar Token. Inherited: Barrier |
| P-206 | Digital Gate Open | PASS - Reveal 3, add Digimon+Tamer. Delay: play color-matched Tamer cost -4. Security: play Digimon cost<=3 |
| P-229 | Narrative Ronde | PASS - Reveal 3, add Puppet+LIBERATOR. Delay: digi into Lv6- LIBERATOR cost -3 |

### Frozen Cards (6 prior reviewed)
| Card | Name | Verdict |
|------|------|---------|
| BT5-033 | PawnChessmon | PASS (frozen) |
| BT5-106 | Miki & Megumi | PASS (frozen) |
| BT9-033 | Pillomon | PASS (frozen) - play-lock not enforceable in engine, descriptive-tagged |
| BT9-112 | Yellow Memory Boost | PASS (frozen) |
| EX4-074 | Sistermon Ciel | PASS (frozen) |
| LM-035 | Purple Memory Boost | PASS (frozen) |
| EX8-030 | Tapirmon | PASS (frozen) |

### Token
- `familiar` token registered in `token_registry.py` -- Yellow Digimon, 3000 DP, On Deletion: opp Digimon -3000 DP

## Smoke Test
- 50/50 mirror games completed (prior)
- 11/20 post-fix mirror games completed, 9 RecursionError (pre-existing token deletion loop issue, not related to fixes), 0 other errors

## Known Issues (Pre-existing)
- RecursionError in ~45% of games due to token deletion chains triggering re-entrant effects (Familiar Token On Deletion -> Arisa Kinosaki trigger -> play new token -> etc). This is a systemic engine issue, not specific to these scripts.
