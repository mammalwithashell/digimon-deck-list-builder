## Context

`train_specialist_league.py` resolves each round's warm-start from the **registry
champion** for the deck (`init_from = registry[deck]`). The registry is also what
the promotion gate writes (keep-best). So the gate and the warm-start share one
source, which means a non-promoting deck — whose registry entry stays the
generalist — re-rolls from the generalist every round and discards the round's
trained weights. The fix separates the two: the gate keeps owning the
pool/registry; the warm-start follows the deck's own training chain.

Current (coupled):
```
  round k:  init_from = registry[deck]          # generalist for any kept deck
  gate:     if r_k beats registry[deck] >=0.55 -> registry[deck] = r_k
```
Target (decoupled):
```
  round k:  init_from = deck's own r_{k-1} ckpt  (round 1: generalist)   # ACCUMULATE
  pool:     opponents = gated registry champions (best-known per deck)    # unchanged
  gate:     if r_k beats registry[deck] >=0.55 -> registry[deck] = r_k    # unchanged
```

## Goals / Non-Goals

**Goals:**
- A deck's specialist compounds its experience across rounds (round k continues
  round k-1), independent of whether it promoted.
- The promotion gate, registry, opponent pool, PFSP, and matchup matrix behave
  exactly as today (the pool stays the gated best-known champions).
- A `--warmstart {accumulate,champion}` toggle (default `accumulate`) reproduces
  the legacy behavior for A/B comparison.
- Non-promoting decks' accumulated checkpoints are retained for post-hoc anchored
  eval (so we can finally answer "does deep specialization help Gaia Red?").

**Non-Goals:**
- Changing the gate threshold (0.55) or the gate's head-to-head methodology.
- Changing the per-deck *deliverable* selection — the registry champion (proven
  ≥0.55) remains the shipped per-deck model; this change only stops discarding the
  accumulated specialist.
- Any engine / PyO3 / card-script / tensor-action change.
- Lowering the gate so weak decks auto-promote (a separate, optional experiment).

## Decisions

1. **Warm-start source = the deck's own latest round checkpoint.** Round 1 →
   the generalist seed (no prior round). Round k>1 → the final checkpoint of
   `<save-dir>/<deck>/r{k-1}` (the highest `step_*.zip`, or a `final.zip` if the
   per-round run writes one). Resolve from disk, not from the registry.

2. **The opponent pool stays the gated champions.** The pool/registry emission
   path is untouched, so a specialist still trains against the *best-known*
   (gated) version of every other deck — accumulating its own warm-start does NOT
   leak its in-progress weights into anyone's opponent set. This preserves the
   "frozen, monotone pool" invariant.

3. **`--warmstart {accumulate,champion}` flag, default `accumulate`.**
   `champion` = today's behavior (init from `registry[deck]`); `accumulate` =
   the new default (init from own `r{k-1}`). One branch in the warm-start
   resolver; threaded through `LeagueSpec` like the other CLI knobs.

4. **Retain accumulated checkpoints.** `keep_last_per_deck` retention must not
   delete a deck's round chain such that `r{k-1}`'s final is gone before round k
   reads it; retention is applied per-deck across rounds without orphaning the
   warm-start link. Non-promoting decks' final accumulated checkpoint is kept on
   disk for post-hoc `anchored_eval_cli` runs.

5. **Missing-prior-checkpoint fallback.** If round k>1 can't find `r{k-1}`'s
   checkpoint (e.g. a `--from-round` resume, or retention pruned it), fall back to
   the registry champion (legacy behavior) with a clear warning rather than
   aborting — accumulation is best-effort, correctness is not compromised.

6. **Docstring + comments fixed** to describe accumulate-from-own-checkpoint as
   the actual behavior (the current "warm-started from its round-(k-1) checkpoint"
   wording becomes true).

## Risks / Trade-offs

- **Drift / overfitting with no reset.** The accidental "reset to generalist" was
  acting as a regularizer. Mitigation: the generalist remains in every deck's
  opponent pool (anchors broad competence), LR stays low/decayable, and the
  in-training anchored panel surfaces a collapsing deck within 1–2 panels.
- **Catastrophic forgetting of general play.** Same mitigation — pool diversity +
  anchored monitoring; and the gated *deliverable* is still the proven champion,
  so a drifted accumulated model never ships by default.
- **Deliverable ambiguity.** An accumulated specialist can end up *better than the
  generalist yet below the 0.55 gate* (e.g. 0.52). Today it'd be discarded; now
  it's retained but still not the default deliverable. Surfacing it (post-hoc
  anchored eval) is in-scope; auto-shipping sub-gate models is not.
- **Disk.** Retaining per-round chains for all 6 decks grows `models/specialists/`.
  Bounded by `keep_last_per_deck`; the round-chain link just must be preserved.
- **A/B integrity.** Default flips to `accumulate`; anyone reproducing the old run
  must pass `--warmstart champion`. Documented in the runbook + the flag help.
