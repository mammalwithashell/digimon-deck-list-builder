# Archetype QA: Rocks
Date: 2026-03-12
Total cards: 28

## Summary
- PASS: 10
- QA-FAIL: 18 (4 critical, 5 high, 4 medium, 5 low)
- BLOCKED: 0
- Engine limitation: 1

## Smoke Test
- 50/50 mirror-match games completed successfully (random policy, max 500 steps)

---

## Card Results

### Batch 1: Eggs + Rookies
| Card | Name | Verdict |
|------|------|---------|
| EX8-005 | Tumblemon | PASS |
| BT21-055 | Sunarizamon | QA-FAIL (critical) |
| EX10-025 | Sunarizamon | QA-FAIL (medium) |
| EX8-046 | Gotsumon | PASS |
| EX8-047 | Sunarizamon | QA-FAIL (low) |

### Batch 2: Champions
| Card | Name | Verdict |
|------|------|---------|
| EX8-048 | Landramon | PASS |
| EX10-028 | Landramon | QA-FAIL (medium) |
| P-167 | Landramon | QA-FAIL (medium) |
| BT14-009 | Gotsumon | PASS |

### Batch 3: Close Tamers
| Card | Name | Verdict |
|------|------|---------|
| EX8-067 | Close | PASS |
| EX10-063 | Close | PASS |
| P-169 | Close | PASS |

### Batch 4: Fragment Ultimates
| Card | Name | Verdict |
|------|------|---------|
| EX8-051 | Proganomon | QA-FAIL (high) |
| EX10-032 | Proganomon | QA-FAIL (critical) |
| EX8-055 | Pyramidimon | QA-FAIL (high) |
| EX10-033 | Pyramidimon | QA-FAIL (high) |

### Batch 5: Complex Megas
| Card | Name | Verdict |
|------|------|---------|
| EX10-034 | Blastmon | QA-FAIL (critical) |
| EX10-036 | Magneticdramon | QA-FAIL (high) |
| EX7-049 | Metallicdramon | ENGINE-LIMITATION |

### Batch 6: Archetype Options
| Card | Name | Verdict |
|------|------|---------|
| BT9-103 | Kongou | PASS |
| EX8-070 | Zofr Kabus | PASS |
| EX10-069 | Unique Emblem: Gravel Hearts | QA-FAIL (critical) |
| LM-031 | Black Scramble | QA-FAIL (medium) |

### Batch 7: Generic Support
| Card | Name | Verdict |
|------|------|---------|
| BT16-082 | Ukkomon | QA-FAIL (low) |
| BT20-055 | Invisimon | QA-FAIL (low) |
| P-039 | Black Memory Boost! | PASS |
| P-107 | Defense Training | QA-FAIL (high) |
| P-206 | Digital Gate Open | QA-FAIL (low) |

---

## QA Failures

### Critical (effect never fires or crashes at runtime)

#### BT21-055 Sunarizamon — cost reduction silently ignored
- Missing `set_timing(EffectTiming.BeforePayCost)` on the digivolution cost reduction effect
- Missing leak guard (`if context.get('card_source') is not card: return False`)
- Without BeforePayCost timing, the engine never evaluates this effect during cost calculation
- Severity: critical — the card's main effect (reduce evo cost by 1) does nothing

#### EX10-032 Proganomon — [Hand][Main] dead code + inherited bug
- **[Hand][Main] effect is dead code**: Uses `OnStartMainPhase` timing and checks `card.permanent_of_this_card() is None` to detect "card is in hand." But `_collect_triggered_effects` only scans `battle_area` and `breeding_area` permanents — hand cards are never iterated. The entire special digivolve-from-hand effect cannot fire.
- **Inherited condition**: `condition_inh` returns `True` unconditionally. Missing `card not in trashed_cards` check and Mineral/Rock trait check on the host permanent. De-Digivolve fires on ANY digi-card trash, not just when this card is trashed.
- Severity: critical — the card's signature mechanic (hand digivolve onto Sunarizamon) is completely nonfunctional

#### EX10-034 Blastmon — wrong timing + runtime crash
- **Wrong timing**: `[All Turns] [Once Per Turn] When Digimon attack` uses `EffectTiming.OnDeclaration` — the engine never fires this timing for attacks. Should use `OnUseAttack`.
- **Runtime TypeError**: `value_fn=lambda: 3000` and `value_fn=lambda: 1` take 0 arguments, but the engine calls `value_fn(result, target, context)` with 3 arguments. Both `CHANGE_DP` and `CHANGE_SECURITY_ATTACK` modifiers will crash.
- Severity: critical — SA+1 and +3000 DP on attack trigger both crash and never fire

#### EX10-069 Unique Emblem: Gravel Hearts — Close checked as trait instead of name
- `_play_sunarizamon_or_close` filter checks `any('Close' in t for t in getattr(c, 'card_traits', []))` — "Close" is a card name, not a trait. Filter never matches Close cards.
- Delay trigger condition uses `event_perm.has_trait('Close')` — same error. Should be `event_perm.contains_card_name('Close')`. The `[Your Turn]` trigger never fires when a Close suspends.
- Severity: critical — both the [Main] play effect and the Delay trigger are broken for Close cards

