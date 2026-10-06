---
item: interaction:probe:EX7-073#effect#2:scope:neg
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T14:39:08.904649Z
state_before: ORACLE
reason: attempt cap: oracle_retry 3/3 spent (state ORACLE)
---

# Escalated: `interaction:probe:EX7-073#effect#2:scope:neg`

**Why:** attempt cap: oracle_retry 3/3 spent (state ORACLE)

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T143144830894Z-pilot-three-musketeers-e77b66` | author_interaction | claude | sonnet | accepted | 0.7118 |

## Item data

```json
{
  "author_attempt": "20261006T143144830894Z-pilot-three-musketeers-e77b66",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/EX7/EX7-073-effect2.yaml",
  "card_ids": [
    "EX7-073"
  ],
  "covers": [
    "EX7-073#effect#2"
  ],
  "deck_books": {
    "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml": "qa/dcgo-exams/EX7/three_musketeers_pool.json"
  },
  "history": [
    "20261006T143144830894Z-pilot-three-musketeers-e77b66"
  ],
  "merge": {
    "attempt_id": "20261006T143144830894Z-pilot-three-musketeers-e77b66",
    "branch": "card-loop/pilot-three-musketeers/run",
    "engine": false,
    "ok": true,
    "scope": {
      "cards": [],
      "cards_behavioral_filter": "",
      "full_suite_required": false,
      "reasons": [],
      "side_binaries": [],
      "suite": {
        "commands": [],
        "green": true,
        "impact_scope": {
          "argv": [
            "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe",
            "code/tools/impact_scope.py",
            "--json",
            "--path",
            "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml",
          "cwd": "D:\\cl-tm\\cl-pilot-three-musketeers-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "937c04138dd9c6bca793946bc97165ce3a9a029d",
    "touched": [
      "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml"
    ]
  },
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "EX7-073#effect#2",
    "denominator": null,
    "divergence": null,
    "first_divergence": null,
    "ids": [
      "probe:EX7-073#effect#2:scope:neg"
    ],
    "job_id": "exam-EX7-073-effect2-scope-neg",
    "job_outcome": null,
    "mismatch": null,
    "reason": "an earlier exam-EX7-073-effect2-scope-neg is still running on a player (C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness\\claimed\\exam-EX7-073-effect2-scope-neg.json exists): submitting now would hand back THAT run's result. Wait for it to finish, or clear an orphaned claim with `dcgo-harness --root <root> status --sweep`.",
    "recorded": [],
    "refused": [],
    "scenario": "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml",
    "sidecar": null,
    "verdict": "unmeasured"
  },
  "oracle_results": {
    "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "EX7-073#effect#2",
      "denominator": null,
      "divergence": null,
      "first_divergence": null,
      "ids": [
        "probe:EX7-073#effect#2:scope:neg"
      ],
      "job_id": "exam-EX7-073-effect2-scope-neg",
      "job_outcome": null,
      "mismatch": null,
      "reason": "an earlier exam-EX7-073-effect2-scope-neg is still running on a player (C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness\\claimed\\exam-EX7-073-effect2-scope-neg.json exists): submitting now would hand back THAT run's result. Wait for it to finish, or clear an orphaned claim with `dcgo-harness --root <root> status --sweep`.",
      "recorded": [],
      "refused": [],
      "scenario": "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml",
      "sidecar": null,
      "verdict": "unmeasured"
    }
  },
  "oracle_retry_paths": [
    "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml"
  ],
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "scenario_paths": [
    "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml"
  ],
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/EX7/EX7-073-effect2-scope-neg.yaml": [
      "step 8 expect.prompt 'OptionalSkill' not asserted sim-side (kind UnionZone { zones: UnionZoneSet(3) } has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "step 10 expect.prompt 'SelectCountEffect' not asserted sim-side (kind EffectChoice has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "step 19 answers our TriggerOrder prompt by identity -- 'EX7-073' ordinal 0 is branch 0 of [EX7-073, EX7-073]. That order is OURS; DCGO resolves the same step against its own list, so a disagreement surfaces as a divergence. Prefer `trigger:` where the branch is a keyword, or `trigger_not:` where it is the one that is not.",
      "step 22 expect.prompt 'SelectCardEffect' not asserted sim-side (kind SourceMulti { min: 0, max: 2, picked: 0 } has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "source": "probe"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
