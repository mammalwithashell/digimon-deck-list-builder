# Evolution learning: same-position audit, 2026-10-25

The frozen policy is learning much less evolution behavior than search supplies.
Search finds the unique winning move in the three immediate tactical fixtures;
the network alone misses battle evolution and DNA. This supports investigating
distillation before adding an evolution reward. It does not establish that search
is generally strong or that two training epochs are the sole cause.

## Scope and provenance

- Campaign: `english-meta-pets-4x1000-20261025`, twelve archetypes, 24 list entries.
- Completed generation 1: 1,000 games, 76,042 non-forced training positions.
- Completed generation 2: 1,000 games, 82,020 positions. Both sources are clean:
  committed chunk hashes verified, no truncated games, all row termination flags true.
- Compared frozen models 0, 1, 2 on identical stored observations and full legal
  masks. Generation 0 is the parent before these two search-training updates.
- The audit ran in the campaign's pinned Linux image, against a read-only mount,
  with 0.5 CPU, low CPU shares, 4 GiB memory and no network. It exited successfully.
  The production training and monitor containers remain running; rewards, models,
  generation settings and production files were not changed.
- Schema: `standard_lite_v2.3`, action schema 1. Only Main-phase normal raising,
  battle and DNA actions, and Breeding-phase promotion, are counted. Effect-driven,
  counter and end-of-turn evolution routes are outside this decision-level audit.
- Source hashes, image digest, model hashes, all aggregate results and 96 tactical
  trials are in [the evidence JSON](2026-10-25-evolution-imitation.json).

The current objective is masked cross-entropy against search visit distributions
plus terminal-outcome value MSE (coefficient 1). The campaign uses two epochs per
update, batch 512, learning rate 1e-4 and up to three generations of replay.
Value targets are win +1, loss -1, draw 0. There is no evolution bonus, length
penalty or discounted return in this search-training objective. PPO environment
dense rewards are not the targets used here.

## Search supplies evolution targets

These are all stored generation-2 decisions, not counts of resolved game events.
An opportunity is a non-forced recorded position with at least one legal action
for that route. A teacher preference is the highest-visit action, breaking ties
in action-ID order. Route probability sums visits over all actions in that route.

| Route | Opportunities | Teacher-preferred positions | Share of opportunities | Mean target route probability |
|---|---:|---:|---:|---:|
| Raising evolution | 15,204 | 1,581 | 10.40% | 11.72% |
| Battle evolution | 13,503 | 1,026 | 7.60% | 10.00% |
| Promotion | 2,391 | 849 | 35.51% | 41.34% |
| DNA | 445 | 27 | 6.07% | 4.99% |

Generation 1 had 534 battle-preferred positions out of 12,793 opportunities and
11 DNA-preferred positions out of 335. Generation 2 improves those counts. The
absolute number of DNA-positive examples is small, but the whole mixed-deck
dataset is not an appropriate denominator for DNA use: many lists lack DNA
targets. The [DNA-capable deck follow-up](2026-10-25-dna-deck-cohorts.md) identifies
10 eligible slots and narrows generation 2 to 795 deck appearances, 31,495
recorded positions and 445 legal Main-phase DNA opportunities. Search prefers
DNA at 27 of those opportunities; checkpoint 2 at none. This establishes an
imitation gap on those states, not that more DNA would necessarily win games.

## What the network retains on those exact positions

For each route, take a fixed-seed reservoir of up to 512 positions where the
teacher preferred that route. These are deliberately selected positive cohorts,
not estimates of overall action frequency. All three models see the same rows.

| Route | Positive cohort size | Parent chooses route | Checkpoint 1 chooses route | Checkpoint 2 chooses route |
|---|---:|---:|---:|---:|
| Raising evolution | 512 | 15 | 15 | 15 |
| Battle evolution | 512 | 0 | 0 | 1 |
| Promotion | 512 | 336 | 261 | 235 |
| DNA | 27 (all) | 0 | 0 | 0 |

The battle-evolution probability does increase: 7.70% in the parent, 9.76% in
checkpoint 1, 12.75% in checkpoint 2. The teacher's mean is 45.46%. Raising moves
from 10.06% to 16.14%, against the teacher's 43.16%. DNA moves from 6.29% to 7.87%,
against 31.45%. Thus there is some learning, but it rarely flips deterministic
top-choice behavior. A search preference need not carry more than 50% probability.

Promotion is different: teacher-positive mean probability falls from 55.06% in
the parent to 47.29% in checkpoint 2, against the teacher's 67.68%. On a separate
uniform opportunity sample, checkpoint 2's mean promotion probability is 41.59%,
almost matching the teacher's 41.90%. Matching the overall rate can conceal a
failure to select promotion in the right states. Cross-entropy improves on that
opportunity sample while worsening on the promotion-positive sample.

