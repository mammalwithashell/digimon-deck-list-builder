---
item: clause:BT26-072#inherited#0
run_id: pilot-data-squad
escalated_at: 2026-10-06T23:28:51.921434Z
state_before: AUTHORING
reason: attempt cap: author_clause 3/3 spent (state AUTHORING)
---

# Escalated: `clause:BT26-072#inherited#0`

**Why:** attempt cap: author_clause 3/3 spent (state AUTHORING)

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T232054368991Z-pilot-data-squad-a7394a` | author_clause | claude | sonnet | accepted | 0.9953 |
| 2 | `20261006T232402850692Z-pilot-data-squad-18516d` | author_clause | claude | sonnet | accepted | 0.5243 |
| 3 | `20261006T232656220501Z-pilot-data-squad-6f6494` | author_clause | claude | sonnet | accepted | 0.4954 |

## Item data

```json
{
  "author_attempt": "20261006T232656220501Z-pilot-data-squad-6f6494",
  "author_family": "claude",
  "author_stage": "author_clause",
  "card_ids": [
    "BT26-072"
  ],
  "covers": [
    "BT26-072#inherited#0"
  ],
  "deck_books": null,
  "history": [
    "20261006T232054368991Z-pilot-data-squad-a7394a",
    "20261006T232402850692Z-pilot-data-squad-18516d",
    "20261006T232656220501Z-pilot-data-squad-6f6494"
  ],
  "merge": {
    "attempt_id": "20261006T232656220501Z-pilot-data-squad-6f6494",
    "branch": "card-loop/pilot-data-squad/run",
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
            "qa/dcgo-exams/BT26/BT26-072-inherited0.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT26/BT26-072-inherited0.yaml",
          "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "681acee8b4b371c8a6552019dac6c58a41e576a5",
    "touched": [
      "qa/dcgo-exams/BT26/BT26-072-inherited0.yaml"
    ]
  },
  "oracle_results": null,
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "ruling_block_written": null,
  "scenario_paths": [
    "qa/dcgo-exams/BT26/BT26-072-inherited0.yaml"
  ],
  "sim_failure": {
    "qa/dcgo-exams/BT26/BT26-072-inherited0.yaml": [
      "FAILED: step 1: our engine asks a selection here; the scenario must answer it with a `select:` step (pending kind: Replacement, prompt: 'You may activate ST24-12's triggered effect')"
    ]
  }
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
