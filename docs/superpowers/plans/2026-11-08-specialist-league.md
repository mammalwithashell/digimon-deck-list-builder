# Specialist league: one search model per archetype (planned 2026-11-08)

Status: **built and run (2026-11-09).** This file records the original design. What was actually
built, including the promotion gate (added after gen 2 regressed), remote gate helpers, commands
and measured costs, is in `docs/runbooks/specialist-league.md`.

## Goal

Train one search (PUCT/PIMC) model per archetype. Each specialist pilots only
its own archetype's lists and trains against the frozen champions of every
other archetype, plus CP8 as a fixed anchor. The products are:

1. per-archetype pilots (desktop AI picks the specialist for the chosen deck);
2. a search-level archetype x archetype win matrix (deckbuilding signal);
3. a league whose opponents are frozen per generation, so targets do not drift.

Decision (user, 2026-11-08): **go wide, no pilot study**: about 20 archetypes,
3-4 gate-passing lists each.

## Roster

- The 13 current archetypes, plus up to 7 the user names.
- 3-4 lists per archetype. Every card in a list must be admissible under the
  training gate (CLAUDE.md rule 34; `data/card_admissibility.json`). The check
  is scratchpad `gate_lists.py` (to be moved into `code/tools/` when built).
- Sources, in order: `data/deck_library.json` (aliases via
  `data/archetype_aliases.json`), then the DigiLab API (regionals plus parsed
  decklists) for archetypes the library lacks under the pool's name. Light
  Hybrid, SW and ShineGreymon Data Squad are in this second group.
- List choice within an archetype: prefer distinct builds (different
  tech/engine choices) over near-duplicates, weighted by recency and
  placement.

Snapshot (2026-11-08) of gate-passing library lists for the current pool:
Chronicle 18, Chronomon 33, Examon 47, Glowing Dawn 161, Medusamon 141,
TS Jupitermon 168, TS Mervamon 24, Three Musketeers 109, Toho Braves 142,
Virus Busters 19. Light Hybrid, SW and ShineGreymon Data Squad are missing
from the library under those names. Strong candidates outside the pool:
Styracomon 54, Saiyu Warriors 51, Vemmon 52, Data Squad 48, Rocks 81,
Titans 43, DNA Omnimon 44, TS Bears 29, BeatBreak 17.

## Initialization

- **Warm start every specialist from the best generalist** (CP8, or the fresh
  run's gen 10/12 if the no-mirror CP8 checks and diagnostics say it is
  stronger). From-scratch specialists would split each generation's data
  ~20 ways, and a data-starved search value head memorises (CP8's value MSE:
  0.27 on its own training states, 0.82 on new ones).
- Keep the tanh value head and its holdout/best-epoch machinery. If the base
  is CP8 (identity value head), run the critic-only tanh refit first
  (`azero_trainer --calibrate-value-only`), or keep identity for every
  specialist. Decide this once, for all specialists.
- Archetypes new to the pool start from the same base: it already knows the
  game, and their first generation is their seed round.

## Generation loop (synchronous)

For generation g, with champions C(g-1) frozen for every archetype:

1. **Self-play per specialist A.** About 600 games. The learner always pilots
   one of A's lists. Opponent mixture:
   - ~80% other archetypes' champions C(g-1), each piloting its own lists,
     weighted toward the matchups A loses (win rate from the previous
     generation's games; floor so every archetype appears);
   - ~20% CP8 piloting a random list from the original 13 (excluding A's
     archetype);
   - no mirrors (A never faces its own archetype).
   - Gen 1: no specialists exist yet, so the base generalist pilots every
     opponent deck.
2. **Train A** on its replay window (window 3; consider 4-5 because each model
   sees less new data). Whole-game holdout 0.2. Epoch selection: report both
   the combined loss and value-only MSE, and pick on value MSE if it keeps
   rising against the incoming model (seen at fresh-run gens 5-6).
3. **Gate A.** New A vs old A, each piloting A's lists against the *same*
   frozen field on the same seeds and deck pairs (a paired comparison, no
   mirrors). Promote when the paired score difference is positive at a
   pre-set threshold; otherwise C(g) = C(g-1) for A.
4. After all gates, freeze C(g) and start g+1.

Plan for 3-4 generations, then re-assess.

## Evaluation

- Archetype x archetype matrix at search 320/4 between current champions,
  both seats and both opening orders, **no mirrors**. Both `make_schedule`
  modes are mirror-aware; the matrix needs its own scheduler.
- Generalist comparison: specialist A vs the base generalist piloting A, each
  against the same field. This answers whether specialization helps; the
  user skipped the pilot study, so this is where the answer comes from.
- CP8 anchor score per specialist on a fixed schedule, comparable across
  generations.

## Compute (estimate)

20 specialists x ~600 games is ~12,000 games per generation. The fresh run
does ~1,300 games/hour on one 8-vCPU ccx33 at search 320/4 with playout-cap
randomization, so one box needs ~9 hours per generation plus training and
gates. Use 3 boxes (~3 hours each), or one 48-vCPU box.

## Build list (nothing exists yet)

- League driver: per-specialist run folders, synchronous generations,
  champion registry per archetype, hash-bound plans (reuse
  `run_small_generations` patterns, including `bind_prepared_run`).
- Learner-deck restriction in self-play: the learner pilots only A's lists
  while opponents pilot their own (check `run_selfplay_generation` /
  `search_opponents` for an existing per-seat deck filter before adding one).
- Opponent entries that carry their own deck set (champion X pilots X's lists).
- Paired gate and matrix scheduler on the `local_checkpoint_search` harness.
- Roster builder: gate check plus list selection, emitting the frozen
  snapshot.

## Open questions

- Value head for a CP8 base: tanh refit or identity?
- Promotion threshold and gate size per specialist (paired games are cheap:
  ~100-200).
- Mixture weights: by loss rate, meta share, or both?
