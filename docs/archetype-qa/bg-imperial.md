# Archetype QA: BG Imperial
Date: 2026-03-14
Total cards: 25

## Summary
- PASS: 12
- IMPLEMENTED: 8
- BLOCKED: 2

## Card-by-Card Verdicts

### BT12-002 DemiVeemon — PASS
Inherited [When Attacking] [Once Per Turn] draw 1 if green Digimon in play. Correct.

### BT3-002 DemiVeemon — PASS
Inherited [When Attacking] [Once Per Turn] draw 1 if Jamming. Correct.

### BT12-021 Veemon — PASS
[On Play] reveal top 3, add Imperialdramon/Free Digimon + Davis Motomiya Tamer. Uses `effect_reveal_and_select_multi` with proper selection. BLOCKED: inherited End of Your Turn DNA digivolve (engine gap).

### P-117 Veemon — PASS
BeforePayCost digivolution cost -1 for [Free] if Tamer in play. Inherited draw 1 if 2+ colors. Correct.

### BT12-047 Wormmon — PASS
[On Play] reveal top 3, add Imperialdramon/Free Digimon + Ken Ichijoji Tamer. Uses `effect_reveal_and_select_multi`. BLOCKED: inherited End of Your Turn DNA digivolve (engine gap).

### EX1-014 ExVeemon — PASS
Jamming keyword. Inherited Jamming conditional on Imperialdramon name or Free trait. Correct.

### ST9-09 Stingmon — PASS
BeforePayCost play cost -1 with leak guard. Inherited draw 1 if blue Digimon. Correct.

### BT12-022 ExVeemon — PASS
WhenDigivolving DNA into green gains 1 memory. Inherited Jamming. Correct.

### BT12-050 Stingmon — PASS
WhenDigivolving DNA into blue gains 1 memory. Inherited Piercing. Correct.

### ST9-05 Paildramon — PASS
WhenDigivolving DNA: return opp Digimon <=6000 DP to deck bottom with selection. When Attacking once per turn unsuspend self. Correct.

### BT16-027 Imperialdramon: Fighter Mode — PASS
Blast Digivolve. On Play/When Digivolving: bottom deck opp Digimon with <= digi-card count. End of Attack: unsuspend self + conditional Dragon Mode bottom deck. All with proper selection. Correct.

### BT3-103 Hidden Potential Discovered! — PASS (main BLOCKED)
Main effect BLOCKED: one-shot digivolution cost reduction hook not available in engine. Security add-to-hand works. Approximation uses CHANGE_DIGIVOLUTION_COST modifier for all green Digimon with end_of_turn expiry.

### BT12-028 Paildramon — IMPLEMENTED
- **Fix**: `register_modifier` argument order was reversed (ModifierType first, Permanent second). Fixed to correct positional order (Permanent first, ModifierType second) so CANNOT_ATTACK modifiers actually register correctly.
- Inherited [End of Attack] condition correctly checks top card only via `perm.contains_card_name('Imperialdramon')` and `perm.has_trait('Free')` -- both are top-card-only methods.

### BT16-025 Paildramon — IMPLEMENTED
- **Fix**: `register_modifier` argument order was reversed for CANNOT_UNSUSPEND. Fixed to correct positional order so the modifier actually applies.

### BT16-028 Imperialdramon: Dragon Mode — PASS
Alt-digi from Paildramon and Dinobeemon. When Digivolving: CANNOT_UNSUSPEND + suspend/unsuspend trade (correct arg order). All Turns reactive trigger. Correct.

### BT20-020 Imperialdramon: Fighter Mode — PASS
Raid, Piercing. When Digivolving play restriction + conditional security trash. OnLoseSecurity delete. `register_modifier` uses correct arg order. Correct.

### BT12-031 Imperialdramon: Fighter Mode — IMPLEMENTED
- **Fix**: `register_modifier` argument order was reversed for CHANGE_DP. Fixed to correct positional order.
- **Fix**: `value_fn` was `lambda: 1000 * count` (0 args) but engine calls `value_fn(current, target, ctx)` (3 args). Fixed to `lambda current, target, ctx: current + 1000 * count`.

