# FINDING: our engine deletes a P0 permanent that DCGO keeps

**Status:** genuine divergence — survives every tooling explanation ruled out tonight. Root cause not yet identified.

**Why this one is credible.** Four artifact classes that produced false findings earlier are all excluded here:

- **memory drift** — recorded memory 7, engine memory 7 at the divergence. Agree.
- **phantom actions** — every play in this recording carries an `action_detail`; the affordability fix landed.
- **hand reconstruction** — verified consistent with `initial_state.initial_hand` plus draws.
- **the `-1` sentinel panic** — fixed; the replay now reaches this point instead of aborting.

## The divergence

```
recorded  step 15: digivolve_hand_0_to_field_0, card=EX12-035, cost_paid=3, mem_after=4
engine legal set : play_hand_0/2/3/4, pass    (no digivolve offered)

DCGO   board_p0: ['EX12-016']   held continuously from step 6 through step 15
engine board_p0: EMPTY,  EX12-016 sitting in trash
```

`EX12-035` MetalGarurumon has `[Digivolve] Lv.5 w/[Garurumon] in name or w/[ME]/[VB] trait: Cost 3`, and `EX12-016` MetalGreymon is Lv.5 with the `[ME]`/`[VB]` traits. `cost_paid=3` confirms DCGO used exactly that printed path. The play is legal and unremarkable — **our engine simply does not have the MetalGreymon to digivolve onto.**

So this is a **board-state** divergence, not an action-legality one. The action our engine refused is a symptom; the disagreement happened earlier.

## What is known about the deletion

- P0 played `EX12-016` at step 5 for `cost_paid=7` (both engines agree on that; memory moved 0 -> -7 on both sides).
- By the divergence, our engine has it in trash alongside `EX12-001` (the Digi-Egg) and `EX12-007`.
- DCGO's `board_p0` snapshots show `['EX12-016']` unbroken across steps 6-15, so DCGO never lost it.
- Step 14 is an attack by P0 (`id=114`) with `board_p0: ['EX12-007', 'EX12-016']` — two permanents at that moment. P1's board holds `EX12-021` (2000 DP), `EX12-066` (Tamer), `EX12-016` (7000 DP).

A plausible shape is that our engine resolved that attack's battle differently and deleted the attacker (and possibly more), but that is a hypothesis, not a conclusion — and hypotheses formed by staring at our own engine's state are exactly what produced three wrong findings earlier tonight.

## How to actually pin it

1. Step the recording in `digimon-engine-mcp` and find the FIRST step where `board_p0` diverges from the recording's snapshot. `board_p0` is recorded on every action row, so the divergence point is directly observable rather than inferable — do not reason backwards from the end state.
2. Once the step is known, read what happened there in both engines before forming a hypothesis about why.
3. `EX12-001` in trash is worth explaining too — a Digi-Egg reaching the trash is a specific event, not incidental.

## Note

`board_p0` in these recordings is the **battle area only** (DCGO commit `be359bb5b` changed it from every-frame to battle-area). The breeding area is not included, so absence from `board_p0` does not by itself mean a card left play.
