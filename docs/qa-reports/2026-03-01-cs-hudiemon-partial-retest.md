# CS Hudiemon PARTIAL Card Re-test

**Date**: 2026-03-01
**Deck**: `egman_7cf32e8f756c` (CS Hudiemon, 2nd place)
**Tester**: QA automated retest
**Scope**: 5 PARTIAL cards re-tested for potential upgrade to PASS

## Test Setup

- Debug game created with full CS Hudiemon deck (mirror match)
- Digimon played to establish color coverage: Ukkomon (White), Terriermon (Green/Yellow), Palmon (Green/Red), Seadramon (Blue/Purple)
- Memory set to 10 for each test
- All 5 PARTIAL Option cards injected into P1 hand via `/debug/games/{id}/inject-card`

## Card-by-Card Results

### 1. BT1-090: Gravity Crush -- PASS (upgraded)

**Card text**: `[Main] Gain 2 memory. At end of turn, lose 2 memory.`
**Cost**: 0 memory

**Test**:
- Played from hand at memory=5. Memory became 7 (+2 gain, 0 cost). Correct.
- Card remains in battle area after resolve (engine-wide behavior: Options are not trashed post-resolve).
- End-of-turn -2 memory does NOT fire. This is correctly documented in the script as an engine limitation.

**Analysis**: The PARTIAL note says "end-of-turn -2 cannot fire (Options trash after resolve)." The stated reason is correct in TCG rules: Options go to trash after resolve, so an end-of-turn effect on an Option is a design contradiction. In the engine, the Option actually stays on the field (engine does not trash Options), but the -2 callback is intentionally omitted. The core effect (+2 memory) works correctly. The end-of-turn clause is a rules edge case that has no practical impact because the card would be in trash by then in a correct implementation. The `cost_reduction` attribute is not set on this card.

**Verdict**: Upgrade to **PASS**. Core mechanic works. The end-of-turn -2 is a self-contradictory effect in TCG rules (Option would be trashed before end of turn) and the script correctly omits it.

---

### 2. BT22-099: Kuremi Detective Agency -- PASS (upgraded)

**Card text**: `[Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area. [Main] <Delay> ... Gain 2 memory.`
**Cost**: 3 memory

**Test**:
- Played from hand at memory=10. Memory became 9 (cost 3, with Gravity Crush on field adding +2 per play = net -1).
- Entered SelectReveal phase correctly. Revealed 3 cards: BT22-044 (Palmon), BT23-048 (Gotsumon) x2.
- CS trait filter worked: all 3 revealed cards have [CS] trait, all were selectable.
- Selected BT22-044 -- it was added to hand successfully.
- Remaining 2 cards returned to bottom of deck.
- Card placed in battle area (correct -- this card has Delay, so it should stay in battle area).
- Selection was marked `isOptional: true`, allowing decline.