### High (incorrect behavior)

#### EX8-051 Proganomon — inherited fires too broadly
- Inherited De-Digivolve condition returns `True` unconditionally
- Should check `card not in context.get('trashed_cards', [])` and verify host permanent has Mineral/Rock trait
- Result: De-Digivolve fires whenever ANY digi-card is trashed from ANY permanent, not just when this specific card is trashed from a Mineral/Rock Digimon

#### EX8-055 Pyramidimon — SA+1 doesn't persist for turn
- SA+1 granted via `perm._temp_sa_modifier += 1`
- `_temp_sa_modifier` is cleared by `clear_attack_state()` after each attack resolves
- Card says "gains <Security A. +1> for the turn" — should persist across multiple attacks
- Should use `register_modifier(ModifierType.CHANGE_SECURITY_ATTACK, ..., expiry='end_of_turn')` instead

#### EX10-033 Pyramidimon — cost reduction leaks globally
- `register_modifier(ModifierType.CHANGE_PLAY_COST, target_perm, value_fn=lambda: -reduction)` — the `calculate_play_cost` method iterates ALL `CHANGE_PLAY_COST` modifiers globally without filtering by `target_perm`
- The `target_perm` argument is used only for expiry/cleanup, not for scoping
- Result: cost reduction applies to ALL play cost calculations, not just the targeted opponent Digimon
- Needs a condition function that checks the card being costed matches the target

#### EX10-036 Magneticdramon — condition too strict for cross-Digimon trash
- `_has_mineral_rock_in_sources` condition requires a single Digimon to have 3+ qualifying digi-cards (counter resets per permanent)
- Card says "from any of your Digimon's digivolution cards" — 3 sources can come from multiple Digimon
- Process correctly collects across Digimon, but condition wrongly blocks valid multi-Digimon cost payments

#### P-107 Defense Training — digivolve target wrong
- Delay process passes `perm` (the option card's own battle-area permanent) directly to `effect_digivolve_from_hand`
- Card says "1 of your Digimon may digivolve" — the player must first choose which Digimon
- Should call `effect_select_own_permanent` to pick the target, then `effect_digivolve_from_hand` in the callback

### Medium (player agency missing / fragile implementation)

#### EX10-025 Sunarizamon — auto-selects trash cards
- Process auto-selects first 2 qualifying Mineral/Rock cards from trash in iteration order
- Card says "You may place 2 cards... from your trash" — player should choose which 2

#### EX10-028 Landramon — direct mutation + no player choice
- Directly mutates `card_sources` list (`trash_perm.card_sources.remove(cs_to_trash)`) — Anti-Pattern #10
- Auto-selects first matching source when multiple Mineral/Rock sources exist on same Digimon
- Card says "trashing any 1 card" — implies player selection

#### P-167 Landramon — direct mutation + missing deck choice
- Same direct `card_sources` mutation as EX10-028
- "Return the rest to the top or bottom of the deck" is hardcoded to always return to top
- Should use `effect_choose_branch` to let player choose top vs bottom

#### LM-031 Black Scramble — fragile security effect
- Security effect retrieves "this card" by popping `player.trash_cards[-1]`
- Assumes this option card is the last element trashed — fragile if other cards are trashed during resolution
- Should locate the card by identity reference rather than position

### Low (is_optional flag / minor timing)

#### EX8-047 Sunarizamon — reveal is optional when it shouldn't be
- `effect_reveal_and_select_multi` called with `is_optional=True`
- Card text has no "you may" on the reveal — it's mandatory. Only the individual adds are conditional on finding matches

#### BT16-082 Ukkomon — reveal optional + hatch forced
- `is_optional=True` on `effect_reveal_and_select` — reveal should be mandatory
- `player.hatch()` called unconditionally when conditions met — card says "you **may** hatch" (should be optional)

#### BT20-055 Invisimon — missing turn check on security effect
- `[Security] [End of Opponent's Turn]` condition does not verify it is the opponent's turn
- Could fire at end of owner's own turn if the card is somehow in security at that point

#### P-107 Defense Training — reveal is optional
- `is_optional=True` on `effect_reveal_and_select` — should be mandatory

#### P-206 Digital Gate Open — reveal is optional
- `is_optional=True` on `effect_reveal_and_select_multi` — both selections should be mandatory

---

## Engine Gaps

### EX7-049 Metallicdramon — WhenRemoveField lacks removal-cause context
- Card text: "When this Digimon would leave battle area **other than by one of your effects**"
- The `WhenRemoveField` context does not carry a removal-cause flag
- The script cannot distinguish self-removal from opponent-removal
- Consistent with how all other `WhenRemoveField` scripts in the codebase handle this limitation
- **Not fixable at script level** — requires engine enhancement to pass removal source in context
