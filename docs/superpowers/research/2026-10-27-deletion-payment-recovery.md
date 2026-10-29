# Deletion-payment loop and fitted checkpoint recovery

The EX13 continuation saved checkpoint 1 after 1,100 games, 105,563 training
positions and zero truncations. Its combined diagnostic evaluation then stopped
after 6,273 of 7,800 games at seed `44524201`, Medusamon versus Chronicle. The
original pet panel completed, but the failed evaluation prevented generations
2–4 from starting. Preserve that campaign as failed evidence.

## Reproduction and rules

The recording is retained at
`code/digimon-engine/tests/fixtures/training/medusamon-immune-token-payment.json`.
BT20-053 Grademon granted the Chronicle player's Petrification Token immunity
to opponent Digimon effects. EX13-060 Alphamon applied -8000 DP to EX11-012
Medusamon twice, the second activation after playing BT20-012 Ginryumon.

At recorded row 93, Medusamon reached zero DP. Its optional leave replacement
offered the immune token as a deletion payment. The deletion correctly failed,
but the following unconditional cancellation still protected Medusamon. The
zero-DP rules check repeated the same choice indefinitely. Action 116 selected
that token in an `AnyField` selection; the old diagnostic mislabeled it ATTACK.

Comprehensive Rules 4.3 sections 15-7-1/2 require successful process conditions,
and 15-7-4 requires an executable optional payment. Section 15-15-5-3 still
permits selecting unaffected targets for ordinary effects. Medusamon's official
Q6514 explains repeated zero-DP saves through actual token deletion. The DCGO
EX11-012 implementation similarly checks the deletion outcome before protecting
the original permanent.

Apply the shared repair to adjacent scalar `DeletePermanent` /
`CancelReplacement` payment sequences. There are 14 matching YAML definitions;
this describes structural exposure, not 14 independently reproduced failures.
The active pool includes EX11-012 and BT24-018 Styracomon. Preserve lowest-DP
eligibility before filtering payment feasibility, and only cancel after the
exact selected permanent was deleted. Payer protection, redirection, passive
immunity and optional decline must leave the original replacement uncancelled.

## Recovery contract

Prepare a new root with `tools.cloud_search.prepare_payment_recovery`. Mount
the failed campaign read-only at `/archive`; mount an empty destination at
`/experiment`. Keep operator logs outside the empty destination during preparation.
The command requires exact source-plan and checkpoint artifact hashes and a new
immutable image digest. It revalidates native admission, model contracts and
eight full-budget health games before writing a usable campaign plan.

Retain fitted checkpoint 1 weights under `generation/gen1` and initialize
`next_gen=2`. Do not copy previous replay shards, progress or training summaries
into the new generation tree. Historical collection counts belong to lineage.
Keep the existing parent-checkpoint4 and development-fixed anchors unchanged.

`tools.cloud_search.resume_payment_recovery` evaluates checkpoint 1 again on the
repaired engine, then collects, fits and evaluates logical generations 2–4.
Each collection remains 1,100 games with eight workers, 320 simulations, four
worlds and batch four. Keep the three-generation replay window, two optimizer
epochs, batch 512 and learning rate 0.0001. The window grows from repaired-engine
data only. Tensor and action contracts are unchanged.

Keep the exact EX13 pool: 13 archetypes, 21 distinct training lists, 26 sampling
slots. Keep both evaluation panels and their 10,800 scheduled games per
checkpoint, original seeds, list identities, opening order and anchors. Report
completeness is validated against every scheduled deck/seed/anchor entry.
The controller fails closed; it neither adopts partial output nor automatically
restarts or promotes a model. W&B records inherited checkpoint lineage and the
three remaining generations separately.

Before launch, replay both the exact recorded payment boundary and the full
saved-policy game on the shipping image. CI image smoke includes the boundary
regression so a stale wheel cannot silently ship the original loop.

## Preserved provenance

- Source campaign: `ex13-repaired-4x1100-20261027`.
- Source plan SHA-256:
  `a1802ab194f26eafc1570138a7d28da9cdbb7172e7cad77841dfe6f97ba42e2f`.
- Checkpoint 1 ZIP SHA-256:
  `07caace73e5dd32a5970f046fb49d2450f0beaf9b20f80fe7c54fa431d23b240`.
- Checkpoint/evaluation/failure archive SHA-256:
  `c0d01d396dc20c81167682e32d03579144f80ce961ad171cff47f05d740c74b7`.

The archive was copied back locally and hash-verified. Old replay remains intact
in the original campaign. The new image, source commit, CI run, preparation
evidence and live outcome are recorded in the recovery's deployment artifacts.

## Medusamon current-DP follow-up

EX11-012's shared When Digivolving / End of Attack deletion clause also used a
fixed 12,000-DP threshold. Its printed text compares against this Digimon's DP;
CR 4.3 section 15-8-3-7 reads that value while processing. Both the availability
condition and target filter now use the existing `source_dp` formula, which
reads effective DP including bonuses and reductions. No engine, action or
observation contract changes are required.

Four regressions in the card's behavioral module cover increased and reduced
DP, equality, changed DP between triggering and processing, and the absence of
any qualifying target or trash follow-up. Both printed timings exercise real
pending choices and action masks, including cloned target selection. The
existing source-liveness check continues to prevent a departed source's queued
effect from activating; this patch does not add last-known-DP fallback logic.

This follow-up is a subsequent image revision. The payment-recovery experiment
retains its frozen image until an explicit rollout; editing the card definition
does not alter the running container or its evaluation baseline.
