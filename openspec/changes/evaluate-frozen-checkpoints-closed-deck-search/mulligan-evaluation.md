# Planned mulligan evaluation

Added at the user's request on 2026-10-29. Status: planned, not launched.

## Questions

Do CP8, CP12, CP16 and CP20 differ in raw keep/redraw preferences? Does search improve or change those preferences? How sensitive are decisions to the number of sampled hidden worlds and search seeds? Do selected disagreements change eventual outcomes when subsequent play is held fixed?

The current frozen campaign uses 320 total simulations split across four worlds (80 per world). A world's redraw branch has a fixed sampled replacement, so repeated visits are not independent replacement-hand draws. More worlds trade tree depth for broader uncertainty coverage; an observed benefit cannot automatically be attributed to either factor alone.

## Matched root diagnostic

| Configuration | Total simulations | Worlds | Simulations per world |
|---|---:|---:|---:|
| Policy-only | 0 | N/A | N/A |
| Current search reference | 320 | 4 | 80 |
| Broader sampling | 320 | 8 | 40 |
| Broader sampling | 320 | 16 | 20 |

- Test all four frozen models. Use zero root noise and the same model-routing, legal action dispatch and observation contracts as the reference harness.
- Start with the known-decklist regime to isolate mulligan sampling. Treat a later closed-deck replication as a separate information-regime comparison on a qualified matched runtime.
- Freeze an archetype/list/seat/opener-balanced cohort from the primary and confirmation recordings. Remove duplicate copies of the same root shared across matchup panels. Record the exact count before execution.
- Reconstruct each complete pre-decision state, including the other seat's already completed mulligan when applicable. Verify hand, perspective, legal mask and replay prefix. Swap models at that identical root; do not substitute a newly dealt hand.
- Repeat search over a fixed set of search seeds, paired across models/configurations. Keep world-generation prefixes aligned where the implementation permits. Freeze repeat counts after a small excluded runtime pilot.
- Keep expert-picked and model-disagreement examples separate from the representative cohort; selection on surprising behavior must not determine population rates.

## Evidence and interpretation

Record checkpoint/runtime/data hashes, root identity, original hand, actual chosen action, keep/redraw policy probabilities, root plan visits and Q values, sampled replacement-hand diversity, relevant leaf-value summaries and wall time. Preserve bounded diagnostics and exact replay references.

Audit preservation of the actor's known hand, resampling of unknown cards, and separation of search RNG from the actual game's future RNG. Do not reveal actual future draws to the decision procedure. Known-deck versus closed-deck access must remain explicit.

Report matched policy/search disagreement and stability across search seeds. Analyze level-3 presence, distinct-copy level-3 plus Training/draw/search, and Hybrid Tamer presence separately by archetype. Validate egg compatibility and access-card requirements before calling a sequence playable. Heuristic agreement is a review signal, not a correctness label. More worlds and higher Q are not expert ground truth; model values are not assumed calibrated.

Original and final-hand composition are useful descriptive outcomes, but replacement randomness must be labelled. Repeated seeds, shared hands and paired seats are correlated; use the source block/root as the resampling unit and never count each repeat as an independent game. Report uncertainty and distinguish representative rates from the selected review set.

## Controlled outcome follow-up

Preregister a subset and a finite continuation budget after the root diagnostic. Fork each selected state into keep and redraw, fixing the subsequent actor pilot, opponent, information regime and search budget. Use many paired independent continuation seeds, separated from the seeds used to choose the action. Preserve already revealed information and respect mulligan order. Couple randomness only where valid after the two branches diverge.

This estimates the consequence of the opening decision under those fixed continuation policies. It does not establish universally optimal mulligans or human strength. Report win/draw score differences with block-aware uncertainty and all failures; never retry rejected games or score incomplete games as completed outcomes.

## Scheduling and boundaries

This is an added evaluation workstream, not a change to the running campaign or stages A-D totals. Profile first, then freeze cohort sizes, repeats, runtime and resource allocation. Use available local/Hetzner capacity without interrupting active work. Adding this plan does not launch jobs, change training, replace frozen artifacts or promote models.
