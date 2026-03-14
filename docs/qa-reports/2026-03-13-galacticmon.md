# Galacticmon Archetype QA Report

**Date**: 2026-03-13
**Tester**: Claude (API-driven gameplay + script audit)
**Cards Tested**: 33 (all unvalidated)
**Method**: Debug game API with deterministic hands + script source review

## Summary

| Verdict | Count | Cards |
|---------|-------|-------|
| PASS | 12 | BT21-006, BT18-060, BT21-056, BT11-065, BT18-092, BT21-087, BT21-060, BT21-062, EX11-046, ST13-08, LM-048, BT11-111 |
| PARTIAL | 7 | BT21-098, EX11-066, P-094, EX11-070, EX11-036, BT7-105, EX6-072 |
| QA-FAIL | 14 | BT11-061, BT11-105, BT21-058, EX11-006, EX11-027, EX11-029, EX11-033, EX11-040, EX11-042, EX11-045, EX11-062, EX11-073, BT18-065, P-151 |

**Issues found**: 27

---

## Per-Card Results

### PASS Cards

#### BT21-006 Tsumemon (Lv.2 Egg)
- **Inherited**: +3000 DP when 4+ Vemmon in evo cards
- Condition correctly checks `card_sources[:-1]` for Vemmon count
- **Verdict**: PASS

#### BT18-060 Vemmon (Lv.3)
- **[On Play]**: Reveal top 3, add 1 Vemmon-text to hand, place 1 Vemmon as bottom evo card, rest to deck bottom
- Tested via gameplay: reveal phase entered, card added to hand, Vemmon placed under Digimon (source_count increased), rest returned to deck
- Memory correctly deducted (cost 3)
- **Inherited**: Digi cost -1 for Vemmon-text Digimon - registered
- **Verdict**: PASS

#### BT21-056 Vemmon (Lv.3)
- **[On Play]**: Trash Vemmon-text from hand, return non-egg Vemmon-text from trash to hand
- Tested via gameplay: SelectHand phase, selected card trashed, Fusionize returned from trash
- **Minor**: No Decline option in SelectHand for the "By trashing" cost (systemic issue)
- **Verdict**: PASS (systemic optional-cost issue noted)

#### BT11-065 Snatchmon (Lv.4)
- **[When Digivolving]**: Places up to 2 Vemmon from trash as bottom evo cards, checks 4+ Vemmon for Fusionize recovery
- Tested: 2 Vemmon placed from trash, source count increased
- **Inherited**: Unsuspend + Blocker on Vemmon return to deck bottom - correctly uses OnDigivolutionCardReturnToDeckBottom timing
- **Minor**: Auto-places Vemmon without "you may" player choice
- **Verdict**: PASS

#### BT18-092 Zenith (Tamer)
- **[Start of Main Phase]**: Trash Vemmon from hand -> Draw 1 + gain 1 memory
- Tested: SelectHand triggered, Decline/Pass (action 62) available - correctly optional
- **[Your Turn]** attack effect: Suspend tamer + return 2 Vemmon -> De-Digivolve 1
- Script review: correct implementation
- **Verdict**: PASS

#### BT21-087 Zenith (Tamer)
- **[Start of Your Turn]**: If 2 or less memory, set to 3
- Tested: Memory correctly set from -3 to 3 on new turn
- **[On Play]**: Reveal top 3, play Vemmon free OR add Vemmon-text to hand, trash rest
- Tested: Effect auto-resolved (played Vemmon to field, trashed rest)
- **Security**: Play without cost - registered
- **Verdict**: PASS

#### BT21-060 Destromon (Lv.5)
- **Alt digivolve**: From Vemmon for cost 6 - correct
- **[When Digivolving]**: Stack trash immunity + De-Digivolve per 2 Vemmon
- Tested: Effect logged, IMMUNE_FROM_STACK_TRASHING modifier registered
- **[All Turns]**: Play Vemmon from evo cards when leaving - WhenRemoveField timing
- **Inherited**: Return 2 Vemmon to deck bottom to end attack - fires OnDigivolutionCardReturnToDeckBottom
- **Verdict**: PASS

