# Virus Busters DNA data investigation, 2026-10-25

**Confirmed and fixed locally: EX12-032 WereGarurumon accepted an illegal
level-5 DNA material pair. Its legal level-4 route was already available.**
This corrects the initial suspicion that the wrong data blocked its ordinary
DNA development. No live training image, process, reward or checkpoint changed.

## Source and runtime trace

The inspected card face prints blue/yellow level 4 + purple/red level 4, cost 0.
The local DCGO `EX12/Blue/EX12_032.cs` implementation independently checks
`Levels_ForJogress(card).Contains(4)` for both materials. The engine YAML also
specifies level 4. This is source inspection, not a live DCGO oracle run.

The stale `data/cards.json` record instead had level 5 in both the `xros_req`
DNA text and parsed `dna_costs`. The parser reproduced the bad source text;
this was not demonstrated to be a parsing error. No EX12-032 ingest override
existed to protect the corrected recipe.

Runtime construction explains the surprising reproduction:

1. `Game::new` obtains the shared card store from production card data.
2. `card_store::build_shared_card_store` invokes DSL alt-path enrichment.
3. `dsl_bridge::enrich_card_data_with_dsl_alt_paths` appends distinct YAML DNA
   recipes without deleting an existing differing recipe.
4. EX12-032 therefore had both the bad level-5 recipe and the correct level-4
   recipe. `dna_digivolve::get_dna_stacking_order_ext` accepts either.

The older `ex12_032_dna_materials_are_printed_level4_pairs` test creates metadata
from YAML alone, so it cannot detect the extra production-data recipe. The new
test registers real DSL effects, then loads actual production card metadata,
allowing the normal shared-store enrichment to run.

Relevant bridge, matcher and EX12-032 YAML sources are unchanged between the
live image's engine revision `e1e7fff2c689e8505a8146d9012502be11dce8c1` and the local
pre-fix branch. Local production-data behavior is therefore directly relevant,
but this study did not deploy or rerun the cloud games on a patched image.

## Before and after

| Check | Before | After |
|---|---|---|
| All four printed color combinations, both stack orders, level 4 | Legal | Legal |
| Blue level 5 + purple level 5 | **Incorrectly legal** | Rejected |
| Mixed levels or wrong color pairing | Not reached after earlier failure | Rejected |
| Level-4 DNA selection, zero memory cost, draw, clone continuation | Resolves | Resolves |
| Other three Virus Busters DNA targets: printed levels accepted, higher pair rejected | Pass | Pass |

The failing Rust regression was captured before changing data: three tests
passed, while the negative level-5 case failed with mask value 1 instead of 0.
After correction all four tests pass. The material selections remain visible
through normal actions; the clone resolves to the same verification digest.

The three other targets checked are EX12-035 MetalGarurumon (level 5 pair),
EX12-037 Omnimon (level 6 pair), and EX12-044 Angewomon (level 4 pair). Their
printed material-level requirements, production data and YAML agree. This is a
focused DNA-recipe check, not a full fidelity certification of these cards.

## Potential impact on existing training data

The generation-2 Virus Busters cohort had 233 normal Main-phase DNA opportunity
positions. Decoding each legal DNA action's hand slot using exported observation
layout metadata gives:

| DNA target | Positions offering target | Search ranks target first |
|---|---:|---:|
| EX12-032 WereGarurumon | 153 | 11 |
| EX12-035 MetalGarurumon | 54 | 2 |
| EX12-037 Omnimon | 10 | 2 |
| EX12-044 Angewomon | 62 | 2 |

Target opportunities overlap when several different targets are legal in one
position, so the first numeric column must not be summed as unique positions.
Copies of one target in hand are deduplicated per position. The preferences sum
to the earlier 17 Virus Busters DNA-positive positions. Generation 1 offered
EX12-032 in 105 positions, with zero teacher-argmax preferences.

These counts do not identify the chosen materials, actual sampled action or
resolved evolution. The 153 WereGarurumon positions are **potentially affected**,
not 153 proven illegal opportunities. Likewise the eleven preferred moves are
not eleven proven illegal evolutions. Establishing that requires full-state
replay and material-recipe comparison. Search/value rollouts could also have
explored the illegal recipe without choosing it at the real-game root.

Consequently, this bug does not explain low evolution usage by suppressing the
legal level-4 route. It can contaminate the legal action set and search targets.
Do not treat missing imitation of these eleven EX12-032 preferences as proof
that the model rejected a correct move. The broader battle-evolution audit is
not reclassified by this specific DNA-data finding.

## Repair and validation

- Corrected EX12-032 `xros_req` and derived `dna_costs` in `data/cards.json`.
- Added a persistent `card_overrides.json` source-text correction; ingest already
  re-derives the parsed DNA fields after such overrides.
- Regenerated only `code/digimon-engine/cards/ex12/EX12-032.json`.
- Added production-data Rust regression and an ingest regression that starts
  with bad level-5 upstream text, applies the actual override, and checks central
  and split data consistency. No generic engine rule or YAML effect changed.

Validation: four Rust regressions passed; fourteen Python ingest/parser tests
passed; the split JSON consistency check and all thirteen tier-0 suites passed.
The target attribution audit
ran successfully in the frozen image with a read-only experiment mount,
network disabled and 0.5 CPU / low CPU shares. It verified source chunk hashes.

Reproduce the focused checks:

```text
cargo test -p digimon-engine --test virus_busters_dna_data -- --test-threads=1
python -m pytest code/tests/tools/test_virus_busters_dna_data.py code/tests/tools/test_xros_cost_parser_assembly.py -q
python code/tools/split_cards_json.py --card EX12-032 --check
```

The attribution tool is `code/tools/cloud_search/audit_dna_targets.py`; it takes
the prior raw DNA-cohort audit, frozen `cards.json` and generation root. Raw
output remains at `/opt/digimon-search/dna-cohort-audit-20261025/targets-result.json`.
The [committed evidence](2026-10-25-virus-busters-dna-data.json) records its counts
and the before/after test evidence.

Deployment remains pending. A future training image must include the corrected
data; editing this checkout does not repair the currently running image or
retroactively repair its shards. Preserve frozen-run provenance and evaluate
with the corrected engine before using this cohort for a promotion decision.
