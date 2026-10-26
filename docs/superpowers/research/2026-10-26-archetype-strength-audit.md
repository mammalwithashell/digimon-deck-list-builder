# Frozen 13-archetype strength and fidelity audit

The audit found a confirmed shared End of Attack rules defect, an evaluation
pool with limited list diversity, and simulator rankings that do not closely
match observed EX13 tournament results. More training alone is not yet a
defensible explanation or remedy for the discrepancies.

The user directed the current cloud continuation to **keep running for diagnostic
data** after the defect was disclosed. No engine, deck, model, search budget or
training controller was changed. A diagnostic warning was added to the run's
W&B summary and `/opt/digimon-search/fixed-continuation-20261026/diagnostic-warning-20261026.json`.
No automatic promotion is enabled.

## Evidence and boundaries

- Frozen scope: 13 archetypes; 21 distinct training lists in 26 equally weighted
  sampling slots; one evaluation list per archetype; 280 unique train/eval cards.
- Eight meta archetypes have two training lists and one held-out list. Five
  pets have one list duplicated into two sampling slots and same-list evaluation.
- New baseline: 7,800 completed policy-only games on the Nyaromon-fixed image,
  including 2,600 games against identical frozen checkpoint-4 weights, 200 per
  candidate archetype, with balanced opening order. No incomplete games.
- This baseline is checkpoint 0 of the new campaign, with old checkpoint 4's
  exact weights `f33e15a451b385fd8dcb9961b9351fc54d8535a8509d4cb3fc41a83dd83a86df`.
- Image: `sha256:705f2c4b86c5458a2df7f09ded1e1d9cc49160f65bca442cd187b0b1480f0711`,
  source `6627540bab607109b9eb8d5216ff647b8e4b9ae8`.
- External evidence: primary public DigiLab EX13 profiles retrieved October 26,
  about 18:35 UTC. These cover global EX13 matches, not an established English-only
  event cohort. Win rates exclude draws. Different lists, pilots, opponents and
  event match formats prevent interpreting the percentage gap as causal.
- The previous 3,120-game same-model cross-deck matrix uses the older Nyaromon
  engine. It is retained as a separate population-weight sensitivity analysis,
  not pooled with the new baseline.

Detailed artifacts are under `runs/archetype-audit-20261026/`: `list-audit.json`,
`meta-evidence.json`, `meta-weight-sensitivity.json`, `behavior-audit.json`,
`mechanics-audit.json`, the frozen inputs, and an expandable `behavior-audit.html`.
The JSON files preserve source URLs, dates, records, card counts and denominators.

## All-archetype comparison

Simulator percentages below are the **new fixed-image baseline**, not a newer
trained model and not the earlier 480-appearance round robin. Each simulator
row has 200 candidate games against the identical model over the 13-deck pool.