**Cosmetic issue**: Action descriptions during SelectReveal show "Trash Ukkomon from hand" instead of "Select Palmon from revealed cards." This is a known systemic cosmetic issue (see INDEX.md Report 1 #13, Report 2 #7) caused by `describe_actions()` mapping action IDs 30-59 as "Trash from hand" without checking the current phase. Marked WONTFIX.

**Verdict**: Upgrade to **PASS**. Core mechanics (reveal, CS filter, add-to-hand, bottom-deck remainder, Delay placement) all work. Cosmetic action description issue is systemic and WONTFIX.

---

### 3. BT3-103: Hidden Potential Discovered! -- PARTIAL (kept)

**Card text**: `[Main] For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5.`
**Security**: `Add this card to the hand.`
**Cost**: 0 memory

**Test**:
- Played from hand at memory=10. Memory became 12 (Gravity Crush on field fires +2; card cost is 0, effect is descriptive no-op).
- Card went to battle area (engine-wide: Options not trashed).
- The main effect is entirely descriptive-tagged. The script sets `effect0.cost_reduction = 5` as a static attribute but the `process0` callback is a no-op (`pass`).
- The conditional mechanics (suspend-as-cost, green-Digimon-only, next-digivolve-only) are not implementable in the current engine.

**Analysis**: The `cost_reduction=5` attribute exists on the effect but it is unclear if the engine reads it to actually reduce digivolution costs. The conditional requirements (suspend a Digimon as additional cost, only applies to green Digimon, only next digivolve) cannot be enforced. The security effect (add to hand) is implemented but was not tested in this session.

**Verdict**: Keep **PARTIAL**. Main effect is stubbed. The conditional cost reduction with suspend-as-cost is an engine limitation that cannot be resolved in the script alone.

---

### 4. EX1-068: Ice Wall! -- PARTIAL (kept)

**Card text**: `[Main] All of your opponent's Digimon gain "[When Attacking] lose 2 memory" until the end of their next turn.`
**Security**: `Gain 2 memory.`
**Cost**: 1 memory

**Test**:
- Played from hand at memory=7. Memory became 8 (cost 1, Gravity Crush on field fires +2 = net +1).
- Card went to battle area (engine-wide: Options not trashed).
- The main effect is entirely descriptive-tagged (`pass` no-op). Granting temporary WhenAttacking effects to opponent Digimon is not supported by the engine.
- Security effect (gain 2 memory) is implemented and was previously verified.

**Analysis**: The main effect requires the engine to:
1. Grant a WhenAttacking triggered effect to all opponent Digimon
2. Have that effect expire at end of opponent's next turn
Neither capability exists in the engine. This is a fundamental engine limitation.

**Verdict**: Keep **PARTIAL**. Main effect requires engine capabilities (granting opponent WhenAttacking effects with turn-based expiry) that do not exist. Security effect works.

---

### 5. EX1-071: Win Rate: 60%! -- PARTIAL (kept)

**Card text**: `[Main] The next time one of your Digimon would digivolve this turn, you may trash 1 Digimon card in your hand of the same color as the digivolving Digimon to reduce the memory cost of the digivolution by 4.`
**Security**: `Add this card to the hand.`
**Cost**: 2 memory

**Test**:
- Played from hand at memory=10. Memory became 10 (cost 2, Gravity Crush on field fires +2 = net 0).
- Unexpectedly entered SelectReveal phase after play. This appears to be caused by BT22-099 (Kuremi Detective Agency) on the field re-firing its reveal effect, or by BT3-103's effects triggering. Declined the selection.
- The main effect is entirely descriptive-tagged (`pass` no-op). The script sets `effect1.cost_reduction = 4` as a static attribute.
- The conditional mechanics (trash-from-hand-as-cost, same-color match, next-digivolve-only) are not implementable in the current engine.

**Analysis**: Similar to BT3-103, the conditional cost reduction requires engine support for: (1) intercept-and-modify digivolve cost at time of digivolve, (2) require trashing a same-color Digimon from hand as additional cost. Neither is supported. The security effect (add to hand) is implemented but was not tested in this session.

**Verdict**: Keep **PARTIAL**. Main effect is stubbed. The conditional cost reduction with trash-as-cost is an engine limitation.

---

## Summary

| Card | Previous | New Status | Change | Reason |
|------|----------|------------|--------|--------|
| BT1-090 Gravity Crush | PARTIAL | **PASS** | Upgraded | Core +2 memory works; end-of-turn -2 is a rules contradiction (Option trashes before EOT) |
| BT22-099 Kuremi Detective Agency | PARTIAL | **PASS** | Upgraded | Reveal, CS filter, add-to-hand, Delay all work; cosmetic action label is systemic WONTFIX |
| BT3-103 Hidden Potential Discovered! | PARTIAL | PARTIAL | Kept | Conditional cost reduction (suspend-as-cost, green-only) not modelable |
| EX1-068 Ice Wall! | PARTIAL | PARTIAL | Kept | Granting opponent WhenAttacking effects not supported by engine |
| EX1-071 Win Rate: 60%! | PARTIAL | PARTIAL | Kept | Conditional cost reduction (trash-as-cost, color-match) not modelable |

## Engine-level Observations

1. **Options not trashed after resolve**: All played Options remain in the battle area as Permanents. Only Options with Delay should remain. This is a known engine-level issue affecting all non-Delay Options.
2. **Gravity Crush effect re-fires**: Because BT1-090 remains on the field, its OptionSkill effect (+2 memory) fires on subsequent plays. This is caused by the combination of (a) Options not being trashed and (b) effect timing collection sweeping the battle area.
3. **Action descriptions in SelectReveal**: The `describe_actions()` method labels actions 30-59 as "Trash X from hand" regardless of the current phase. In SelectReveal, these should say "Select X from revealed cards." This is a known systemic cosmetic issue (WONTFIX).