These findings cannot be explained solely by the evaluated models reaching
different positions: observations and legal masks are identical in this audit.
They are in-sample imitation diagnostics, not held-out generalization or strength
measurements. Rows from one game are correlated. Generation-1 and generation-2
cohorts are separate; they should not be treated as independent match results.

## Tactical checks

The native harness uses four abstract engine fixtures and exact bounded minimax
to establish root-action outcomes. All root actions received proven outcomes;
unknown outcomes are never counted as draws. The first three each have one
winning action, with every alternative losing.

| Fixture | Parent / checkpoint 1 / checkpoint 2 direct policy | Search at 320/4 | Search at 1280/4 |
|---|---|---|---|
| Battle-evolve into Rush for lethal | All miss | All find win | All find win |
| DNA fresh materials for lethal | All miss | All find win | All find win |
| Promote for lethal | All find win | All find win | All find win |
| Raising evolution or hard-play, then win next turn | All hard-play; both choices win | All hard-play; win | All hard-play; win |

Each budget has four evaluator settings (three frozen checkpoints plus uniform
policy/zero value), three seeds, four worlds and batch four. Search chooses the
unique winning action in all 72 immediate-tactic trials, and a proven winning
action in all 24 raising-control trials. Seeds have identical results here;
these repeated simple fixtures do not provide 96 independent examples of skill.

The raising control is deliberately graded against both proven winning actions.
It cannot support a claim that preferring hard-play is an error. A meaningful
raising-development benchmark still needs positions where raising uniquely
improves the result, including longer defensive and resource-management lines.

Synthetic cards have unknown production embeddings. These fixtures test search
and model action selection, not real-card fidelity or representative meta play.
The local native engine is commit `5c8ab7222`, while the live image is
`e1e7fff2c689e8505a8146d9012502be11dce8c1`; the abstract fixtures do not exercise
the intervening card fixes. This is not a bitwise replay of production games.
The audit uses SB3 policy inference; tactics use the exported ONNX checkpoints.
No new cross-runtime parity claim is made by this study.

Training targets contain exploration noise (epsilon 0.25), whereas tactical
search uses zero root noise. Stored argmax counts are not counts of actions
actually sampled during the early temperature window. Nor are they proof of
optimal play. Four-times-higher search budget gives no tactical action benefit
in this small set; it says nothing decisive about long-horizon meta decisions.

## Recommended next experiment

Keep the reward objective fixed. Run a controlled offline fit experiment before
spending more collection compute or adding a blanket digivolution incentive:

1. Start each variant from the same parent and replay data. Compare 2, 4, 8 and
   16 epochs. Split by complete games, not adjacent rows, for held-out checks.
2. Track route-conditioned cross-entropy, teacher probability, top-choice
   agreement, and promotion-positive retention alongside overall loss. Include
   ordinary play/attack decisions so gains cannot be bought by always evolving.
3. Add a policy-only diagnostic fit to test whether joint value fitting competes
   with imitation. Treat it as a diagnostic, not a deployable replacement for a
   search policy-value model. Test balanced route sampling separately if needed;
   changing sampling also changes the training objective unless corrected.
4. Evaluate the selected candidate with seat-balanced games and tactical cases,
   including real-card raising lines. Preserve current frozen opponents and
   report per-archetype results, truncations and promotion behavior.

Underfitting, shared-feature interference, limited representation and weak/rare
teacher signals remain possible contributors. This study distinguishes the
imitation problem from complete absence of evolution targets; it does not yet
identify which training change resolves it.

## Reproduction and checks

- `python code/tools/cloud_search/audit_evolution_imitation.py --root /experiment --out result.json --generations 1 2 --sample 512`
- `cargo run -p digimon-engine --example evolution_diagnostic -- data/cards.json MODEL_DIRECTORY OUTPUT.json`
  where `MODEL_DIRECTORY/gen{0,1,2}/model.onnx` holds the frozen exports. Output
  must not already exist. Use an isolated Cargo target and the documented Windows
  stack setting. The evidence JSON records model SHA-256 hashes.
- `python -m pytest code/tests/rl/test_evolution_imitation_audit.py -q`: 2 passed
  (phase-aware route decoding and reproducible bounded reservoir sampling).
- Native harness completed all 96 trials, with conservation assertions enabled
  and all root proof outcomes known. No engine production code was changed.

Raw audit references are retained locally at
`runs/evolution-research-20261025/imitation.json` and remotely at
`/opt/digimon-search/evolution-imitation-20261025-1/result.json`. The compact
committed evidence omits thousands of sample row references but retains all
cohort counts, model statistics, and tactical results.