### BT21-037 Lighdramon — IMPLEMENTED
- **Fix**: DP +2000 was applied with `perm.change_dp(2000)` (no duration). Changed to `register_modifier(perm, ModifierType.CHANGE_DP, ...)` with `expiry='end_of_opponent_turn'` per C# `EffectDuration.UntilOpponentTurnEnd`.
- **Fix**: `value_fn` signature corrected to `lambda current, target, ctx: current + 2000` (3-arg).
- **Fix**: Effect order corrected to match C#: suspend first, then DP change (was reversed).
- **Fix**: Suspend filter now checks `not p.is_suspended` to match C#'s `CanSuspend` check.

### ST9-06 Imperialdramon Dragon Mode — IMPLEMENTED
- **Fix**: Auto-selected first qualifying blue/green Digimon from digi-stack. Replaced with proper `request_selection` using `GamePhase.SelectSource` for both blue and green card selection, matching C#'s `SelectCardEffect` with `Root.DigivolutionCards`.

### BT3-093 Davis Motomiya — IMPLEMENTED
- **Fix**: Memory-to-3 effect used `OnStartMainPhase` timing. Changed to `OnStartTurn` to match C#'s `SetMemoryTo3TamerEffect` which fires at `OnStartTurn`.
- On-play reveal uses `effect_reveal_and_select_multi` with proper selection phases. Correct.

### LM-030 Green Scramble — IMPLEMENTED
- **Fix**: Removed spurious `effect0.cost_reduction = 3` from OptionSkill effect (the cost reduction is already handled by `effect_digivolve_from_hand(cost_reduction=3)`).
- **Fix**: Delay activation auto-picked first green Digimon from trash. Replaced with `request_selection` using `GamePhase.SelectTrash` for proper player choice.
- **Fix**: Delay condition now requires opponent to have at least 1 Digimon, matching C# condition.

### BT17-077 Imperialdramon: Paladin Mode — IMPLEMENTED
- **Fix**: [When Attacking] unsuspend fired unconditionally even if deck-bottom bounce was blocked by protection. Now checks whether the target was actually removed from the opponent's battle area before unsuspending, matching C#'s `DeckBouncePeremanentAndProcessAccordingToResult` success/failure pattern.

### BT17-097 Return to the Primogenitor — PASS
Main effect: digivolve from hand with cost -4, delay placement. Delay: prevent deletion by digivolving into Imperialdramon + CANNOT_BE_DESTROYED. Security: play Davis/Ken Tamer from hand/trash. All with proper selection. Correct.

### BT16-085 Davis Motomiya & Ken Ichijoji — PASS
Security play self. Start of Main: play Veemon/Wormmon free with bounce tracking. Your Turn: suspend to gain 1 memory on blue/green digivolve. DNA sub-effect trashes digi-cards. All correct.

### BT16-040 Wormmon — PASS (known limitation)
Alt-digi from Minomon. Inherited When Attacking suspend with selection. Start of Main / On Play trash digivolve with player selection for both permanent and trash card. Known limitation: `perm_filter` globally checks if any qualifying trash card exists rather than validating per-permanent digivolution compatibility (engine lacks `CanPlayCardTargetFrame` equivalent). Acceptable for RL.

## Blocked Cards
### BT12-021 / BT12-047 (inherited only)
- **Effect**: [End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand.
- **Missing mechanic**: End-of-turn DNA digivolve as inherited effect is not supported by the engine.

### BT3-103 Hidden Potential Discovered! (main effect only)
- **Effect**: [Main] For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5.
- **Missing mechanic**: Player-level temporary one-shot digivolution cost reduction hook with suspend-as-cost.
- **Current approximation**: Uses CHANGE_DIGIVOLUTION_COST modifier on all field Digimon for the turn (not one-shot, not green-only).
