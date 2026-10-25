# DNA-capable deck cohort correction, 2026-10-25

The mixed-pool denominator in the earlier evolution discussion was too broad for
interpreting DNA frequency. Filter by the actual DNA targets in each decklist,
then attribute each decision to the acting player's list. Do not include a
non-DNA opponent's decisions merely because it played against a DNA deck.

This follow-up covers the same completed generations 1 and 2 and frozen models
0, 1 and 2. It does not change training, rewards, decks or live model files.

## Card-based cohort

The frozen pool has 24 slots representing 20 distinct lists. Ten slots, nine
distinct lists, across six archetypes contain a printed DNA target:

| Archetype | Eligible slots | DNA targets and copies per eligible list |
|---|---:|---|
| Chronicle | 2/2 | BT20-060 Alphamon: Ouryuken x3 in each |
| Chronomon | 1/2 | BT24-037 Silphymon x2 in slot 1 |
| Examon | 2/2 | BT23-047 Examon x3 and EX13-045 Examon x3 in each |
| TS Mervamon | 1/2 | BT24-037 Silphymon x2 in slot 0 |
| Toho Braves | 2/2 | BT8-084 Kimeramon x1 in slot 0; BT16-077 Dinobeemon x1 in slot 1 |
| Virus Busters | 2/2, duplicated list | EX12-032 WereGarurumon x4, EX12-035 MetalGarurumon x4, EX12-037 Omnimon x3, EX12-044 Angewomon x3 |

The ten unique target IDs were checked against card images, loaded card data and
YAML DNA paths. Every DNA path target in this pool also has a nonempty `dna_costs`
record. The audit therefore uses that field for this frozen pool, with an
assertion that no excluded list produces a legal Main-phase DNA action.

ST24-15 **DNA Charge** does not enable DNA digivolution; its name does not qualify
ShineGreymon Data Squad. BT25-028 Dianamon's effect names GraceNovamon, but none
of the frozen lists contains a GraceNovamon target. An isolated reference to DNA
does not establish an executable route in a list. The remaining no-target slots
are excluded, including the non-Silphymon Chronomon and TS Mervamon variants.

## Corrected generation-2 denominators

One appearance means one deck occupying one seat in one game. Both seats count
when both decks are eligible; appearances are not independent game results.

| DNA-capable archetype | Seat appearances | Appearances with recorded DNA opportunity | DNA-legal decision points | Search prefers DNA | Checkpoint 2 prefers DNA |
|---|---:|---:|---:|---:|---:|
| Chronicle | 141 | 2 | 2 | 0 | 0 |
| Chronomon, eligible list only | 84 | 1 | 1 | 0 | 0 |
| Examon | 152 | 41 | 170 | 8 | 0 |
| TS Mervamon, eligible list only | 98 | 6 | 30 | 2 | 0 |
| Toho Braves | 147 | 2 | 9 | 0 | 0 |
| Virus Busters | 173 | 64 | 233 | 17 | 0 |
| **Total** | **795** | **116** | **445** | **27** | **0** |

The eligible seats occur in 630 of the 1,000 games. Their recorded positions
total 31,495, including 11,071 Main-phase decisions. Thus:

- 116/795 appearances (14.59%) have a recorded Main-phase DNA opportunity.
- 445/11,071 recorded Main-phase decisions (4.02%) offer DNA.
- Search ranks DNA first in 27/445 DNA-legal decisions (6.07%), occurring in 24
  distinct seat appearances. These are visit-argmax preferences, not resolved
  DNA events or the actual temperature-sampled action counts.
- Checkpoint 2 ranks DNA first in 0/445 of the same legal positions, including
  0/27 search-positive positions. The earlier imitation finding survives the
  cohort correction; the claim of low usage relative to all decks does not.

Generation 1 provides a comparison: 837 eligible appearances, 100 with recorded
DNA opportunities, 335 DNA-legal decisions and 11 search-positive positions.
All eleven search-positive positions were Examon. Generation 2's positives
extend to Virus Busters and TS Mervamon. Raw counts alone do not establish a
strength improvement or a desirable DNA usage rate.

## Interpretation and boundaries

DNA coverage should be followed separately for each capable list and card pool.
Examon and Virus Busters account for 403/445 generation-2 legal opportunities;
an aggregate DNA metric mostly describes those decks. Chronomon has only one
recorded opportunity, so its zero selections are weak evidence about policy
quality. Capability does not imply that a board assembled the materials, the
target was in hand, or DNA was strategically desirable.

This audit counts only normal Main-phase actions 63..92, with phase checking.
It does **not** count Blast DNA, end-of-turn DNA or DNA inside an effect-selection
sequence. Chronicle's Alphamon: Ouryuken also has Blast DNA; its two normal-Main
opportunities must not be interpreted as its total DNA opportunities. Those
other routes require their own opportunity and resolved-event analysis.
Forced actions are not stored in the training shards and are outside these
denominators. No actual selected-action or successful-resolution count is
reconstructed from visit distributions.

One card-data inconsistency was found during the image check: **EX12-032
WereGarurumon** prints level-4 materials on both sides of its DNA requirement,
and its YAML agrees, but the card data loaded by the frozen image records level
5 on both sides. Local and frozen `cards.json` hashes matched at audit time.
The [subsequent reproduction and fix](2026-10-25-virus-busters-dna-data.md) shows
that runtime DSL enrichment preserved the legal level-4 route while also
retaining the illegal level-5 route. This did not block legal level-4 DNA. The
repository data is now corrected; the live image and historical audit remain
unchanged. Treat those opportunity counts as frozen simulator behavior, not
proof of correct printed-card coverage. Eleven of generation 2's teacher-positive
DNA decisions target EX12-032 and need material-route verification before being
used as evidence of a policy-quality failure.

The appropriate follow-up is a route-specific analysis within these capable
lists, plus a focused EX12-032 data/behavior check. Increasing DNA preference
across the whole twelve-archetype pool would not be justified by these results.

## Evidence and reproduction

- [Machine-readable counts, pool membership and model statistics](2026-10-25-dna-deck-cohorts.json).
- Pool SHA-256: `5860cf948cd49d9ab4e1ac23b271dfffbccbac69d78288ed4123af1ef410b06a`.
- Audit uses the same pinned Linux image as the prior study, read-only experiment
  mount, network disabled, 0.5 CPU, low CPU shares and 4 GiB memory. All chunk
  hashes and termination flags checked; no legal Main DNA actions came from
  excluded lists. All 335/445 opportunities were evaluated, without sampling.
- Raw per-position references:
  `/opt/digimon-search/dna-cohort-audit-20261025/result.json`, also copied locally
  to `runs/evolution-research-20261025/dna-cohorts.json`.
- Reproduce: `python code/tools/cloud_search/audit_dna_deck_cohorts.py --root /experiment --cards /app/data/cards.json --out result.json`.
- Validation: four tests passed across `test_dna_deck_cohorts.py` and
  `test_evolution_imitation_audit.py`, including same-archetype list exclusion,
  name-only false positives, mover/seat attribution and global game offsets.
- Research container exited zero. No model fitting or production mutation ran.