| Archetype | Simulator | Global EX13 | External W-L-D | Evaluation list | Main audit priority |
|---|---:|---:|---|---|---|
| [Chronicle](https://digimon.digilab.cards/deck/chronicle?format=EX13) | 63.5% | 50.5% | 447-439-69 | 14/18 event finish | Overperformance; Grademon timing candidate; weak list-source sample |
| [Chronomon](https://digimon.digilab.cards/deck/chronomon?format=EX13) | 50.5% | 50.8% | 274-265-80 | 11/19 | Large list shift; Succession, recovery and protection choices |
| [Examon](https://digimon.digilab.cards/deck/examon?format=EX13) | 49.5% | 62.7% | 1481-880-169 | 3/11 | Joint DNA setup and battle-win payoff; more representative lists |
| [Glowing Dawn](https://digimon.digilab.cards/deck/glowing-dawn?format=EX13) | 42.5% | 49.6% | 581-590-99 | 6/13 | Confirmed ST23-04 attack-carrier timing bug; Arts/resource sequences |
| [Light Hybrid](https://digimon.digilab.cards/deck/light-hybrid?format=EX13) | 44.0% | 55.0% | 33-27-2 | Same pet list | Two timing candidates; multi-source setup; small tournament sample |
| [Medusamon](https://digimon.digilab.cards/deck/medusamon?format=EX13) | 49.0% | 49.6% | 184-187-21 | Same pet list | Medusamon timing candidate; token/payment tactics; agreement is not certification |
| [SW](https://digimon.digilab.cards/deck/saiyu-warriors?format=EX13) | 57.0% | 45.9% | 141-166-32 | Same pet list | Overperformance and sparse broad audit evidence; focused Erlangmon tests pass |
| [ShineGreymon Data Squad](https://digimon.digilab.cards/deck/data-squad-shinegreymon?format=EX13) | 42.5% | 47.3% | 43-48-4 | Same pet list | Tamer/resource/Arts chain and Burst conversion; small tournament sample |
| [TS Jupitermon](https://digimon.digilab.cards/deck/jupitermon?format=EX13) | 54.5% | 62.7% | 908-540-76 | 1/19 | Low-security evolution, protection, Succession and Counter choices |
| [TS Mervamon](https://digimon.digilab.cards/deck/ts-minervamon?format=EX13) | 54.0% | 64.0% | 714-401-107 | 7/48 | Largest held-out list shift; mixed-zone play-budget choices |
| [Three Musketeers](https://digimon.digilab.cards/deck/beelstarmon-3m?format=EX13) | 45.0% | 58.9% | 531-371-88 | 43/48 | Confirmed Ruin Mode timing bug; list quality and full Option/Arts chains |
| [Toho Braves](https://digimon.digilab.cards/deck/toho-braves?format=EX13) | 56.5% | 66.3% | 572-291-105 | 14/48 | Shishimamon timing candidate; death/replay and Retaliation trades |
| [Virus Busters](https://digimon.digilab.cards/deck/virus-busters?format=EX13) | 44.5% | 48.4% | 31-33-6 | Same pet list | Rare joint Nyaromon DNA opportunities; broader coverage and small real sample |

Aliases were checked: Three Musketeers is DigiLab **Beelstarmon (3M)**, SW is
**Saiyu Warriors**, Light Hybrid is DigiLab's **Light Hybrid variant within the
Koji Hybrid family**, and ShineGreymon is
**DATA SQUAD ShineGreymon**, not DATA SQUAD Ravemon.

Primary starting point: https://digimon.digilab.cards/meta?format=EX13.
Exact profile links and each independently retrieved table are in `meta-evidence.json`.

## Confirmed rules defect

The focused native tests execute real attacks through legal masks in both
seats, using production card data and embedded effects. Results: **10 tests,
7 passed, 3 failed, zero ignored**. Failures are preserved in
`runs/archetype-audit-20261026/mechanics-regressions.log`; executable reproductions
are `code/digimon-engine/tests/archetype_audit_regressions.rs`.

1. ST23-04 Murasamemon's inherited End of Attack wrongly spends a Tamer source
   and unsuspends its carrier after another ally attacks.
2. The same effect wrongly activates after an opponent attacks.
3. EX4-074 ShineGreymon: Ruin Mode wrongly deletes itself and Recovers 1 after
   another ally attacks. This directly affects the frozen Three Musketeers list.

Own-attack ST23-04 behavior passes. All six focused EX12-034 Erlangmon tests pass,
including hand/source free play, own/other leave, cloning, compaction, filters,
once-per-turn use and wrong-target negatives.

Printed images, canonical rules 4.3 section 15-16-15-1 (PDF page 35), and DCGO's
attacking-stack checks agree on the carrier restriction. EndOfAttack is currently
broadcast to every permanent, and printed DSL effects do not automatically get
that restriction. This is a shared timing issue, not uniquely an inherited effect.

Eight cards across six frozen archetypes share the pattern. **Only ST23-04 and
EX4-074 are confirmed here.** Additional candidates: AD1-015 and P-153 (Light
Hybrid), BT25-041 (Glowing Dawn), EX11-012 (Medusamon), EX12-046 (Toho Braves),
EX13-057 (Chronicle). The relevant source and YAML match the deployed image.

A repair must distinguish carrier-scoped printed End of Attack from legitimate
broad observers and scheduled processing. Restricting the entire global event
to the attacker would be unsafe. The tests establish a material confound, not
the fraction of any deck's win-rate discrepancy caused by it.

## List selection is a separate issue

The selector sorts eligible lists by date, then top-cut flag, takes the first
two for training and the third for evaluation (`english_meta_search_job.py`).
It is not a competitive-performance-stratified selection. All 24 tournament
snapshot records match their stored source card counts, so this is a selection
issue rather than corrupted list ingestion.

All frozen lists pass the current light admission gate. The 280 cards all have
nonempty card-local test files, but 61 unique cards have no ledger entry. Absence
is accepted by the admission policy. File presence and admission do not establish
clause-level correctness or DCGO confirmation. Most notably, 19/20 SW cards and
18/19 Virus Busters cards have no ledger entry. These are evidence gaps, not
claims of 37 broken cards.

Held-out shifts from the closest training list are 17/54 copies for TS Mervamon,
13/54 Chronomon, 8/54 Examon, 6/54 Jupitermon and Three Musketeers, 5/54 Chronicle
and Glowing Dawn, and 3/54 Toho. Such changes are legitimate generalization tests,
but one list cannot represent an entire archetype's strength.

The audit found unused, locally admitted successful EX13 candidate lists for all
eight meta archetypes; exact counts/URLs are saved in `list-audit.json`. These were
not substituted into the live pool. Two additional recent Three Musketeers
sources were verified: Llama, 1st/42 on October 24
(https://digimon.digilab.cards/decklist/126919), and Onion44, 1st/31 on October 19
(https://digimon.digilab.cards/decklist/125077). Their exact engine admission remains
separate from the older locally admitted candidate set.

Correction to the initial Three Musketeers hypothesis: Llama's winning list also
omits Cherubimon and Chronomon: Holy Mode. Their absence cannot by itself diagnose
our evaluation list as weak. Both new candidate lists contain Junomon: Hysteric
Mode, but that observation alone is not causal evidence for a substitution.

## Population-weight and behavior findings

Reweighting the original 156 directed matchup cells by the October 26 recorded
tournament-entry counts within the same 13-deck pool (12 opponents per deck,
excluding mirrors and renormalizing, with no outside-pool replacement) does
not remove the main anomalies: Three Musketeers 38.54 -> 37.12%, Examon 46.88 ->
45.26%, Chronicle 65.00 -> 64.93%, SW 60.21 -> 59.97%. No new games were generated
for this calculation. Outside-pool opponents, global/English differences and
small 40-game cells remain limitations.

The new baseline wins 2,528/2,600 games against greedy (97.23%). That anchor is
largely saturated; it cannot certify modern-deck proficiency. Against the same
model, raising evolution was selected on 9,856/13,004 legal decision exposures
(75.79%). Main battle-evolution choices were 3,609/11,830; actual effect-driven
entries are tracked separately and must not be inferred from that ratio.

The policy promoted on **4,159/4,159** observed legal promotion decisions across
the 13 decks. This is a behavioral flag for delayed-development tactical tests,
not proof every promotion was wrong. Native masks remain authoritative.

Three Musketeers entered BT25-085 in 94/200 same-model games and EX7-073 in 64/200.
These are descriptive entry rates, not conditional failure rates: hand presence
does not establish a legal profitable evolution. Full free-Option/Arts/payment
chains need dedicated instrumentation and tactical outcomes.

Virus Busters had only three legal EX12-001 sequence opportunities in these 200
same-model games; all three completed DNA and the subsequent attack. The larger
corrected checkpoint comparison had 11 successes out of 14 opportunities in
520 checkpoint-4 appearances. These separate schedules are not pooled. The
remaining question is setup/eligibility coverage, not a blanket refusal to DNA.

## Next measurement order

1. Repair and requalify printed End of Attack carrier semantics, retaining broad
   observers, clone/identity behavior, and own-attack positives. Rerun affected
   fixed schedules; retain the current diagnostic data separately.
2. Add a separate multi-list evaluation panel using independently sourced strong
   lists; preserve the original benchmark. Pet decks need distinct variants before
   claiming unseen-list generalization.
3. Repeat the completed 320/4/batch4 search-versus-policy screen after the repair,
   then expand seeds and cross-deck comparisons. Mirrors isolate own-list
   piloting; cross-deck comparisons test matchup sensitivity. Preserve seat/seed
   pairing, incomplete-game reporting and paired uncertainty; never score capped
   games as draws.
4. Add archetype tactics and sequence opportunity/completion funnels, especially
   Three Musketeers, Examon, Virus Busters and deliberate breeding-area waiting.
5. Scale training through measured learning-curve milestones. One million games
   is a possible compute scale, not an established proficiency threshold.

Meta agreement should be a sanity check, not a training objective or a reason
to alter rules until rankings look familiar. Hold lists, opponents and evaluation
seeds fixed within each comparison, and distinguish policy-only strength from
search-assisted strength.

The optimized local search-versus-policy screen completed separately under
`runs/archetype-audit-20261026/mirror-search-release/`: **104 completed games**, four
seat-paired seeds per archetype, 320 total simulations / four worlds / batch four.
It compares identical frozen weights in exact-list mirrors, with open decklists,
no exploration noise, and no cloud workload. The tracked engine, DSL, bindings
and card data match the deployed image's source revision. The known rules
defects remain, so this is diagnostic evidence only.

Search won **77/104 games (74.04%)**, with 27 losses, no draws, and no failed or
truncated pairs. All 52 planned pairs were valid. It won 39/52 games going first
and 38/52 going second. The entire screen took 481.56 seconds locally, including
worker startup; this one-sided mirror diagnostic is not a self-play training
throughput benchmark.

Treating each two-game seed pair as the sampling unit, the 95% paired-bootstrap
interval is **66.35-81.73%**. Resampling within each fixed archetype gives
**68.27-79.81%**; a conservative bounded-pair interval is **55.20-92.87%**.
These uncertainty estimates do not account for engine defects or population
mismatch. Independent verification and full methodology are recorded in
`mirror-search-release/search-analysis.json` and `search-analysis.md`.

Provenance note: the driver-hash metadata field was added six seconds after the
screen launched, so the frozen plan intentionally has no driver hash. Archived
source reconstruction and the sole one-line metadata difference are documented
in `mirror-search-release/driver-provenance.md`; no launch-time driver hash is
claimed. The executable, model, model metadata, pool and card-data hashes all
match their pre-run values. Search settings and execution logic did not change.

| Archetype | Search wins / games against identical policy-only weights |
|---|---:|
| Chronicle | 8/8 |
| Chronomon | 3/8 |
| Examon | 7/8 |
| Glowing Dawn | 5/8 |
| Light Hybrid | 6/8 |
| Medusamon | 5/8 |
| SW | 5/8 |
| ShineGreymon Data Squad | 7/8 |
| TS Jupitermon | 7/8 |
| TS Mervamon | 7/8 |
| Three Musketeers | 4/8 |
| Toho Braves | 7/8 |
| Virus Busters | 6/8 |

These are within-deck mirror outcomes, not a cross-deck ranking. Four seeds per
deck do not establish reliable per-archetype effects. Three Musketeers split
every paired seed; Chronomon had three splits and one pair of search losses.
They are follow-up priorities, not evidence that search is ineffective or
harmful for those archetypes. The broad aggregate supports a benefit from
lookahead under this engine, but does not identify the cause of the real-meta
ranking gaps or certify correct card behavior.

The earlier development-build attempt was stopped without completed pairs
because it was too slow for the planned screen; no strength claim uses it.
The optimized executable built successfully. Twelve Python audit-tool tests
pass, including frozen provenance checks, incomplete-pair exclusion and explicit
draw denominators. The three failing native regressions remain visible; this
audit does not claim a green engine suite.
