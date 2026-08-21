# WITHDRAWN: memory floor is NOT the cause of these divergences

**Status: WITHDRAWN.** I marked this CONFIRMED and was wrong. Twice, on the same claim.

## What actually happened

I read the divergence as "DCGO plays EX12-035 MetalGarurumon, play_cost 12, at memory 0".
It did not. The arithmetic settles it:

```
EX12-016 MetalGreymon  play_cost = 7
recorded memory:  step 5 = 0  ->  step 7 = -7     EXACTLY -7
board_p0 at step 7: ['EX12-016']                  MetalGreymon, not MetalGarurumon
```

DCGO played **EX12-016 for 7** — an entirely legal play. Our engine's `hand[0]` is
`EX12-035` (cost 12), so it masked `play_hand_0` as unaffordable. **DCGO and our engine
disagree about which card is at hand index 0.**

Corroborating: our engine's hand ends `[3]EX12-021 [4]EX12-007`, while the recording's
`initial_hand` ends `[3]EX12-007 [4]EX12-021`. The orders already differ.

## SUPERSEDED: it is not hand ordering either

I also claimed the hands were mis-ordered. **They are not.** Traced through:

- `restore_side_from_dcgo_snapshot` pushes `initial_hand` UNREVERSED (library, security and
  digitama each get `rev()`; the hand deliberately does not).
- So our hand starts `[035, 016, 073, 007, 021]` — exactly `initial_hand`.
- `digivolve_hand_3` then consumes index 3 (`EX12-007`), and digivolving **draws 1**
  (rule 8-1-3-3), taking deck 40 -> 39 and appending the draw.
- Result: `[035, 016, 073, 021, 007]` — exactly what the replay dumped. Self-consistent.

`hand_0` is `EX12-035` in BOTH engines. So the question is not which card was played but
**why DCGO paid 7 for a card that costs 12.**

## Leading explanation: an unrecorded cost reduction (user's hypothesis)

`EX12-035` has an `[Assembly]` alt-path (materials from `zones: [trash]`), and DCGO's play
moved memory by 7 rather than 12 — a reduction of 5.

The telling evidence is what is ABSENT: step 4 carries a `SelectCountEffect` for the
digivolve, but there is **no selection row whatsoever** between the two plays at steps 5
and 6. If DCGO declared an Assembly, its material-count prompt was never recorded.

**If so this is a recorder gap — a fourth one** — and it is invisible in exactly the way the
others were: the recording looks complete, replays cleanly up to the point of use, and then
produces a divergence that reads as an engine rules bug.

## Status of both of my hypotheses

- memory-floor rule: **UNTESTED** by this corpus. No cost-12 play at memory 0 occurred.
- hand ordering: **DISPROVED**. Our reconstruction is correct.
- unrecorded cost reduction: **LEADING, not proven.** Needs the DCGO-side check below.

## What would settle it

1. Read DCGO's `EX12_035.cs` and find whether its Assembly/cost-reduction path routes
   through a prompt the recorder hooks, or through a path with no `UserSelectionManager`
   call at all (the latter would explain the silence).
2. Check whether P0's trash was genuinely empty in DCGO at that moment — our engine says it
   was, but that is our view, and the same mistake (trusting our own state as evidence about
   DCGO) is what produced two wrong findings already.
3. Add the material-count/Assembly declaration to the recording schema, then re-run.

## Why the rules argument does not rescue it

The §16/§1-4-2 analysis below is preserved for reference, but it is moot here: no play of a
cost-12 card at memory 0 ever occurred. Treat our engine's memory-floor rule as UNTESTED by
this corpus, not as vindicated or refuted.

## How this went wrong twice

1. Claimed CONFIRMED from `cards.json` alone, which does not expose `[Assembly]` paths.
2. Retracted correctly when challenged, then found the real blocker (no memory in recordings).
3. Closed the schema gap, saw memory agree, and re-confirmed — but I matched the divergence to
   the *card our engine had at hand[0]* rather than checking what DCGO's memory delta proved it
   had actually played. The memory field I had just added contained the disproof the whole time.

The lesson is specific: when two engines disagree about an indexed action, verify WHICH OBJECT
each side resolved the index to before reasoning about the action's legality.

---

## Preserved for reference: the original rules analysis

## Our rule

`code/digimon-engine/src/action/mask.rs` (and `game/memory.rs::pay_memory`):

```rust
if (game.memory - cost) < game.rules.memory_range.0 { continue; }   // unplayable
```

With `memory_range = (-10, 10)`: `1 - 12 = -11 < -10`, so the play is masked out. `pay_memory` documents the same idea — *"Returns true if affordable (memory stays above rules.memory_range.0)"*.

## Why that is wrong

**1-4-2-2:** "0 on the memory gauge is the center, and it has the numbers 1 through 10 on both the left and the right. The highest number on the memory gauge is 10 on both the left and the right. **The memory won't exceed 10.**"

That is a **clamp on the gauge's value**, not a restriction on which costs may be paid.

**1-3-11-1** is the only payment restriction: "you can't declare to use a card if you can't pay its cost or alternate cost."

**2-6-1** defines play cost with no sufficiency condition. **7-1-3-2** simply says "The player pays the specified play cost."

Nothing anywhere makes a cost unpayable because it would drive the marker past the end of the gauge. Paying 12 from memory 1 moves the marker to the opponent's 10 and the turn ends — ordinary hard-casting, a routine play pattern.

DCGO (source priority #2, battle-tested) agrees: it made the play.

## Blast radius

12 call sites across 7 files use `memory_range.0` as an affordability floor:

- `action/mask.rs` — play legality (this bug)
- `game/memory.rs` — `pay_memory` returns false
- `game_actions/link.rs`, `game_actions/misc.rs`, `game_actions/mod.rs`
- `effect_context/action/digivolve.rs`
- `game/mod.rs` — `pay_memory_unchecked` exists as the deliberate opt-out

So this is a consistent modelling decision, not a one-line slip. Fixing it means deciding that the floor clamps rather than gates, then making `pay_memory` clamp and removing the mask gate.

## Why per-card tests never caught it

This is a **global rules** bug, not a card effect. Every EX12 card's behavioural test passes, because each exercises its own text with adequate memory. Only a differential oracle replaying real games at realistic memory levels surfaces it.

## Recommended fix

1. `pay_memory` clamps to `memory_range.0` instead of returning false (matching 1-4-2-2), keeping the `false` return only for genuinely unpayable non-memory costs.
2. Drop the mask's memory gate for plays; keep cost reduction logic untouched.
3. Re-check each of the other call sites individually — some may encode a *different* legality rule that happens to share the constant.
4. Guard with a behavioural test: memory 1, hard-cast a cost-12 Lv.6, assert it is legal, memory lands at -10, and the turn ends.

**Do not blanket-replace all 12 sites.** Judge each, as with the DCGO `isYou` sweep.
