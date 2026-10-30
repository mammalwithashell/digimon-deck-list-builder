# Cloud EX13 pool adoption

The prepared EX13 cloud pool was already available on main. It should have been inspected before limiting the first local comparison to constructed ST1 variants. The old ST1 experiment and its failed promotion result remain unchanged.

## Source and current admission

- Fetched `origin/main` on 2026-10-29 UTC: `d0d052026`, already contained in this branch. No missing remote-main commit required merging.
- The runbook points to `data/training_pools/ex13-20261026/`. Read-only SSH inspection confirmed the latest recorded history campaign finished checkpoint 12. Its native deck sampler and training, held-out, pet and combined-diagnostic snapshot bytes match the committed source files exactly. No cloud service, run, checkpoint or source input was changed.
- Current Python shared snapshot admission and standard deck legality accept **21/21 training lists across 13 archetypes**, **23/23 source-held-out lists across 12 archetypes**, **36/36 supplemental validation lists across 10 archetypes**, and **5/5 original pet regressions**.
- The native CLI also admits every partition. These were explicitly zero-game gate checks, not played-game or model-strength evidence. The global mechanics gate is open; no gate or card count was relaxed.
- The original native schedule has **26 weighted slots for 21 distinct training lists**. Those slots preserve archetype balance. The prior uses one equal weight per distinct training list, not duplicate mass from repeated sampling slots.
- Existing source partitions are preserved. No held-out list was moved into training or candidate priors. Light Hybrid has no unseen-list test here; its pet regression remains separately labelled. Supplemental source evidence remains weaker/mixed and is used only as validation input.

## Prepared fair-belief inputs

`code/tools/prepare_cloud_belief_pool.py` now imports these frozen snapshots through the existing shared admission and legality APIs, checks cross-partition content leakage, preserves native training weights, prepares the 21-candidate closed prior and independently checks the native admission surface. It refuses to overwrite a frozen output.

```powershell
python code/tools/prepare_cloud_belief_pool.py --source data/training_pools/ex13-20261026 --out <new-directory> --cli <current-native-cli.exe>
```

The prepared local bundle is `D:/cargo-target-wt/closed-deck-belief-training/cloud-ex13-20261029`. It contains `train.json`, `validation.json`, `test.json`, `pet_regression.json`, `prior.json`, immutable pre-prior provenance and a completed native admission report. The prior contains training lists only and uses the existing globally admitted fallback population. The five pet lists stay separate even where their contents also appear in training.

## Initial closed-search smoke results

Admission succeeded, but the initial closed-world builder was not qualified for this full pool. One mirror-game probe was run for each of the 21 distinct training lists: uniform evaluator, four simulations, two worlds/batch two, `standard_lite_v2`, decision cap 1200, and seeds `20261029 + index` in the source `distinct_training_decks.json` order. These are interface/continuation probes, not policy evaluations or training runs.

**1/21 completed:** the Medusamon list. **20/21 stopped explicitly** at the first unsupported closed-world state. Failed probes were preserved and were not counted as completed games, draws or card-admission failures. Later continuation coverage in each stopped game remains unmeasured.

| First stopping condition | Probes |
|---|---:|
| `closed-list continuation requires audit: DigivolveCostChoice` | 7 |
| `closed-list continuation requires audit: RevealBucketStep` | 8 |
| `closed-list continuation requires audit: AttackDeclarationDecision` | 1 |
| `closed-list continuation requires audit: ZoneCardPickStep` | 1 |
| `public zone contains a hidden source; knowledge integration required` | 1 |
| `closed-list continuation requires audit: OptionalReplacement` | 1 |
| `closed-list continuation requires audit: RunTail` | 1 |

The next engineering work is to audit/rebind these continuation forms and represent concealed field-source knowledge, with composition-changing paired-game tests. Clearing the guards without those checks could reintroduce hidden-truth dependence. This is a limitation of the new closed-belief builder, not evidence that the existing cloud pool lacks admissible lists.

No broad model-comparison result or promotion follows from these probes. A new broader training protocol should use the runbook operating point as evidence: 1,100-game generations for this 13-archetype pool, 320 total simulations/four worlds, existing trained-model contracts (`standard_lite_v2`) and independently frozen evaluation panels. The previous 32-game, 32-simulation ST1 study cannot stand in for that qualification.

## Validation and receipts

- Five focused tests cover retained pet/train overlap, cross-partition content leakage independent of card ordering, and duplicate-list rejection.
- Shared snapshot loading verifies snapshot hashes, deck IDs, mechanics readiness, card admission and compiled/playable IDs; the existing engine validator checks legality. Native preparation/gate checks passed for all partitions.
- Committed receipts in `evidence/cloud-pool/` preserve exact source/cloud hashes, per-partition admission counts, native/binding identities, prior digest and all 21 smoke outcomes. Raw probe decks/logs/failure files remain under the local bundle.

## Follow-up qualification

The [2026-10-29 software qualification](cloud-search-qualification.md) supersedes this initial readiness snapshot: 127 probes now complete without failure or truncation across three training-list seeds and every held-out, validation and pet entry. The original 1/21 evidence remains intact. Broader model comparison and promotion evidence remain separate and pending.
