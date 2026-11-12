# Specialist league: what to change for the next run

Findings from the first specialist league (`specialist-league-20261108`,
18 archetypes, base = fresh-value gen12) and the gen 5 matchup matrix
(2026-11-10), with the changes they suggest. The aim of the next run is pilots
that are clearly better at their own deck, not just lucky gate passes.

## 1. What the first league showed

**Training produces regressions, and the gate mostly rejects them.**

- In the trainer's held-out split (whole games), 87 of 88 models had their best
  epoch at epoch 0, and 68 of 88 ended worse than their starting weights.
  Training value loss falls while held-out value error rises: the value head
  memorises the outcomes of ~600 self-play games.
- The trainer restores the best epoch but never compares it with the untrained
  weights, so a generation always ships a change, even a harmful one.
- Many gen 6 rejections were far outside gate noise (about +/-0.05 on the
  paired mean): DATA SQUAD Ravemon 0.18 vs 0.46, Light Hybrid 0.14 vs 0.31,
  Venmon 0.02 vs 0.16.

**The gate is lenient, not harsh.** "Paired mean > 0" promotes a challenger
that is really no better about half the time. Over 6+ generations that admits
noise.

**Gen 5 matchup matrix** (`specialist_matrix.py`, 1,224 games per side, same
seeds and lists, champions vs base on both sides). The clean pilot measure is
each champion against opponents that are still piloted by the base model
(n=56 games each, roughly +/-0.07):

| Champion | vs base-piloted field | same games, base pilot | Lift |
|---|---|---|---|
| TS Gear Forest (gen 5) | 0.79 | 0.64 | +0.14 |
| Toho Braves (gen 5) | 0.86 | 0.71 | +0.14 |
| Dynasmon (gen 5) | 0.73 | 0.66 | +0.07 |
| ADVENTURE (gen 4) | 0.50 | 0.45 | +0.05 |
| TS Mervamon (gen 4) | 0.91 | 0.88 | +0.04 |
| Chronomon (gen 3) | 0.71 | 0.70 | +0.02 |
| SW (gen 5) | 0.70 | 0.70 | 0.00 |
| Medusamon (gen 5) | 0.46 | 0.48 | -0.02 |
| ShineGreymon Data Squad (gen 5) | 0.55 | 0.57 | -0.02 |
| TS Jupitermon (gen 2) | 0.70 | 0.73 | -0.04 |
| Three Musketeers (gen 3) | 0.48 | 0.70 | **-0.21** |

- Lift against the whole field is zero-sum and confounded: an archetype still
  on the base model "loses" whenever its opponents improve. Measure pilots
  against base-piloted opponents (above), or run a champion-vs-base-field
  matrix directly.
- Only two pilots improved clearly. Three Musketeers' promoted champion is
  clearly worse than base.
- **The ranking reflects pilot skill as much as deck strength, and it is not the
  real meta.** Decks that win with straightforward proactive turns rank high.
  Decks whose strength is a specific line rank low, because the agent does not
  find the line:
  - **Three Musketeers:** BeelStarmon (BT25-085, the dual card) uses a
    [Three Musketeers] Option for free on digivolve/attack. The key target is
    Bind Red Trigger (P-180), which trashes a security card and goes under a
    3M Digimon. When an effect trashes it from the sources (BeelStarmon's
    unsuspend, LadyDevimon BT25-083) it deletes a 7000 DP or lower Digimon.
    All 5 league 3M lists run both cards.
  - **DNA decks in this cohort: Examon and Virus Busters.** Both are still on
    the base model after 6 generations.
- Some top results may also come from engine/card bugs. TS Mervamon's 0.84
  against the field has not been audited.

## 2. Changes to training (the update step)

1. **Never ship a regression.** Evaluate the starting weights on the held-out
   set as "epoch 0" and keep them unless a trained epoch beats them.
2. **Fewer passes, gentler steps.** 1-2 epochs or a lower learning rate. With
   ~600 games per generation, 4 epochs overfits from the first epoch.
3. **More games per value target.** The value head needs far more outcomes
   than 600 games x 3-generation window. Options: more self-play games per
   generation, a longer replay window, or mixing in the shared base's data.
4. **Regularise the value head toward its starting point** (smaller value
   loss weight, or an anchor/KL term to the base value) so a specialist
   refines the base's judgement, not replaces it.
5. **Track value calibration per generation** (held-out value MSE against the
   starting weights) and stop a specialist that drifts for two generations.

## 3. Changes to search targets (the signal)

1. **First check that search teaches anything.** For each archetype, play
   search 320/4 against the raw policy. If search barely beats the policy, the
   policy targets carry almost no signal and more generations will not help
   that deck.
2. **More search where lines are long.** 320 simulations cover less than one
   turn, because every choice inside an effect is its own decision node.
   Combo decks (3M, Examon, Virus Busters) need a larger budget or a
   per-archetype budget.
3. **Seed hard decks from expert play.** Behaviour cloning from DigiLab lists
   and the YouTube expert games (see memory `project_youtube_expert_play_seeds`)
   gives the policy the combo lines that search cannot find from scratch.
4. **Combo-aware opening curriculum.** Start some self-play games from
   positions where the combo is available (e.g. BeelStarmon in hand, Bind Red
   Trigger reachable) so the value head sees what the line is worth.

## 4. Changes to the gate (what counts as better)

1. **Require a margin.** Promote when the paired mean beats 0 by about two
   standard errors, or confirm a pass on a second, independent seed set.
2. **Weight matchups by importance.** The game is very matchup dependent. Score
   the gate per opponent archetype and weight it by that archetype's share of
   the field (or the real meta), so a specialist is not promoted for gaining in
   a matchup that never happens while losing a common one.
3. **Report per-matchup results, not only the mean.** Keep each specialist's
   best and worst matchups visible, and refuse a promotion that loses heavily
   in an important matchup even when the mean is positive.
4. **Gate against base-piloted opponents too.** A field-wide check against
   base pilots (the clean measure above) catches promotions that only
   reshuffle results among improving opponents.

## 5. Measurements to add

- **Key-play counters per archetype**, recorded in game results: 3M = free Bind
  Red Trigger via BeelStarmon and its delete on trash; Examon / Virus Busters =
  DNA digivolutions; similar signatures for the other decks. A pilot that never
  makes its key play cannot be "mastering" the deck, whatever its win rate.
- **Matchup matrix every generation** (`specialist_matrix.py`); the base
  matrix depends only on decks and seeds, so it is run once and reused.
- **Watch a few games** of the weakest specialists each generation before
  trusting the numbers.
- **Audit top performers for bugs** (start with TS Mervamon) before reading
  their results as deck strength.
- **Run the deck pool through the training gate** (`data/card_admissibility.json`)
  before the league starts. The first league did not: ADVENTURE's lists hold
  about 20 cards with no card spec that played as blank bodies. The deck audit
  (`qa/archetype-qa/league-deck-audit-2026-11-10.md`) lists the engine bugs
  found in all 18 decks; several weak decks (ShineGreymon Data Squad, Venmon,
  Virus Busters, Three Musketeers, Glowing Dawn) were weakened by them.

## 6. Related

- League procedure: `docs/runbooks/specialist-league.md`
- Matrix tool: `code/tools/cloud_search/specialist_matrix.py`
- Gen 5 matrix data (local): `D:/ex13-data/specialist-league/matrix/{gen5,base}/`