#### BT21-062 Galacticmon (Lv.6)
- **[When Digivolving]**: Place 4 Vemmon-text from trash as evo cards, use Ragnarok Cannon free
- Tested: 4 cards placed from trash (trash count decreased by 4)
- **[Start of Main Phase]**: Delete 1 opponent Digimon
- Tested: SelectTarget phase with opponent Digimon as target
- **[All Turns]**: Return 4 Vemmon to deck bottom to prevent leaving
- Script: correct WhenRemoveField implementation
- **Verdict**: PASS

#### EX11-046 Galacticmon (Lv.6)
- **Alt digivolve**: From Snatchmon (cost 9), from Galacticmon (cost 5)
- **[On Play][When Digivolving]**: Choose highest cost opponent Digimon, delete all others. If 4+ Vemmon gain Blocker + immunity
- Tested: Effect triggered correctly, Blocker + CANNOT_BE_AFFECTED modifiers registered
- **[End of Opponent's Turn]**: Digivolve into Galacticmon from hand/trash free
- **Verdict**: PASS

#### ST13-08 Chikurimon (Lv.3)
- **[All Turns]**: Players can't reduce play costs
- Tested: Plays correctly, effect registered on field
- **Verdict**: PASS

#### LM-048 Chrome Memory Boost!
- Script review: Reveal top 3, add green/black Digimon, Delay gain 2 memory, Security place
- Implementation looks correct
- **Verdict**: PASS

#### BT11-111 Galacticmon (Lv.6)
- Script review: Alt digivolve from Snatchmon (cost 9), When Digivolving place 4 Vemmon / 8+ delete, WhenRemoveField return 4 Vemmon, Start of Main Phase trash security
- All implementations correct
- **Minor**: Auto-places Vemmon without "you may" choice
- **Verdict**: PASS

---

### PARTIAL Cards

#### BT21-098 Ragnarok Cannon (Option)
- **[Main]**: Delete lowest cost opponent Digimon, place in battle area - PASS
- **Delay**: Triggers when Galacticmon attacks - correct condition
- **Issue #1** (Low): Delay effect logic checks `if opp_digimon` as proxy for "didn't delete" instead of actually tracking whether deletion succeeded. Could fail if Digimon survives deletion.
- **Security**: Play Vemmon-text cost<=6 from hand/trash, add to hand - PASS
- **Verdict**: PARTIAL

#### EX11-066 Xeno (Tamer)
- **On Play / Start of Main Phase**: Trash Vemmon-text -> Draw 1 + gain 1 memory - Tested working
- **Issue #2** (High): Effect 3 (All Turns trigger on other Digimon play/digivolve) documented as **engine gap** - won't fire when other Digimon are played. The suspend+reveal+place Vemmon mechanic is the core value of this card and doesn't work.
- "Also treated as Zenith" - engine gap (name aliasing)
- **Verdict**: PARTIAL

#### P-094 Destromon (Lv.5)
- **[On Play][When Digivolving]**: Budget-based multi-delete with Vemmon scaling - well implemented
- **Issue #3** (High): Inherited effect (redirect attack by placing 2 Vemmon from Galacticmon to deck bottom) is **BLOCKED** - redirect_attack engine gap. Process is no-op.
- **Verdict**: PARTIAL

#### EX11-070 Unchained (Tamer)
- Security play + memory set to 3: PASS
- End of Turn DNA digivolve + Mind Link: Present
- **Issue #4** (Medium): Missing inherited "opponent's effects can't trash stacked cards" protection
- **Issue #5** (Medium): Missing inherited "[End of All Turns] play 1 [Unchained] from evo cards" effect
- **Verdict**: PARTIAL

#### EX11-036 Dalphomon (Lv.6)
- Vortex keyword set correctly
- On Play/When Digivolving/When Attacking suspend effects present
- End of Turn digivolve other Digimon into black Maquinamon-text present
- **Issue #6** (Medium): Inherited effect "this Digimon may attack" after link trigger is **BLOCKED** - force_attack engine gap
- **Verdict**: PARTIAL

#### BT7-105 Pride Memory Boost! (Option)
- Main effect reveal + play black Digimon free: working
- Delay gain 2 memory: present
- **Issue #7** (Medium): When played, both the Main effect and Security effect appear to trigger (both log as OnEnterFieldAnyone), potentially causing duplicate placement in battle area. The `is_security_effect` flag may not be properly gating execution to security-only context.
- **Verdict**: PARTIAL

#### EX6-072 Mega Digimon Assembly! (Option)
- **Issue #8** (Medium): DNA digivolve filter checks `lv >= 6` but card says target must be "level 7 Digimon card" - allows digivolving into Lv6 when it should be Lv7 only
- **Issue #9** (Low): Color ignore condition is a no-op stub
- Security: auto-picks from trash without player choice
- **Verdict**: PARTIAL

---

### QA-FAIL Cards

#### BT11-061 Vemmon (Lv.3)
- **Issue #10** (Critical): The [Main] effect (suspend to reveal top 3, add Snatchmon/Destromon/Galacticmon/Fusionize to hand, place Vemmon as bottom evo card) uses `EffectTiming.OnDeclaration` which has **no action mask or decoder entry**. The action space only supports Training and Delay for field effects (1000-1999 range). This core effect is completely untriggerable during gameplay.
- **Inherited**: Digi cost -1 for Destromon/Galacticmon - registered correctly
- **Verdict**: QA-FAIL

#### BT11-105 Fusionize (Option)
- **Issue #11** (Critical): Digivolution cost computed but **never deducted from memory** (lines 120-126 compute cost variable but never call `game.memory -= cost` or equivalent)
- **Issue #12** (Medium): Auto-selects trash card and target Digimon without player choice
- **Issue #13** (Medium): "you may" not enforced - no option to decline digivolve
- **Verdict**: QA-FAIL

#### BT21-058 Snatchmon (Lv.4)
- **Issue #14** (High): Card says "trash the rest" after reveal but script places remaining cards at deck bottom instead of trashing
- **Issue #15** (High): Card says "place up to 2 [Vemmon]" but script only places 1
- **Issue #16** (Medium): Card says "1 of your Digimon's" (any Digimon) but script only targets "this Digimon"
- **Issue #17** (Low): Auto-places without "you may" choice
- **Inherited**: Delete opponent Digimon cost<=4 on Vemmon return - correct
- **Verdict**: QA-FAIL

#### EX11-006 Flickmon (Lv.2 Egg)
- **Issue #18** (High): Missing "linked with [Maquinamon]" condition check - only checks card text for Maquinamon
- **Issue #19** (High): Missing digivolution cost reduction of 2 mentioned in card text
- **Verdict**: QA-FAIL

#### EX11-027 Maquinamon (Lv.3)
- **Issue #20** (High): On Play reveals top 3 but only adds 1 card - card says "Add 1 [Maquinamon] AND 1 card with [Maquinamon] in its text" (two separate adds)
- **Issue #21** (High): Missing link step after reveal - card says "Then, you may link this Digimon or 1 [Maquinamon] in your hand to 1 of your other Digimon"
- Link Requirements metadata not visible in script
- **Verdict**: QA-FAIL

#### EX11-029 Turbomon (Lv.4)
- **Issue #22** (Medium): Card says [On Play] [When Digivolving] but script only has When Digivolving - missing On Play timing flag
- **Issue #23** (Medium): When linked plays Unchained but missing "if you have 1 or fewer Tamers" condition check
- **Issue #24** (High): Missing Piercing inherited keyword
- **Verdict**: QA-FAIL

#### EX11-033 Maneuvermon (Lv.5)
- On Play/When Digivolving link and When Linked suspend effects present
- **Issue #25**: Needs deeper testing - On Play flag may be missing similar to EX11-029
- **Verdict**: QA-FAIL (pending deeper validation of On Play timing)

#### EX11-040 Mulemon (Lv.4)
- On Play/When Digivolving link Maquinamon present
- **Issue #26** (Medium): When linked plays Unchained but missing "if you have 1 or fewer Tamers" condition check (same as EX11-029)
- Inherited Reboot keyword needs verification
- **Verdict**: QA-FAIL

#### EX11-042 MockingBirdmon (Lv.5)
- On Play/When Digivolving play Maquinamon present
- When linked delete opponent Digimon cost<=5 present
- **Inherited**: Redirect attack is **BLOCKED** - redirect_attack stub (no-op)
- **Verdict**: QA-FAIL (inherited completely non-functional)

#### EX11-045 Metatromon (Lv.6)
- Blocker, De-Digivolve 2, can't digivolve modifier, End of Turn digivolve effects present
- Inherited "when effects add to evo cards, delete lowest cost" present
- **Issue**: On Play flag presence needs verification - script structure needs deeper review
- **Verdict**: QA-FAIL (pending deeper validation)

#### EX11-062 Shoto Kazama (Tamer)
- Start of Turn memory set to 3: likely correct
- **All Turns** when Digimon suspend by effect - complex trigger mechanism
- Vortex attacks player condition present
- **Issue**: The "when any Digimon suspend" trigger likely has same engine gap as EX11-066 (can't observe other permanents' state changes)
- **Verdict**: QA-FAIL (trigger mechanism unreliable)

#### EX11-073 ExMaquinamon (Lv.7)
- **Issue #27** (Critical): End of Opponent's Turn effect trashes **player's own security** instead of **opponent's security** - card says "trash your opponent's top security card"
- Missing SA+1 (Security Attack +1) keyword
- Missing Blocker keyword
- When Digivolving: missing "If DNA digivolving" condition check
- **Verdict**: QA-FAIL

#### BT18-065 Snatchmon (Lv.4)
- DigiXros with Vemmon: basic functionality works (tested - cost correctly reduced)
- When Digivolving place 2 Vemmon from trash: present
- End of Turn digivolve from hand if 4+ evo cards: present
- **Issue**: The DigiXros "while you have no Digimon other than Vemmon, trash cards also qualify" condition needs verification
- Inherited: Unsuspend + Blocker on Vemmon return - correct implementation
- **Verdict**: QA-FAIL (pending DigiXros condition validation)

#### P-151 Digimon Liberator (Option)
- LIBERATOR trait ignore color: stub
- Reveal + play cost<=3: present but trait check needs validation
- **Issue**: Contains stub descriptive tags suggesting incomplete implementation
- **Verdict**: QA-FAIL (insufficient trait-based filtering validation)

---

## Systemic Issues

1. **No action mask for field [Main] effects** (affects BT11-061): The engine only supports Training and Delay in the 1000-1999 action range. Custom [Main] effects (like "suspend to reveal") have no way to be triggered by the player.

2. **"By trashing" optional costs missing Decline option**: Multiple scripts (BT21-056, EX11-066) enter SelectHand for the trash cost but don't offer a Decline/Pass option for the "By trashing" pattern, making the cost mandatory when it should be optional.

3. **Engine gap for cross-permanent triggers**: Effects like "When your Digimon are played/digivolve" on Tamers don't trigger because OnEnterFieldAnyone only dispatches to the permanent that was played/digivolved.

4. **redirect_attack engine gap**: Multiple cards (P-094 inherited, EX11-042 inherited) need attack redirection which is not implemented.

5. **force_attack engine gap**: EX11-036 inherited needs to grant attack ability which is not implemented.
