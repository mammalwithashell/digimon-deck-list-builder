# FINDING: in DCGO, a rejected play attempt reorders the hand (violates 7-1-2-3)

**Status:** confirmed from recorded data with per-action `card_id`. Not yet minimised to a standalone repro.

## The rule

**7-1-2-3:** "If a card can no longer be played after revealing it as part of the procedure for playing it, the revealed card is **returned to its original location**. This isn't considered removal from an area. In addition, the memory doesn't move when a card can't be played because its cost can't be paid."

Returned to its **original location** — i.e. the same position in hand.

## What DCGO actually does

Seed 700004, P0's opening sequence (`card_id` and `cost_paid` are recorded per action, so this is observed, not inferred):

```
initial_hand: [EX12-035, EX12-016, EX12-073, EX12-007, EX12-021]

step 3  id=459  digivolve   card=EX12-007  cost_paid=0   <- hand index 3 -> EX12-007, matches initial_hand
step 5  id=0    play_hand_0 card=EX12-035  NO action_detail -> NEVER COMPLETED
step 6  id=0    play_hand_0 card=EX12-016  cost_paid=7   <- hand index 0 now resolves to EX12-016
```

After step 3 consumes index 3 and the digivolve draw appends, the hand is
`[EX12-035, EX12-016, EX12-073, EX12-021, <drawn>]`. `EX12-035` is at index 0, and step 5
confirms it. The step-5 play is rejected (cost 12 at memory 0, correctly refused). By step 6,
index 0 resolves to `EX12-016` — consistent with `EX12-035` having been moved to the **back**
of the hand rather than returned to index 0.

Memory correctly does not move (0 at both steps), so DCGO honours the second half of 7-1-2-3
and violates the first.

## Why it matters to the harness

`dcgo-replay` now skips recorded actions with no completion (`a95ce737a`), which is correct
and necessary — but **not sufficient**. The rejected attempt has an observable side effect in
DCGO that our engine does not reproduce (and should not, since it is a rules violation). So
our engine's hand ordering diverges from DCGO's from that point, and the *next* hand-indexed
action mismatches.

Verified: after the skip landed, the same recording still fails, now at the following action.

## Options, in preference order

1. **Stop DCGO's AI from queueing unaffordable plays.** The cleanest fix — no rejected attempt,
   no reorder, no recorded phantom action. Requires finding where the bot picks a play and
   adding the affordability check it currently lacks.
2. **Do not record rejected attempts.** The recorder hooks `QueueMainPhaseAction`, which fires
   before legality is decided. Deferring the row until resolution conflicts with the recorder's
   streaming design (it cannot enrich a flushed line — this is why `action_detail` is a separate
   correlated row), so this is more invasive than it sounds.
3. **Fix DCGO to honour 7-1-2-3** and return the card to its original index. Correct per the
   rules, and it would make recordings replayable without special handling — but it changes DCGO
   gameplay behaviour, which deserves a deliberate decision since DCGO is our oracle.
4. **Replicate the reorder in the replay.** Rejected: it would bake a rules violation into our
   engine's replay path purely to match a bug.

## Note on oracle trust

This is the second candidate case where DCGO appears to be the incorrect side (the first is
`FINDING-barrier-on-security-battle.md`). Triage still has no verdict axis for "divergence
confirmed, and DCGO is wrong" — every divergence implicitly accuses our engine. With two
candidates now, that gap is worth closing.
